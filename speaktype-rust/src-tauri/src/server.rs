use crate::logging;
use std::fs;
use std::net::TcpListener;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

static SERVER_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
static SERVER_PORT: Mutex<Option<u16>> = Mutex::new(None);

const DEFAULT_PORT: u16 = 8002;
const PORT_RANGE_END: u16 = 8020;
const POLL_INTERVAL: Duration = Duration::from_millis(200);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);

/// Find an available port starting from DEFAULT_PORT.
fn find_available_port() -> Option<u16> {
    for port in DEFAULT_PORT..=PORT_RANGE_END {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Some(port);
        }
    }
    None
}

/// Get the port the embedded server is running on.
#[allow(dead_code)]
pub fn get_server_port() -> Option<u16> {
    *SERVER_PORT.lock().unwrap()
}

/// Resolve the base path for bundled resources.
/// Tauri places them at Contents/Resources/resources/ (the extra "resources/" comes
/// from the glob pattern "resources/python/**/*" in tauri.bundle.conf.json).
fn bundled_resources_dir(app: &AppHandle) -> std::path::PathBuf {
    app.path()
        .resource_dir()
        .expect("failed to resolve resource dir")
        .join("resources")
}

/// Check whether the app bundle includes embedded server resources.
pub fn has_embedded_server(app: &AppHandle) -> bool {
    let base = bundled_resources_dir(app);
    let python_bin = base.join("python").join("bin").join("python3");
    let server_script = base.join("server").join("whisper_server.py");
    python_bin.exists() && server_script.exists()
}

/// Open (or rotate) the server log file for child process output.
fn open_server_log() -> Option<fs::File> {
    let dir = dirs::config_dir()?.join("speaktype");
    fs::create_dir_all(&dir).ok()?;
    let log_path = dir.join("server.log");
    let old_path = dir.join("server.log.old");

    // Rotate existing log
    if log_path.exists() {
        let _ = fs::rename(&log_path, &old_path);
    }

    fs::File::create(&log_path).ok()
}

/// Resolve the model directory, falling back to the first available bundled model.
fn resolve_model_dir(base: &std::path::Path, requested_model: &str) -> Option<std::path::PathBuf> {
    let preferred = base.join("models").join(format!("whisper-{}", requested_model));
    if preferred.exists() {
        return Some(preferred);
    }

    // Scan for any bundled model as fallback
    let models_dir = base.join("models");
    if let Ok(entries) = fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("whisper-") && entry.path().is_dir() {
                logging::log_message(&format!(
                    "[server] Model '{}' not bundled, falling back to '{}'",
                    requested_model,
                    name_str.trim_start_matches("whisper-")
                ));
                return Some(entry.path());
            }
        }
    }

    logging::log_message(&format!(
        "[server] No bundled models found in {}",
        models_dir.display()
    ));
    None
}

pub struct ServerInfo {
    pub port: u16,
    pub model: String,
}

/// Spawn the embedded whisper server as a child process.
/// Returns server info (port + actual model used), or None on failure.
pub fn start_server(app: &AppHandle) -> Option<ServerInfo> {
    let settings = crate::settings::Settings::load();
    let base = bundled_resources_dir(app);

    let python_bin = base.join("python").join("bin").join("python3");
    let server_script = base.join("server").join("whisper_server.py");

    let model_dir = match resolve_model_dir(&base, &settings.model) {
        Some(dir) => dir,
        None => return None,
    };
    let actual_model = model_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .trim_start_matches("whisper-")
        .to_string();

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
        "[server] Starting embedded server: {} {} --model {} --port {}",
        python_bin.display(),
        server_script.display(),
        model_dir.display(),
        port,
    ));

    let mut cmd = Command::new(&python_bin);
    cmd.arg(&server_script)
        .arg("--model")
        .arg(&model_dir)
        .arg("--port")
        .arg(port.to_string())
        .arg("--device")
        .arg("cpu")
        .arg("--compute")
        .arg("auto");

    // Redirect stdout/stderr to log file; prevent stdin inheritance
    cmd.stdin(std::process::Stdio::null());
    if let Some(log_file) = open_server_log() {
        let log_clone = log_file.try_clone().ok();
        cmd.stdout(log_file);
        if let Some(stderr_file) = log_clone {
            cmd.stderr(stderr_file);
        }
    }

    match cmd.spawn() {
        Ok(child) => {
            logging::log_message(&format!("[server] Spawned server (pid={}, port={}, model={})", child.id(), port, actual_model));
            *SERVER_PROCESS.lock().unwrap() = Some(child);
            *SERVER_PORT.lock().unwrap() = Some(port);
            Some(ServerInfo { port, model: actual_model })
        }
        Err(e) => {
            logging::log_message(&format!("[server] Failed to start server: {}", e));
            None
        }
    }
}

/// Poll the health endpoint until the server is ready.
/// Returns `true` if the server became ready within the timeout.
pub async fn wait_for_server(port: u16) -> bool {
    let health_url = format!("http://127.0.0.1:{}/health", port);
    let start = Instant::now();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    logging::log_message(&format!("[server] Waiting for server on port {}...", port));

    loop {
        if start.elapsed() > STARTUP_TIMEOUT {
            logging::log_message("[server] Timed out waiting for server");
            return false;
        }

        match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                logging::log_message(&format!(
                    "[server] Server ready in {:.1}s on port {}",
                    start.elapsed().as_secs_f64(),
                    port,
                ));
                return true;
            }
            _ => {}
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Kill the embedded server process if it's running.
pub fn stop_server() {
    let mut proc = SERVER_PROCESS.lock().unwrap();
    if let Some(ref mut child) = *proc {
        logging::log_message(&format!("[server] Stopping server (pid={})", child.id()));
        let _ = child.kill();
        let _ = child.wait();
        logging::log_message("[server] Server stopped");
    }
    *proc = None;
    *SERVER_PORT.lock().unwrap() = None;
}
