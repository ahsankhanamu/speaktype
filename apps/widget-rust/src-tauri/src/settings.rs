use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: String,
    pub api_url: String,
    pub model: String,
    pub language: String,
    pub window_x: Option<f64>,
    pub window_y: Option<f64>,
    /// "original" = paste to the app active when recording started
    /// "active"   = paste to the app active when recording stops
    #[serde(default = "default_paste_mode")]
    pub paste_mode: String,
    /// Keys to press after pasting (e.g., "enter", "tab", "enter+enter"). None = no keys.
    #[serde(default)]
    pub post_paste_keys: Option<String>,
}

fn default_paste_mode() -> String {
    "original".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "CmdOrCtrl+Alt+L".to_string(),
            api_url: "http://127.0.0.1:8002/inference".to_string(),
            model: "medium".to_string(),
            language: "auto".to_string(),
            window_x: None,
            window_y: None,
            paste_mode: "original".to_string(),
            post_paste_keys: None,
        }
    }
}

impl Settings {
    fn config_dir() -> PathBuf {
        crate::paths::config_dir()
    }

    fn config_path() -> PathBuf {
        Self::config_dir().join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(s) => s,
                    Err(e) => {
                        crate::logging::log_message(&format!(
                            "[settings] Corrupt settings file at {:?}, backing up and using defaults: {}",
                            path, e
                        ));
                        let backup = path.with_extension("json.corrupt");
                        let _ = fs::rename(&path, &backup);
                        Self::default()
                    }
                },
                Err(e) => {
                    crate::logging::log_message(&format!(
                        "[settings] Failed to read settings file at {:?}: {}",
                        path, e
                    ));
                    Self::default()
                }
            }
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(Self::config_path(), content).map_err(|e| e.to_string())?;
        Ok(())
    }
}
