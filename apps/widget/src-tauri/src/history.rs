use crate::audio;
use crate::logging::log_message;
use crate::paths;
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri_specta::Event;

/// How a dictation left the app: pasted into another app, left on the
/// clipboard because the paste failed, or nothing to paste.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DictationOutcome {
    Pasted,
    Copied,
    PasteFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
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
    /// Missing on entries written before outcomes existed.
    #[serde(default)]
    pub outcome: Option<DictationOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
}

/// Typed event: the history file was written (dictation added, entry deleted,
/// re-pasted, reprocessed or cleared). Windows refetch on it.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct HistoryChanged {}

/// Where `HistoryChanged` goes: the app's emitter, or a counter in tests.
#[cfg_attr(test, allow(dead_code))]
pub(crate) trait HistoryEventSink {
    fn emit_history_changed(&self) -> Result<(), String>;
}

impl HistoryEventSink for tauri::AppHandle {
    fn emit_history_changed(&self) -> Result<(), String> {
        HistoryChanged {}.emit(self).map_err(|e| e.to_string())
    }
}

/// Best-effort: a window that missed the event refetches when it next opens.
pub(crate) fn emit_changed(sink: &impl HistoryEventSink) {
    if let Err(e) = sink.emit_history_changed() {
        log_message(&format!("[history] HistoryChanged emit failed: {e}"));
    }
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

/// How many dictations are kept; `None` (the setting's "Forever") is every
/// entry the file can hold.
pub fn retention_limit(settings: &Settings) -> usize {
    settings
        .history_limit
        .map_or(usize::MAX, |limit| limit as usize)
}

/// A take's saved WAV, if `save_recordings` kept one.
#[derive(Debug, Clone)]
pub struct SavedRecording {
    pub file: String,
    pub sample_rate: u32,
    pub duration_secs: f64,
}

/// Newest-first trim that also deletes the dropped recordings, so retention
/// longer than the file's remaining entries is never silently exceeded.
pub fn trim(entries: &mut Vec<HistoryEntry>, limit: usize) {
    if entries.len() <= limit {
        return;
    }
    let dropped: Vec<HistoryEntry> = entries.drain(limit..).collect();
    History::remove_dropped_entries(&dropped);
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
        Self::load_from(&Self::history_path())
    }

    fn load_from(path: &Path) -> Self {
        let mut history = if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str::<History>(&content) {
                    Ok(mut history) => {
                        history.migrate_entries();
                        history
                    }
                    Err(e) => {
                        let backup = path.with_extension("json.corrupt");
                        let _ = fs::rename(path, &backup);
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

        let had_anonymous = history.entries.iter().any(|e| e.id.is_empty());
        if had_anonymous {
            history.migrate_entries();
        }
        let pruned = history.prune_stale_audio_refs();
        if had_anonymous || pruned {
            if let Err(e) = history.save_to(path) {
                log_message(&format!("[history] Failed to save migrated history: {}", e));
            }
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
        self.save_to(&Self::history_path())
    }

    fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())
    }

    pub fn newest_text(&self) -> Option<String> {
        self.entries
            .iter()
            .map(|entry| entry.text.trim())
            .find(|text| !text.is_empty())
            .map(str::to_string)
    }

    pub fn discard_saved_recording(saved: &Option<SavedRecording>) {
        if let Some(saved) = saved {
            Self::delete_audio_file(&saved.file);
        }
    }

    fn delete_audio_file(relative: &str) {
        let path = Self::audio_abs_path(relative);
        if path.exists() {
            if let Err(e) = fs::remove_file(&path) {
                log_message(&format!(
                    "[history] Failed to delete audio {:?}: {}",
                    path, e
                ));
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
    ) -> Result<SavedRecording, String> {
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

        Ok(SavedRecording {
            file: relative,
            sample_rate,
            duration_secs,
        })
    }

    /// Prepend one dictation and apply the retention limit.
    pub fn add_entry(
        &mut self,
        id: &str,
        text: &str,
        outcome: DictationOutcome,
        limit: usize,
        audio: Option<SavedRecording>,
    ) {
        self.entries.insert(
            0,
            HistoryEntry {
                id: id.to_string(),
                text: text.to_string(),
                timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                audio_file: audio.as_ref().map(|a| a.file.clone()),
                sample_rate: audio.as_ref().map(|a| a.sample_rate),
                duration_secs: audio.as_ref().map(|a| a.duration_secs),
                outcome: Some(outcome),
            },
        );
        trim(&mut self.entries, limit);
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

    /// Re-transcribe `index` with the active model and return the new text
    /// without touching the entry: onboarding's compare shows it beside the
    /// original. Audio comes from a saved recording, or from the last dictation
    /// kept in memory, so this works with *Save recording audio* off. It never
    /// restarts the model server — a compare that warmed a different model
    /// would make "Back to <previous>" a restart, not a no-op.
    pub async fn reprocess(
        app: &tauri::AppHandle,
        index: usize,
        settings: &Settings,
        newest: Option<(String, Vec<f32>, u32)>,
    ) -> Result<String, String> {
        let history = Self::load();
        let entry = history
            .entries
            .get(index)
            .ok_or_else(|| "Index out of range".to_string())?;

        let (samples, sample_rate) = match newest {
            Some((id, samples, rate)) if id == entry.id => (samples, rate),
            _ => Self::read_entry_audio(index)?,
        };
        let duration_secs = samples.len() as f64 / sample_rate as f64;
        let wav = audio::to_wav(&audio::normalize_audio(&samples), sample_rate)?;

        crate::debug::start_session(&settings.model);
        crate::debug::capture_audio(&samples, sample_rate);
        match crate::transcribe::transcribe(app, wav, settings, duration_secs).await {
            Ok(text) if !text.is_empty() => {
                crate::debug::update_meta(duration_secs, "reprocess");
                crate::debug::finish_session(&text);
                Ok(text)
            }
            Ok(_) => {
                crate::debug::discard_session();
                Err("No speech detected in recording".to_string())
            }
            Err(e) => {
                crate::debug::discard_session();
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct HistoryEvents(AtomicUsize);

    impl HistoryEventSink for HistoryEvents {
        fn emit_history_changed(&self) -> Result<(), String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn entry(id: &str, outcome: Option<DictationOutcome>) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            text: "hello".to_string(),
            timestamp: "2026-11-19T10:00:00".to_string(),
            audio_file: None,
            sample_rate: None,
            duration_secs: None,
            outcome,
        }
    }

    #[test]
    fn outcome_roundtrips_and_old_entries_have_none() {
        let legacy = r#"{"entries":[{"id":"a","text":"old","timestamp":"2026-01-01T00:00:00"}]}"#;
        let old: History = serde_json::from_str(legacy).unwrap();
        assert_eq!(old.entries[0].outcome, None);

        let mut history = History::default();
        history.add_entry("b", "new", DictationOutcome::PasteFailed, 100, None);
        let json = serde_json::to_string(&history).unwrap();
        assert!(json.contains(r#""outcome":"pasteFailed""#), "{json}");
        let back: History = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entries[0].outcome, Some(DictationOutcome::PasteFailed));
    }

    #[test]
    fn every_outcome_keeps_its_wire_name() {
        for (outcome, wire) in [
            (DictationOutcome::Pasted, "pasted"),
            (DictationOutcome::Copied, "copied"),
            (DictationOutcome::PasteFailed, "pasteFailed"),
        ] {
            assert_eq!(serde_json::to_value(outcome).unwrap(), wire);
        }
    }

    #[test]
    fn history_limit_is_optional_and_forever_is_its_absence() {
        let mut history = History::default();
        for i in 0..3 {
            history.add_entry(&format!("{i}"), "t", DictationOutcome::Pasted, 2, None);
        }
        let ids: Vec<_> = history.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["2", "1"]);

        let forever = Settings {
            history_limit: None,
            ..Settings::default()
        };
        assert_eq!(retention_limit(&forever), usize::MAX, "None is forever");
        let custom = Settings {
            history_limit: Some(7),
            ..Settings::default()
        };
        assert_eq!(retention_limit(&custom), 7);
        let default = Settings::default();
        assert_eq!(retention_limit(&default), 100, "new and old files keep 100");
    }

    #[test]
    fn delete_entry_emits_history_changed_and_persists() {
        let mut seeded = History::default();
        seeded.add_entry("first", "one", DictationOutcome::Pasted, 100, None);
        seeded.add_entry("second", "two", DictationOutcome::Pasted, 100, None);
        seeded.save().expect("seed history");

        History::delete_entry(0).expect("delete");

        let sink = HistoryEvents::default();
        emit_changed(&sink);
        assert_eq!(sink.0.load(Ordering::SeqCst), 1);
        let remaining = History::load();
        assert_eq!(remaining.entries.len(), 1);
        assert_eq!(remaining.entries[0].id, "first");
    }

    #[test]
    fn newest_text_skips_blank_entries() {
        let mut history = History::default();
        let mut blank = entry("a", None);
        blank.text = "  ".to_string();
        history.entries.push(blank);
        let mut latest = entry("b", None);
        latest.text = " latest line ".to_string();
        history.entries.push(latest);
        assert_eq!(history.newest_text().as_deref(), Some("latest line"));
        assert_eq!(History::default().newest_text(), None);
    }

    #[test]
    fn trim_deletes_dropped_recordings() {
        let mut history = History::default();
        history.entries.push(entry("a", None));
        history.entries.push(entry("b", None));
        trim(&mut history.entries, 1);
        assert_eq!(history.entries.len(), 1);
    }
}
