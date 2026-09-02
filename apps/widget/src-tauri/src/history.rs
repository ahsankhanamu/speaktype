use crate::audio;
use crate::logging::log_message;
use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_ENTRIES: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    #[serde(default)]
    pub id: String,
    pub text: String,
    pub timestamp: String,
    #[serde(default)]
    pub audio_file: Option<String>,
    #[serde(default)]
    pub sample_rate: Option<u32>,
    #[serde(default)]
    pub duration_secs: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
}

impl HistoryEntry {
    pub fn has_audio_on_disk(&self) -> bool {
        self.audio_file
            .as_ref()
            .is_some_and(|rel| History::audio_abs_path(rel).exists())
    }
}

pub fn new_entry_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)
}

impl History {
    fn config_dir() -> PathBuf {
        paths::config_dir()
    }

    fn history_path() -> PathBuf {
        Self::config_dir().join("history.json")
    }

    pub fn recordings_dir() -> PathBuf {
        Self::config_dir().join("recordings")
    }

    fn audio_abs_path(relative: &str) -> PathBuf {
        Self::config_dir().join(relative)
    }

    pub fn load() -> Self {
        let path = Self::history_path();
        let mut history = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<History>(&content) {
                    Ok(mut history) => {
                        history.migrate_entries();
                        history
                    }
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
        };

        if history.entries.iter().any(|e| e.id.is_empty()) {
            history.migrate_entries();
            let _ = history.save();
        }

        if history.prune_stale_audio_refs() {
            let _ = history.save();
        }

        history
    }

    fn prune_stale_audio_refs(&mut self) -> bool {
        let mut changed = false;
        for entry in &mut self.entries {
            if entry.audio_file.is_some() && !entry.has_audio_on_disk() {
                log_message(&format!(
                    "[history] Pruning stale audio ref for entry {} ({:?})",
                    entry.id, entry.audio_file
                ));
                entry.audio_file = None;
                entry.sample_rate = None;
                entry.duration_secs = None;
                changed = true;
            }
        }
        changed
    }

    fn migrate_entries(&mut self) {
        for entry in &mut self.entries {
            if entry.id.is_empty() {
                entry.id = new_entry_id();
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(Self::history_path(), content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn discard_saved_recording(saved: &Option<(String, f64)>) {
        if let Some((path, _)) = saved {
            Self::delete_audio_file(path);
        }
    }

    fn delete_audio_file(relative: &str) {
        let path = Self::audio_abs_path(relative);
        if path.exists() {
            if let Err(e) = fs::remove_file(&path) {
                log_message(&format!("[history] Failed to delete audio {:?}: {}", path, e));
            }
        }
    }

    fn remove_dropped_entries(entries: &[HistoryEntry]) {
        for entry in entries {
            if let Some(ref audio) = entry.audio_file {
                Self::delete_audio_file(audio);
            }
        }
    }

    pub fn save_recording(
        id: &str,
        samples: &[f32],
        sample_rate: u32,
    ) -> Result<(String, f64), String> {
        let dir = Self::recordings_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let relative = format!("recordings/{}.wav", id);
        let path = Self::audio_abs_path(&relative);
        let wav = audio::to_wav(samples, sample_rate)?;
        fs::write(&path, wav).map_err(|e| e.to_string())?;

        let duration_secs = if sample_rate > 0 {
            samples.len() as f64 / sample_rate as f64
        } else {
            0.0
        };

        Ok((relative, duration_secs))
    }

    pub fn add_entry(
        id: &str,
        text: &str,
        audio_file: Option<String>,
        sample_rate: Option<u32>,
        duration_secs: Option<f64>,
    ) -> Result<(), String> {
        let mut history = Self::load();
        let entry = HistoryEntry {
            id: id.to_string(),
            text: text.to_string(),
            timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            audio_file,
            sample_rate,
            duration_secs,
        };
        history.entries.insert(0, entry);

        if history.entries.len() > MAX_ENTRIES {
            let dropped: Vec<HistoryEntry> = history.entries.drain(MAX_ENTRIES..).collect();
            Self::remove_dropped_entries(&dropped);
        }

        history.save()
    }

    pub fn update_text(index: usize, text: &str) -> Result<(), String> {
        let mut history = Self::load();
        let entry = history
            .entries
            .get_mut(index)
            .ok_or_else(|| "Index out of range".to_string())?;
        entry.text = text.to_string();
        history.save()
    }

    pub fn delete_entry(index: usize) -> Result<(), String> {
        let mut history = Self::load();
        if index >= history.entries.len() {
            return Err("Index out of range".to_string());
        }
        let removed = history.entries.remove(index);
        if let Some(ref audio) = removed.audio_file {
            Self::delete_audio_file(audio);
        }
        history.save()
    }

    /// Remove saved audio for an entry but keep the transcription text.
    pub fn delete_entry_audio(index: usize) -> Result<(), String> {
        let mut history = Self::load();
        let entry = history
            .entries
            .get_mut(index)
            .ok_or_else(|| "Index out of range".to_string())?;
        let Some(audio) = entry.audio_file.take() else {
            return Err("This entry has no saved audio".to_string());
        };
        entry.sample_rate = None;
        entry.duration_secs = None;
        Self::delete_audio_file(&audio);
        history.save()
    }

    pub fn clear() -> Result<(), String> {
        Self::clear_recordings_dir()?;
        let history = Self::default();
        history.save()
    }

    pub fn clear_recordings_dir() -> Result<(), String> {
        let dir = Self::recordings_dir();
        if dir.exists() {
            for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                if path.is_file() {
                    fs::remove_file(&path).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    pub fn read_entry_audio_bytes(index: usize) -> Result<Vec<u8>, String> {
        let history = Self::load();
        let entry = history
            .entries
            .get(index)
            .ok_or_else(|| "Index out of range".to_string())?;
        if entry.audio_file.is_none() {
            return Err("This entry has no saved audio".to_string());
        }
        if !entry.has_audio_on_disk() {
            return Err("Audio file not found on disk".to_string());
        }
        let relative = entry.audio_file.as_ref().unwrap();
        let path = Self::audio_abs_path(relative);
        fs::read(&path).map_err(|e| format!("Failed to read audio: {}", e))
    }

    pub fn read_entry_audio(index: usize) -> Result<(Vec<f32>, u32), String> {
        let history = Self::load();
        let entry = history
            .entries
            .get(index)
            .ok_or_else(|| "Index out of range".to_string())?;
        if entry.audio_file.is_none() {
            return Err("This entry has no saved audio".to_string());
        }
        if !entry.has_audio_on_disk() {
            return Err("Audio file not found on disk".to_string());
        }
        let relative = entry.audio_file.as_ref().unwrap();
        let path = Self::audio_abs_path(relative);
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read audio: {}", e))?;
        audio::from_wav(&bytes)
    }
}
