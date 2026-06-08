use crate::logging::log_message;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_ENTRIES: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub text: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
}

impl History {
    fn config_dir() -> PathBuf {
        crate::paths::config_dir()
    }

    fn history_path() -> PathBuf {
        Self::config_dir().join("history.json")
    }

    pub fn load() -> Self {
        let path = Self::history_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(history) => history,
                    Err(e) => {
                        let backup = path.with_extension("json.corrupt");
                        let _ = fs::rename(&path, &backup);
                        log_message(&format!(
                            "[history] Corrupt history.json, backed up to history.json.corrupt: {}",
                            e
                        ));
                        Self::default()
                    }
                },
                Err(e) => {
                    log_message(&format!("[history] Failed to read history.json: {}", e));
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
        fs::write(Self::history_path(), content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_entry(text: &str) -> Result<(), String> {
        let mut history = Self::load();
        let entry = HistoryEntry {
            text: text.to_string(),
            timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        };
        history.entries.insert(0, entry);
        if history.entries.len() > MAX_ENTRIES {
            history.entries.truncate(MAX_ENTRIES);
        }
        history.save()
    }

    pub fn delete_entry(index: usize) -> Result<(), String> {
        let mut history = Self::load();
        if index >= history.entries.len() {
            return Err("Index out of range".to_string());
        }
        history.entries.remove(index);
        history.save()
    }

    pub fn clear() -> Result<(), String> {
        let history = Self::default();
        history.save()
    }
}
