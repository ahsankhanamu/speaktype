use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("speaktype")
}

pub fn config_dir_opt() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("speaktype"))
}

#[allow(dead_code)]
pub fn models_dir() -> PathBuf {
    config_dir().join("models")
}
