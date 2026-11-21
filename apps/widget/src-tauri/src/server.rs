use crate::logging;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;

static SERVER_PROCESS: Mutex<Option<CommandChild>> = Mutex::new(None);
static SERVER_PORT: Mutex<Option<u16>> = Mutex::new(None);
static SERVER_INTENTIONAL_STOP: AtomicBool = AtomicBool::new(false);
static SERVER_CRASH_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

const MAX_CRASH_RESTARTS: u32 = 3;

/// A person unloaded the model: it stays unloaded (no lazy load on the next
/// dictation) until they choose Load now.
static UNLOADED: AtomicBool = AtomicBool::new(false);

/// No embedded server running, but this build has one to start.
static SERVER_STOPPED: AtomicBool = AtomicBool::new(false);

const DEFAULT_PORT: u16 = 8002;
const PORT_RANGE_END: u16 = 8020;
const POLL_INTERVAL: Duration = Duration::from_millis(200);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);

/// Find an available port starting from DEFAULT_PORT.
/// Retries DEFAULT_PORT once after a short delay to handle TIME_WAIT.
fn find_available_port() -> Option<u16> {
    // Try DEFAULT_PORT first
    if TcpListener::bind(("127.0.0.1", DEFAULT_PORT)).is_ok() {
        return Some(DEFAULT_PORT);
    }
    // Could be TIME_WAIT from a just-stopped server — wait briefly and retry
    std::thread::sleep(Duration::from_millis(600));
    if TcpListener::bind(("127.0.0.1", DEFAULT_PORT)).is_ok() {
        return Some(DEFAULT_PORT);
    }
    // Still stuck, fall back to next ports
    ((DEFAULT_PORT + 1)..=PORT_RANGE_END)
        .find(|&port| TcpListener::bind(("127.0.0.1", port)).is_ok())
}

pub struct ServerInfo {
    pub port: u16,
    pub model: String,
}

/// Check whether the sidecar binary actually exists on disk.
pub fn has_embedded_server(app: &AppHandle) -> bool {
    app.shell().sidecar("whisper-server").is_ok()
}

