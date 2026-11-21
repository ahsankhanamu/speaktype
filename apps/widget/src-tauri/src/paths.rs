use std::path::PathBuf;

#[cfg(not(test))]
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("speaktype")
}

/// Tests never touch the user's real settings, history or database.
#[cfg(test)]
pub fn config_dir() -> PathBuf {
    std::env::temp_dir().join(format!("speaktype-test-{}", std::process::id()))
}

pub fn config_dir_opt() -> Option<PathBuf> {
    if cfg!(test) {
        return Some(config_dir());
    }
    dirs::config_dir().map(|d| d.join("speaktype"))
}

pub fn models_dir() -> PathBuf {
    config_dir().join("models")
}

/// Where the running model server records its pid, so a later launch can reap
/// a server an earlier crash left behind.
pub fn server_pid_path() -> PathBuf {
    config_dir().join("whisper-server.pid")
}
