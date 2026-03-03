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
}

fn default_paste_mode() -> String {
    "original".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "CmdOrCtrl+Alt+L".to_string(),
            api_url: "http://localhost:8002/transcribe".to_string(),
            model: "base".to_string(),
            language: "auto".to_string(),
            window_x: None,
            window_y: None,
            paste_mode: "original".to_string(),
        }
    }
}

impl Settings {
    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("speaktype")
    }

    fn config_path() -> PathBuf {
        Self::config_dir().join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
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