/// Spawn the embedded whisper server as a child process.
/// Returns server info (port + actual model used), or None on failure.
pub fn start_server(app: &AppHandle, model: &str) -> Option<ServerInfo> {
    if !should_auto_restart() {
        logging::log_message("[server] Too many consecutive crashes — not starting server");
        let _ = app.emit(
            "server:error",
            serde_json::json!({ "reason": "Server crashed repeatedly. Check Settings → Model." }),
        );
        return None;
    }

    let model = match crate::downloader::resolve_model_for_server(model) {
        Some(model) => model,
        None => {
            let data_dir = crate::paths::config_dir_opt()?;
            let model_path = data_dir.join("models").join(format!("ggml-{}.bin", model));
            let reason = if model_path.exists() {
                "incomplete"
            } else {
                "missing"
            };
            logging::log_message(&format!(
                "[server] No complete model available for '{}', not starting server",
                model
            ));
            let _ = app.emit(
                "server:model_needed",
                serde_json::json!({
                    "model": model,
                    "path": model_path.to_string_lossy(),
                    "reason": reason,
                }),
            );
            return None;
        }
    };

    // Resolve model path
    let data_dir = crate::paths::config_dir_opt()?;
    let model_path = data_dir.join("models").join(format!("ggml-{}.bin", model));

    reap_orphan();

    let port = match find_available_port() {
        Some(p) => p,
        None => {
            logging::log_message(&format!(
                "[server] No available port in range {}-{}",
                DEFAULT_PORT, PORT_RANGE_END
            ));
            return None;
        }
    };

    if port != DEFAULT_PORT {
        logging::log_message(&format!(
            "[server] Port {} in use, using port {}",
            DEFAULT_PORT, port
        ));
    }

    logging::log_message(&format!(
        "[server] Starting sidecar whisper-server --model {} --port {}",
        model_path.display(),
        port,
    ));

    let mut args = vec![
        "-m".to_string(),
        model_path.to_string_lossy().to_string(),
        "--port".to_string(),
        port.to_string(),
    ];

    // Cap decode threads so inference never saturates every core and makes the
    // UI (widget, settings tabs, paste) unresponsive mid-transcription.
    // Leave ~2 cores free; clamp to a sane band across machines.
    {
        let ncpu = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(6);
        let threads = ncpu.saturating_sub(2).clamp(4, 8) as u32;
        logging::log_message(&format!("[server] Decode threads capped at {}", threads));
        args.push("-t".to_string());
        args.push(threads.to_string());
    }

    if let Some(vad_path) = crate::downloader::ensure_vad_model_blocking() {
        logging::log_message(&format!(
            "[server] Enabling Silero VAD ({})",
            vad_path.display()
        ));
        for arg in [
            "--vad",
            "--vad-model",
            &vad_path.to_string_lossy(),
            "--vad-threshold",
            "0.45",
            "--suppress-nst",
        ] {
            args.push(arg.to_string());
        }
    } else {
        logging::log_message("[server] Silero VAD unavailable — starting without VAD trim");
    }

    // The bundled binary sits in `<Contents/MacOS>/`, the app's own folder and
    // not on PATH, so the watcher takes its directory and runs it from there.
    let server_bin = match std::env::current_exe() {
        Ok(exe) => sidecar_beside(&exe),
        Err(e) => {
            logging::log_message(&format!("[server] Can't locate the app executable: {}", e));
            return None;
        }
    };
    let (program, argv) = watch_command(&server_bin, std::process::id(), &args);
    let mut watch_cmd = app.shell().command(&program);
    for arg in &argv {
        watch_cmd = watch_cmd.arg(arg);
    }

    match watch_cmd.spawn() {
        Ok((mut rx, child)) => {
            logging::log_message(&format!(
                "[server] Spawned server (pid={}, port={}, model={})",
                child.pid(),
                port,
                model
            ));
            let pid = child.pid();
            if let Some(start_time) = process_spawned(pid) {
                write_pidfile(&PidRecord { pid, start_time });
            }

            // Log sidecar output and monitor for unexpected termination
            let app_handle_for_monitor = app.clone();
            tauri::async_runtime::spawn(async move {
                use tauri_plugin_shell::process::CommandEvent;
                use tokio::io::AsyncWriteExt;

                let log_dir = crate::paths::config_dir_opt();
                let mut log_file = if let Some(dir) = log_dir {
                    tokio::fs::create_dir_all(&dir).await.ok();
                    tokio::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(dir.join("server.log"))
                        .await
                        .ok()
                } else {
                    None
                };

                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                            if let Some(f) = log_file.as_mut() {
                                let _ = f.write_all(&line).await;
                                let _ = f.write_all(b"\n").await;
                            }
                        }
                        CommandEvent::Terminated(payload) => {
                            if let Some(f) = log_file.as_mut() {
                                let _ = f.write_all(b"[server] Process terminated\n").await;
                            }
                            if !SERVER_INTENTIONAL_STOP.load(Ordering::SeqCst) {
                                let crashes = SERVER_CRASH_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
                                logging::log_message(&format!(
                                    "[server] Server terminated unexpectedly (code={:?}, crashes={})",
                                    payload.code, crashes
                                ));
                                let _ = app_handle_for_monitor.emit(
                                    "server:crash",
                                    serde_json::json!({
                                        "reason": "Process terminated unexpectedly",
                                        "code": payload.code,
                                        "crashes": crashes,
                                    }),
                                );
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            });

            if let Ok(mut proc) = SERVER_PROCESS.lock() {
                *proc = Some(child);
            }
            if let Ok(mut port_lock) = SERVER_PORT.lock() {
                *port_lock = Some(port);
            }
            SERVER_INTENTIONAL_STOP.store(false, Ordering::SeqCst);
            SERVER_STOPPED.store(false, Ordering::SeqCst);
            Some(ServerInfo {
                port,
                model: model.to_string(),
            })
        }
        Err(e) => {
            logging::log_message(&format!("[server] Failed to start sidecar: {}", e));
            None
        }
    }
}

/// Poll the health endpoint until the server is ready.
/// Returns `true` if the server became ready within the timeout.
pub async fn wait_for_server(port: u16) -> bool {
    let health_url = format!("http://127.0.0.1:{}/health", port);
    let start = Instant::now();

    logging::log_message(&format!("[server] Waiting for server on port {}...", port));

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            logging::log_message(&format!("[server] Failed to create HTTP client: {}", e));
            return false;
        }
    };

    loop {
        if start.elapsed() > STARTUP_TIMEOUT {
            logging::log_message("[server] Timed out waiting for server");
            return false;
        }

        if let Ok(resp) = client.get(&health_url).send().await {
            if resp.status().is_success() {
                SERVER_CRASH_COUNT.store(0, Ordering::SeqCst);
                logging::log_message(&format!(
                    "[server] Server ready in {:.1}s on port {}",
                    start.elapsed().as_secs_f64(),
                    port,
                ));
                return true;
            }
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// The port the server is currently running on, if any.
pub fn current_port() -> Option<u16> {
    SERVER_PORT.lock().ok().and_then(|p| *p)
}

pub fn is_running() -> bool {
    current_port().is_some()
}

/// Start the embedded sidecar if it is not already running.
/// Returns `None` when this build has no embedded server (external API mode).
pub async fn ensure_running(app: &AppHandle, model: &str) -> Result<Option<ServerInfo>, String> {
    if !has_embedded_server(app) {
        return Ok(None);
    }

    if is_running() {
        let port =
            current_port().ok_or_else(|| "Server is running but port is unknown".to_string())?;
        return Ok(Some(ServerInfo {
            port,
            model: model.to_string(),
        }));
    }
    if UNLOADED.load(Ordering::SeqCst) {
        return Ok(None);
    }

    if !crate::downloader::is_model_complete(model) {
        return Err(format!("Model '{}' is not fully downloaded", model));
    }

    let _ = app.emit(
        "server:starting",
        serde_json::json!({ "reason": "load_on_demand" }),
    );

    let Some(info) = start_server(app, model) else {
        let _ = app.emit(
            "server:error",
            serde_json::json!({ "reason": "Failed to start server — check that the model is fully downloaded." }),
        );
        return Err("Failed to start whisper server".to_string());
    };

    if !wait_for_server(info.port).await {
        stop_server();
        let _ = app.emit(
            "server:error",
            serde_json::json!({ "reason": "Server failed to start within timeout" }),
        );
        return Err("Server failed to start within timeout".to_string());
    }

    let _ = app.emit(
        "server:ready",
        serde_json::json!({ "port": info.port, "model": info.model }),
    );

    Ok(Some(info))
}

pub fn should_auto_restart() -> bool {
    SERVER_CRASH_COUNT.load(Ordering::SeqCst) < MAX_CRASH_RESTARTS
}

/// Settings → Advanced and the menu bar call off a start that is still waiting.
pub fn is_stopped() -> bool {
    SERVER_STOPPED.load(Ordering::SeqCst)
}

/// A person unloaded the model: stop the server and keep it stopped, so the
/// next dictation does not silently load it again.
pub fn unload() {
    UNLOADED.store(true, Ordering::SeqCst);
    stop_server();
}

/// The next start serves dictation again (Load now, a restart).
pub fn mark_loaded() {
    UNLOADED.store(false, Ordering::SeqCst);
}

/// A server left behind by a crash or force-quit: pid plus its start time, so a
/// reused pid is never mistaken for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PidRecord {
    pid: u32,
    start_time: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct Process {
    exe: String,
    start_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Reap {
    /// No record, a pid that was reused, or the process is gone.
    Forget,
    Kill,
}

/// Tauri puts sidecars next to the app executable (`Contents/MacOS/`), not in
/// `Contents/Resources/`.
fn sidecar_beside(exe: &std::path::Path) -> std::path::PathBuf {
    exe.with_file_name("whisper-server")
}

/// `sh -c '<watcher>' <server> <app pid> <server argv…>`. Validated as a shell
/// script before it was written here; the tests run the same text with a fake
/// server and a fake app so the loop is proven, not just composed.
fn watch_command(server: &std::path::Path, owner: u32, args: &[String]) -> (String, Vec<String>) {
    let mut argv = vec![
        "-c".to_string(),
        watcher_script(),
        server.to_string_lossy().into_owned(),
        owner.to_string(),
    ];
    argv.extend(args.iter().cloned());
    ("/bin/sh".to_string(), argv)
}

/// The watcher, as `sh`: `$0` is the server binary (found through the bundle's
/// `MacOS` directory, which is not on PATH), `$1` the app's pid, then the
/// server's own arguments.
///
/// The server gets a closed-off fd 9 (the plugin's pipe) and `/dev/null` stdin,
/// so it never holds a handle to the app. A subshell closes `sh`'s own copy of
/// fd 9 after two seconds, so a crash or force-quit surfaces as a closed pipe
/// at once. The loop then kills the server within a second instead of leaving
/// it for an hour-long transcription to finish.
fn watcher_script() -> String {
    r#"export PATH="$(dirname "$0"):$PATH"
running() { pid=$(ps -o pid= -p "$1" 2>/dev/null) || return 1
  [ -n "$pid" ] && [ "$(ps -o state= -p "$1" 2>/dev/null | tr -d ' ')" != Z ]; }
owner=$1; shift
"$0" "$@" </dev/null 9>&- &
srv=$!
trap 'kill -TERM "$srv" 2>/dev/null; exit 0' TERM HUP
if [ -e /dev/fd/9 ] || { : >&9; } 2>/dev/null; then ( sleep 2; exec 9>&- ) & fi
while kill -0 "$srv" 2>/dev/null
do
  if ! running "$owner"; then kill -KILL "$srv"; wait "$srv" 2>/dev/null; exit 0; fi
  sleep 1 &
  wait $!
done
wait "$srv""#
        .to_string()
}

fn write_pidfile_at(path: &std::path::Path, record: &PidRecord) {
    let written = std::fs::write(path, format!("{} {}\n", record.pid, record.start_time));
    if let Err(e) = written {
        logging::log_message(&format!(
            "[server] Could not write {}: {}",
            path.display(),
            e
        ));
    }
}

fn read_pidfile_at(path: &std::path::Path) -> Option<PidRecord> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut fields = text.split_whitespace();
    Some(PidRecord {
        pid: fields.next()?.parse().ok()?,
        start_time: fields.next()?.parse().ok()?,
    })
}

fn remove_pidfile_at(path: &std::path::Path) {
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            logging::log_message(&format!(
                "[server] Could not remove {}: {}",
                path.display(),
                e
            ));
        }
    }
}

