use serde_json::Value;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

const MAX_LOG_SIZE: u64 = 1_000_000; // 1MB

fn log_path() -> Option<std::path::PathBuf> {
    let dir = dirs::config_dir()?.join("talktype");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("talktype.log"))
}

fn log_file() -> Option<std::fs::File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path()?)
        .ok()
}

/// Rotate log if it exceeds MAX_LOG_SIZE: rename to .log.old, start fresh
fn rotate_if_needed() {
    if let Some(path) = log_path() {
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > MAX_LOG_SIZE {
                let old = path.with_extension("log.old");
                let _ = std::fs::rename(&path, &old);
            }
        }
    }
}

pub fn log_message(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{}] {}\n", timestamp, msg);
    eprint!("{}", line); // still print to stderr
    if let Some(mut f) = log_file() {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Call once at startup to rotate old logs
pub fn init_logging() {
    rotate_if_needed();
    log_message("=== TalkType started ===");
}

pub struct Sidecar {
    child: Child,
    stdin: Arc<Mutex<std::process::ChildStdin>>,
}

impl Sidecar {
    pub fn spawn(python_path: &str, script_path: &str, api_url: &str) -> Result<Self, String> {
        let python = if python_path.is_empty() {
            // Try to find python in the project .venv
            let script_dir = std::path::Path::new(script_path)
                .parent()
                .unwrap_or(std::path::Path::new("."));
            // Check script's own directory first, then one level up (project root)
            let venv_python = script_dir.join(".venv/bin/python");
            let venv_python_parent = script_dir
                .parent()
                .map(|p| p.join(".venv/bin/python"));
            if venv_python.exists() {
                venv_python.to_string_lossy().to_string()
            } else if let Some(ref parent_venv) = venv_python_parent {
                if parent_venv.exists() {
                    parent_venv.to_string_lossy().to_string()
                } else {
                    "python3".to_string()
                }
            } else {
                "python3".to_string()
            }
        } else {
            python_path.to_string()
        };

        let mut child = Command::new(&python)
            .arg(script_path)
            .arg("--sidecar")
            .arg("--api")
            .arg(api_url)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn sidecar: {} (python: {})", e, python))?;

        let stdin = child.stdin.take().ok_or("Failed to get stdin")?;

        Ok(Self {
            child,
            stdin: Arc::new(Mutex::new(stdin)),
        })
    }

    #[allow(dead_code)]
    pub fn send(&self, cmd: &Value) -> Result<(), String> {
        let mut stdin = self.stdin.lock().map_err(|e| e.to_string())?;
        let line = serde_json::to_string(cmd).map_err(|e| e.to_string())?;
        stdin
            .write_all(format!("{}\n", line).as_bytes())
            .map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn start_reader(mut self, app: AppHandle) -> Arc<Mutex<std::process::ChildStdin>> {
        let stdin = self.stdin.clone();
        let stdout = self.child.stdout.take().expect("stdout was taken");

        // Stdout reader thread — emits events to frontend
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        if let Ok(event) = serde_json::from_str::<Value>(&text) {
                            let event_name = event
                                .get("event")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            // Log non-audio_level events (audio_level is too noisy)
                            if event_name != "audio_level" {
                                log_message(&format!("[sidecar event] {}", text));
                            }
                            // Save successful transcriptions to history
                            if event_name == "pasted" {
                                if let Some(text) = event.get("text").and_then(|v| v.as_str()) {
                                    if let Err(e) = crate::history::History::add_entry(text) {
                                        log_message(&format!("[history] save error: {}", e));
                                    }
                                }
                            }
                            let _ = app.emit(&format!("sidecar:{}", event_name), &event);
                        } else {
                            log_message(&format!("[sidecar stdout] {}", text));
                        }
                    }
                    Err(e) => {
                        log_message(&format!("[sidecar stdout] read error: {}", e));
                        break;
                    }
                }
            }
            log_message("[sidecar] process ended — emitting crash event");
            let _ = app.emit("sidecar:crashed", serde_json::json!({}));
        });

        // Stderr reader thread — log to file
        if let Some(stderr) = self.child.stderr.take() {
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(text) = line {
                        log_message(&format!("[sidecar stderr] {}", text));
                    }
                }
            });
        }

        stdin
    }
}
