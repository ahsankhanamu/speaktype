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
    pub onboarding_window_x: Option<f64>,
    pub onboarding_window_y: Option<f64>,
    /// "original" = paste to the app active when recording started
    /// "active"   = paste to the app active when recording stops
    #[serde(default = "default_paste_mode")]
    pub paste_mode: String,
    /// Keys to press after pasting (e.g., "enter", "tab", "enter+enter"). None = no keys.
    #[serde(default = "default_post_paste_keys")]
    pub post_paste_keys: Option<String>,
    /// When true, save WAV files for each successful transcription (for reprocessing).
    #[serde(default = "default_save_recordings")]
    pub save_recordings: bool,
    /// Appearance: "auto" | "light" | "dark"
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_paste_mode() -> String {
    "active".to_string()
}

fn default_post_paste_keys() -> Option<String> {
    Some("enter".to_string())
}

fn default_save_recordings() -> bool {
    false
}

fn default_theme() -> String {
    "auto".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Super+Control".to_string(),
            api_url: "http://127.0.0.1:8002/inference".to_string(),
            model: "medium".to_string(),
            language: "auto".to_string(),
            window_x: None,
            window_y: None,
            onboarding_window_x: None,
            onboarding_window_y: None,
            paste_mode: "active".to_string(),
            post_paste_keys: Some("enter".to_string()),
            save_recordings: false,
            theme: "auto".to_string(),
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

    /// Map legacy cross-platform modifiers to Super (Command on macOS, Windows key on Windows).
    pub fn normalize_hotkey(hotkey: &str) -> String {
        hotkey
            .split('+')
            .map(|part| match part.trim() {
                "CmdOrCtrl" | "CommandOrControl" | "Command" | "Meta" => "Super".to_string(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join("+")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let settings = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Settings>(&content) {
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
        };

        let normalized = Self::normalize_hotkey(&settings.hotkey);
        if normalized != settings.hotkey {
            let mut migrated = settings;
            migrated.hotkey = normalized;
            let _ = migrated.save();
            migrated
        } else {
            settings
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