fn write_pidfile(record: &PidRecord) {
    let path = crate::paths::server_pid_path();
    if let Some(dir) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            logging::log_message(&format!(
                "[server] Could not create {}: {}",
                dir.display(),
                e
            ));
            return;
        }
    }
    write_pidfile_at(&path, record);
}

fn remove_pidfile() {
    remove_pidfile_at(&crate::paths::server_pid_path());
}

/// Whether `exe` is our server. The app bundle execs its sidecar through the
/// `.app/Contents/MacOS/` directory that contains the bundled binary, so the
/// path ends in `MacOS/whisper-server` there and in `binaries/whisper-server`
/// in a raw build.
fn is_whisper_server(exe: &str) -> bool {
    let name = std::path::Path::new(exe).file_name();
    name.is_some_and(|name| {
        name == "whisper-server" && exe.ends_with(&format!("MacOS/{}", name.to_string_lossy()))
    })
}

fn reap_decision(recorded_pid: Option<u32>, process: Option<Process>, record: &PidRecord) -> Reap {
    if recorded_pid != Some(record.pid) || process.is_none() {
        return Reap::Forget;
    }
    let process = process.expect("checked above");
    if !is_whisper_server(&process.exe) || process.start_time > record.start_time {
        return Reap::Forget;
    }
    Reap::Kill
}

