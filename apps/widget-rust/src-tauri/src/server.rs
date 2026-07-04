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
    for port in (DEFAULT_PORT + 1)..=PORT_RANGE_END {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Some(port);
        }
    }
    None
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
            let reason = if model_path.exists() { "incomplete" } else { "missing" };
            logging::log_message(&format!(
                "[server] No complete model available for '{}', not starting server",
                model
            ));
            let _ = app.emit("server:model_needed", serde_json::json!({
                "model": model,
                "path": model_path.to_string_lossy(),
                "reason": reason,
            }));
            return None;
        }
    };

    // Resolve model path
    let data_dir = crate::paths::config_dir_opt()?;
    let model_path = data_dir.join("models").join(format!("ggml-{}.bin", model));

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

    let sidecar_cmd = match app.shell().sidecar("whisper-server") {
        Ok(cmd) => cmd,
        Err(e) => {
            logging::log_message(&format!("[server] Failed to create sidecar command: {}", e));
            return None;
        }
    };

    let mut sidecar_cmd = sidecar_cmd
        .arg("-m")
        .arg(&model_path.to_string_lossy().to_string())
        .arg("--port")
        .arg(port.to_string());

    if let Some(vad_path) = crate::downloader::ensure_vad_model_blocking() {
        logging::log_message(&format!(
            "[server] Enabling Silero VAD ({})",
            vad_path.display()
        ));
        sidecar_cmd = sidecar_cmd
            .arg("--vad")
            .arg("--vad-model")
            .arg(vad_path.to_string_lossy().to_string())
            .arg("--vad-threshold")
            .arg("0.45")
            .arg("--suppress-nst");
    } else {
        logging::log_message("[server] Silero VAD unavailable — starting without VAD trim");
    }

    match sidecar_cmd.spawn() {
        Ok((mut rx, child)) => {
            logging::log_message(&format!("[server] Spawned server (pid={}, port={}, model={})", child.pid(), port, model));
            
            // Log sidecar output and monitor for unexpected termination
            let app_handle_for_monitor = app.clone();
            tauri::async_runtime::spawn(async move {
                use tokio::io::AsyncWriteExt;
                use tauri_plugin_shell::process::CommandEvent;

                let log_dir = crate::paths::config_dir_opt();
                let mut log_file = if let Some(dir) = log_dir {
                    tokio::fs::create_dir_all(&dir).await.ok();
                    match tokio::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(dir.join("server.log"))
                        .await {
                            Ok(f) => Some(f),
                            Err(_) => None,
                    }
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
            Some(ServerInfo { port, model: model.to_string() })
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
        .build() {
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
        let port = current_port().ok_or_else(|| "Server is running but port is unknown".to_string())?;
        return Ok(Some(ServerInfo {
            port,
            model: model.to_string(),
        }));
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

pub fn stop_server() {
    SERVER_INTENTIONAL_STOP.store(true, Ordering::SeqCst);
    SERVER_CRASH_COUNT.store(0, Ordering::SeqCst);
    if let Ok(mut proc) = SERVER_PROCESS.lock() {
        if let Some(child) = proc.take() {
            logging::log_message("[server] Stopping sidecar server");
            let _ = child.kill();
            logging::log_message("[server] Server stopped");
        }
    }
    if let Ok(mut port) = SERVER_PORT.lock() {
        *port = None;
    }
}
