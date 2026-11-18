use crate::logging;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;

static LLM_PROCESS: Mutex<Option<CommandChild>> = Mutex::new(None);
static LLM_PORT: Mutex<Option<u16>> = Mutex::new(None);
static LLM_INTENTIONAL_STOP: AtomicBool = AtomicBool::new(false);
static LLM_CRASH_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

const LLM_DEFAULT_PORT: u16 = 8123;
const LLM_PORT_RANGE_END: u16 = 8140;
const LLM_STARTUP_TIMEOUT: Duration = Duration::from_secs(120);

pub struct LlmServerInfo {
    pub port: u16,
}

fn find_available_port() -> Option<u16> {
    use std::net::TcpListener;
    if TcpListener::bind(("127.0.0.1", LLM_DEFAULT_PORT)).is_ok() {
        return Some(LLM_DEFAULT_PORT);
    }
    std::thread::sleep(Duration::from_millis(600));
    if TcpListener::bind(("127.0.0.1", LLM_DEFAULT_PORT)).is_ok() {
        return Some(LLM_DEFAULT_PORT);
    }
    ((LLM_DEFAULT_PORT + 1)..=LLM_PORT_RANGE_END)
        .find(|&port| TcpListener::bind(("127.0.0.1", port)).is_ok())
}

/// Spawn `llama-server` (from PATH, e.g. `brew install llama.cpp`) serving the
/// given GGUF model on a free local port. The server exposes an
/// OpenAI-compatible `/v1` API that Scribe's document generation can use.
pub fn start_llm_server(
    app: &AppHandle,
    model_path: &str,
    ctx_size: usize,
) -> Result<LlmServerInfo, String> {
    if !std::path::Path::new(model_path).exists() {
        return Err(format!(
            "GGUF model not found at {:?} — pick a model in Scribe settings",
            model_path
        ));
    }

    let port = find_available_port().ok_or_else(|| {
        format!(
            "No available port in range {}-{}",
            LLM_DEFAULT_PORT, LLM_PORT_RANGE_END
        )
    })?;

    logging::log_message(&format!(
        "[scribe] Starting llama-server -m {} --port {} --ctx-size {}",
        model_path, port, ctx_size
    ));

    let mut cmd = app
        .shell()
        .command("llama-server")
        .arg("-m")
        .arg(model_path)
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--ctx-size")
        .arg(ctx_size.to_string());

    {
        let ncpu = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(6);
        let threads = ncpu.saturating_sub(2).clamp(4, 8) as u32;
        cmd = cmd.arg("-t").arg(threads.to_string());
    }

    let (mut rx, child) = cmd
        .spawn()
        .map_err(|e| format!("Failed to launch llama-server: {} — is it installed? Try `brew install llama.cpp`. ({})", e, e))?;

    logging::log_message(&format!(
        "[scribe] Spawned llama-server (pid={}, port={})",
        child.pid(),
        port
    ));

    let app_for_monitor = app.clone();
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_shell::process::CommandEvent;
        use tokio::io::AsyncWriteExt;

        let log_dir = crate::paths::config_dir_opt();
        let mut log_file = if let Some(dir) = log_dir {
            tokio::fs::create_dir_all(&dir).await.ok();
            tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("llm-server.log"))
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
                        let _ = f.write_all(b"[scribe] llama-server terminated\n").await;
                    }
                    if !LLM_INTENTIONAL_STOP.load(Ordering::SeqCst) {
                        let crashes = LLM_CRASH_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
                        logging::log_message(&format!(
                            "[scribe] llama-server terminated unexpectedly (code={:?}, crashes={})",
                            payload.code, crashes
                        ));
                        let _ = app_for_monitor.emit(
                            "scribe:llm_server:crash",
                            serde_json::json!({
                                "reason": "llama-server terminated unexpectedly",
                                "code": payload.code,
                                "crashes": crashes,
                            }),
                        );
                    }
                    if let Ok(mut proc) = LLM_PROCESS.lock() {
                        *proc = None;
                    }
                    if let Ok(mut port) = LLM_PORT.lock() {
                        *port = None;
                    }
                    break;
                }
                _ => {}
            }
        }
    });

    if let Ok(mut proc) = LLM_PROCESS.lock() {
        *proc = Some(child);
    }
    if let Ok(mut port_lock) = LLM_PORT.lock() {
        *port_lock = Some(port);
    }
    LLM_INTENTIONAL_STOP.store(false, Ordering::SeqCst);
    Ok(LlmServerInfo { port })
}

/// Poll the `/v1/models` endpoint until llama-server is ready.
pub async fn wait_for_llm_server(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{}/v1/models", port);
    let start = Instant::now();
    logging::log_message(&format!(
        "[scribe] Waiting for llama-server on port {}...",
        port
    ));

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            logging::log_message(&format!("[scribe] Failed to create HTTP client: {}", e));
            return false;
        }
    };

    loop {
        if start.elapsed() > LLM_STARTUP_TIMEOUT {
            logging::log_message("[scribe] Timed out waiting for llama-server");
            return false;
        }
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                LLM_CRASH_COUNT.store(0, Ordering::SeqCst);
                logging::log_message(&format!(
                    "[scribe] llama-server ready in {:.1}s on port {}",
                    start.elapsed().as_secs_f64(),
                    port
                ));
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

pub fn current_llm_port() -> Option<u16> {
    LLM_PORT.lock().ok().and_then(|p| *p)
}

pub fn is_llm_running() -> bool {
    current_llm_port().is_some()
}

/// If the local llama.cpp server should be used, returns its base URL.
pub fn local_base_url() -> Option<String> {
    current_llm_port().map(|p| format!("http://127.0.0.1:{}/v1", p))
}

pub fn stop_llm_server() {
    LLM_INTENTIONAL_STOP.store(true, Ordering::SeqCst);
    LLM_CRASH_COUNT.store(0, Ordering::SeqCst);
    if let Ok(mut proc) = LLM_PROCESS.lock() {
        if let Some(child) = proc.take() {
            logging::log_message("[scribe] Stopping llama-server");
            let _ = child.kill();
            logging::log_message("[scribe] llama-server stopped");
        }
    }
    if let Ok(mut port) = LLM_PORT.lock() {
        *port = None;
    }
}