/// Kill the server a crashed launch left behind, before a new one takes a port.
fn reap_orphan() {
    let path = crate::paths::server_pid_path();
    let Some(record) = read_pidfile_at(&path) else {
        return;
    };
    let decision = reap_decision(Some(record.pid), running_process(record.pid), &record);
    if decision == Reap::Forget {
        remove_pidfile_at(&path);
        logging::log_message(&format!(
            "[server] Ignoring stale pidfile for pid {}: that pid now belongs to another process",
            record.pid
        ));
        return;
    }
    logging::log_message(&format!(
        "[server] Reaping whisper-server (pid {}) left behind by an earlier launch",
        record.pid
    ));
    unsafe { libc::kill(record.pid as i32, libc::SIGTERM) };
    let deadline = Instant::now() + Duration::from_secs(2);
    while running_process(record.pid).is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    if running_process(record.pid).is_some() {
        unsafe { libc::kill(record.pid as i32, libc::SIGKILL) };
    }
    remove_pidfile_at(&path);
}

/// The processes `parent` started (the watcher's one child is the server).
#[cfg(target_os = "macos")]
fn child_pids(parent: u32) -> Vec<u32> {
    let mut pids = [0 as libc::pid_t; 16];
    let size = std::mem::size_of_val(&pids) as libc::c_int;
    let bytes =
        unsafe { libc::proc_listchildpids(parent as libc::pid_t, pids.as_mut_ptr().cast(), size) };
    let count = (bytes.max(0) as usize / std::mem::size_of::<libc::pid_t>()).min(pids.len());
    pids[..count]
        .iter()
        .filter(|&&pid| pid > 0)
        .map(|&pid| pid as u32)
        .collect()
}

