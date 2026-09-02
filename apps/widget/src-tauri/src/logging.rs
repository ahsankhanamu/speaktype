use std::fs::OpenOptions;
use std::io::Write;

const MAX_LOG_SIZE: u64 = 1_000_000; // 1MB

fn log_path() -> Option<std::path::PathBuf> {
    let dir = crate::paths::config_dir_opt()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("speaktype.log"))
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
    let line = format!("[{}] [INFO] {}\n", timestamp, msg);
    eprint!("{}", line);
    if let Some(mut f) = log_file() {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Call once at startup to rotate old logs
pub fn init_logging() {
    rotate_if_needed();
    log_message("=== SpeakType started ===");
}