#[cfg(not(target_os = "macos"))]
fn child_pids(_parent: u32) -> Vec<u32> {
    Vec::new()
}

/// How long the server gets to exit on SIGTERM before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(2);

/// Stop the watcher and the server it runs. SIGTERM lets the watcher's trap stop
/// the server; SIGKILL would skip the trap and leave the server running with no
/// parent, so the server is waited for and killed directly if it lingers.
fn stop_watched(watcher: u32) {
    let servers = child_pids(watcher);
    unsafe { libc::kill(watcher as i32, libc::SIGTERM) };
    let alive = |pid: u32| process_spawned(pid).is_some();
    let deadline = Instant::now() + STOP_GRACE;
    while servers.iter().any(|&pid| alive(pid)) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    for &pid in servers.iter().filter(|&&pid| alive(pid)) {
        logging::log_message(&format!(
            "[server] Server {pid} ignored SIGTERM; killing it"
        ));
        unsafe { libc::kill(pid as i32, libc::SIGKILL) };
    }
}

fn process_spawned(pid: u32) -> Option<u64> {
    if unsafe { libc::kill(pid as i32, 0) } != 0 {
        return None;
    }
    // `struct kinfo_proc` opens with `kp_proc.p_starttime` (a `timeval`), which
    // is the first and only 8-byte field of the struct on 64-bit macOS.
    let name = std::ffi::CString::new(format!("kern.proc.pid.{}", pid)).ok()?;
    let mut start_time = [0u8; 8];
    let mut len = start_time.len();
    let read = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            start_time.as_mut_ptr().cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if read != 0 || len != start_time.len() {
        return None;
    }
    Some(u64::from_ne_bytes(start_time))
}

fn running_process(pid: u32) -> Option<Process> {
    let start_time = process_spawned(pid)?;
    let mut buffer = [0u8; 4096];
    let len =
        unsafe { libc::proc_pidpath(pid as i32, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    if len <= 0 {
        return None;
    }
    Some(Process {
        exe: String::from_utf8_lossy(&buffer[..len as usize]).into_owned(),
        start_time,
    })
}

pub fn stop_server() {
    SERVER_INTENTIONAL_STOP.store(true, Ordering::SeqCst);
    SERVER_CRASH_COUNT.store(0, Ordering::SeqCst);
    SERVER_STOPPED.store(true, Ordering::SeqCst);
    remove_pidfile();
    if let Ok(mut proc) = SERVER_PROCESS.lock() {
        if let Some(child) = proc.take() {
            logging::log_message("[server] Stopping sidecar server");
            stop_watched(child.pid());
            let _ = child.kill();
            logging::log_message("[server] Server stopped");
        }
    }
    if let Ok(mut port) = SERVER_PORT.lock() {
        *port = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pidfile_round_trips_in_the_test_config_dir() {
        let recorded = PidRecord {
            pid: 4242,
            start_time: 1_760_000_000,
        };
        let path = crate::paths::config_dir().join("round-trip.pid");
        std::fs::create_dir_all(crate::paths::config_dir()).expect("test config dir");
        write_pidfile_at(&path, &recorded);
        assert_eq!(read_pidfile_at(&path), Some(recorded));
        remove_pidfile_at(&path);
        assert!(!path.exists());
    }

    fn live(exe: &str, start_time: u64) -> Process {
        Process {
            exe: exe.to_string(),
            start_time,
        }
    }

    const WHISPER_SERVER: &str = "/Applications/SpeakType.app/Contents/MacOS/whisper-server";

    use std::os::unix::fs::PermissionsExt;

    /// The watcher runs `whisper-server` from the directory it was given, with
    /// the server's own arguments, and reports its output.
    fn run_watcher(server: &std::path::Path, owner: u32) -> std::process::Output {
        let (program, argv) = watch_command(server, owner, &["-m".to_string(), "ggml".to_string()]);
        std::process::Command::new(program)
            .args(argv)
            .output()
            .expect("run the watcher")
    }

    /// Write an executable fake server and wait until it can be run. While
    /// other test threads fork, a just-written file can still be open for
    /// writing in a child and exec fails with "text file busy"; one successful
    /// probe run means no such writer is left.
    fn ready_script(path: &std::path::Path, body: &str) {
        std::fs::write(
            path,
            format!("#!/bin/sh\n[ \"$1\" = --probe ] && exit 0\n{body}\n"),
        )
        .expect("fake server body");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        for _ in 0..100 {
            if std::process::Command::new(path)
                .arg("--probe")
                .status()
                .is_ok_and(|s| s.success())
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("the fake server never became runnable");
    }

    fn fake_server(name: &str, body: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("speaktype-watch-{}-{}", std::process::id(), name));
        std::fs::create_dir_all(&dir).expect("tempdir");
        ready_script(&dir.join("whisper-server"), body);
        dir
    }

    #[test]
    fn the_watcher_runs_the_server_with_its_arguments_and_a_closed_pipe() {
        let server = fake_server(
            "argv",
            "echo \"argv=$*\"; if read _ <&9; then echo pipe-open; fi",
        )
        .join("whisper-server");
        let output = run_watcher(&server, std::process::id());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "argv=-m ggml"
        );
        assert!(output.status.success());
    }

    #[test]
    fn the_watcher_stops_the_server_when_the_app_dies() {
        let dir = fake_server("orphan", "sleep 30");
        let server = dir.join("whisper-server");
        let mut app = std::process::Command::new("/bin/sh")
            .args(["-c", "sleep 30"])
            .spawn()
            .expect("fake app");
        let pidfile = dir.join("server.pid");
        ready_script(
            &server,
            &format!("echo $$ > '{}'\nexec sleep 30", pidfile.display()),
        );
        let app_pid = app.id();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1500));
            unsafe { libc::kill(app_pid as i32, libc::SIGKILL) };
        });
        let started = Instant::now();
        let output = run_watcher(&server, app_pid);
        assert!(output.status.success());
        assert!(started.elapsed() < Duration::from_secs(15));
        app.wait().expect("reap the fake app");
        let server_pid: u32 = std::fs::read_to_string(&pidfile)
            .expect("fake server pid")
            .trim()
            .parse()
            .expect("pid");
        assert_eq!(process_spawned(server_pid), None, "server survived the app");
    }

    /// Quitting stops the server itself, not just the watcher shell around it.
    #[test]
    fn stopping_the_watcher_stops_the_server_it_runs() {
        let dir = fake_server("stop", "sleep 30");
        let server = dir.join("whisper-server");
        let pidfile = dir.join("server.pid");
        ready_script(
            &server,
            &format!("echo $$ > '{}'\nexec sleep 30", pidfile.display()),
        );
        let (program, argv) = watch_command(&server, std::process::id(), &["-m".to_string()]);
        let mut watcher = std::process::Command::new(program)
            .args(argv)
            .spawn()
            .expect("watcher");
        let deadline = Instant::now() + Duration::from_secs(5);
        let server_pid = loop {
            if let Some(pid) = std::fs::read_to_string(&pidfile)
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
            {
                break pid;
            }
            assert!(Instant::now() < deadline, "the fake server never started");
            std::thread::sleep(Duration::from_millis(20));
        };
        stop_watched(watcher.id());
        watcher.wait().expect("reap the watcher");
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            process_spawned(server_pid),
            None,
            "the server outlived stop_server"
        );
    }

    /// What a SIGTERM/SIGINT/SIGHUP handler runs before exiting: the server is
    /// stopped, not a crash to be restarted.
    #[test]
    fn the_exit_path_stops_the_server() {
        SERVER_PORT.lock().expect("port lock").replace(4711);
        SERVER_INTENTIONAL_STOP.store(false, Ordering::SeqCst);
        SERVER_CRASH_COUNT.store(2, Ordering::SeqCst);
        stop_server();
        assert_eq!(current_port(), None);
        assert!(SERVER_INTENTIONAL_STOP.load(Ordering::SeqCst));
        assert_eq!(SERVER_CRASH_COUNT.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn the_catalog_size_is_the_three_gigabytes_the_menu_promises() {
        assert_eq!(
            format!(
                "~{}",
                crate::format::format_byte_size(crate::format::catalog_model_size_bytes(
                    "large-v3"
                ))
            ),
            "~3 GB"
        );
    }

    #[test]
    fn unloading_keeps_the_model_unloaded_until_asked_to_load() {
        mark_loaded();
        assert!(!UNLOADED.load(Ordering::SeqCst));
        unload();
        assert!(UNLOADED.load(Ordering::SeqCst));
        mark_loaded();
        assert!(!UNLOADED.load(Ordering::SeqCst));
    }

    #[test]
    fn no_pidfile_means_nothing_to_reap() {
        remove_pidfile();
        reap_orphan();
        assert_eq!(read_pidfile_at(&crate::paths::server_pid_path()), None);
    }

    #[test]
    fn reap_decision_never_kills_an_unrelated_process() {
        let record = PidRecord {
            pid: 4242,
            start_time: 1_760_000_000,
        };
        assert_eq!(reap_decision(Some(4242), None, &record), Reap::Forget);
        assert_eq!(
            reap_decision(
                Some(4242),
                Some(live("/usr/bin/python3", 1_759_999_000)),
                &record
            ),
            Reap::Forget
        );
        assert_eq!(
            reap_decision(
                Some(4242),
                Some(live(WHISPER_SERVER, 1_760_000_001)),
                &record
            ),
            Reap::Forget
        );
        assert_eq!(
            reap_decision(
                Some(4242),
                Some(live(WHISPER_SERVER, 1_760_000_000)),
                &record
            ),
            Reap::Kill
        );
    }

    #[test]
    fn the_sidecar_is_found_next_to_the_app_executable() {
        let exe = std::path::Path::new("/Applications/SpeakType.app/Contents/MacOS/speaktype");
        assert_eq!(
            sidecar_beside(exe),
            std::path::Path::new("/Applications/SpeakType.app/Contents/MacOS/whisper-server")
        );
    }
}
