use serde::{Deserialize, Serialize};
use speaktype_stt::{ProviderConfig, SecretStore};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use tauri_specta::Event;

use crate::session_cmds::IpcError;
use crate::shortcuts::{self, ShortcutAction};

/// The whisper-server address the app writes on every start; an `api_url`
/// still holding it means On this Mac.
pub(crate) const DEFAULT_API_URL: &str = "http://127.0.0.1:8002/inference";

/// The app's own whisper-server picks a free local port at each launch
/// (`server.rs`), so any `http://127.0.0.1:<port>/inference` is On this Mac.
fn is_app_managed_url(url: &str) -> bool {
    url.strip_prefix("http://127.0.0.1:")
        .and_then(|rest| rest.strip_suffix("/inference"))
        .is_some_and(|port| port.parse::<u16>().is_ok())
}

/// Keychain account for the AI-writing key, moved out of settings.json.
pub(crate) const LLM_KEY_ACCOUNT: &str = "llm.api_key";

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Settings {
    pub api_url: String,
    pub model: String,
    pub language: String,
    pub window_x: Option<f64>,
    pub window_y: Option<f64>,
    pub onboarding_window_x: Option<f64>,
    pub onboarding_window_y: Option<f64>,
    /// Settings window geometry in logical pixels; None until the user moves or resizes it.
    #[serde(default)]
    pub settings_x: Option<f64>,
    #[serde(default)]
    pub settings_y: Option<f64>,
    #[serde(default)]
    pub settings_w: Option<f64>,
    #[serde(default)]
    pub settings_h: Option<f64>,
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
    /// Score transcriptions for hallucination, drop bad segments, and re-decode
    /// suspicious results. Off falls back to accepting whatever the model emits.
    #[serde(default = "default_hallucination_guard")]
    pub hallucination_guard: bool,
    /// Extra decode passes allowed when a result looks hallucinated (max 2).
    #[serde(default = "default_hallucination_retries")]
    pub hallucination_retries: u32,
    /// Above this probability a segment is silence being transcribed as speech.
    #[serde(default = "default_quality_no_speech_prob")]
    pub quality_no_speech_prob: f32,
    /// Below this mean token log-probability the model was guessing.
    #[serde(default = "default_quality_avg_logprob")]
    pub quality_avg_logprob: f32,
    /// Above this gzip ratio the text is repetitive enough to be a decode loop.
    #[serde(default = "default_quality_compression_ratio")]
    pub quality_compression_ratio: f32,
    /// Named input device to capture from. None/empty = use the macOS default
    /// input (which jumps around as Bluetooth/iPhone/mic connect and disconnect).
    #[serde(default)]
    pub input_device: Option<String>,
    /// The registry's values, migrated from the old `hotkey`,
    /// `scribe_screenshot_hotkey`, `session_hotkey` and `draw_key`.
    #[serde(default)]
    pub shortcuts: BTreeMap<ShortcutAction, String>,
    /// Scribe: master mode switch. When enabled, dictation, clipboard copies,
    /// and screenshots are captured into an auto-started session. The toggle is
    /// a mode switch — the mic stays the only recording control.
    #[serde(default = "default_scribe_enabled")]
    pub scribe_enabled: bool,
    /// Scribe: whether clipboard monitoring captures copied text/images during a session.
    #[serde(default = "default_scribe_clipboard_monitor")]
    pub scribe_clipboard_monitor: bool,
    /// Scribe: upper bound on screenshots stored per session (oldest deleted first).
    #[serde(default = "default_scribe_max_screenshots")]
    pub scribe_max_screenshots: usize,
    /// Scribe: default document template used by session generation.
    #[serde(default = "default_scribe_template")]
    pub scribe_template: String,
    /// What a copied session starts with: `none` (default), `auto` (a persona
    /// picked from what was said) or a persona key from `templates`.
    #[serde(default = "default_session_copy_preface")]
    pub session_copy_preface: String,
    /// Scribe: OpenAI-compatible LLM configuration. Empty base_url means the LLM
    /// assembly layer is disabled (MVP runs locally without it).
    #[serde(default = "default_scribe_llm_base_url")]
    pub scribe_llm_base_url: String,
    #[serde(default)]
    pub scribe_llm_api_key: String,
    /// Where transcription comes from. Missing in files written before the
    /// Providers page existed; [`Settings::load_from`] fills it from `api_url`.
    #[serde(default)]
    pub stt_provider: Option<ProviderConfig>,
    #[serde(default = "default_scribe_llm_model")]
    pub scribe_llm_model: String,
    /// Scribe: use the local llama.cpp server (llama-server) instead of the
    /// configured OpenAI-compatible base URL.
    #[serde(default = "default_scribe_llm_use_local")]
    pub scribe_llm_use_local: bool,
    /// Scribe: absolute path to the GGUF model served by llama-server.
    #[serde(default)]
    pub scribe_llm_local_model: String,
    /// Scribe: context size for the local llama-server.
    #[serde(default = "default_scribe_llm_local_ctx")]
    pub scribe_llm_local_ctx: usize,
    /// Settings schema. Files written before v2 have no key: they are existing
    /// installs, so [`Settings::load_from`] keeps their `post_paste_keys`.
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_sounds")]
    pub sounds: bool,
    /// "small" | "regular" | "large"
    #[serde(default = "default_widget_size")]
    pub widget_size: String,
    /// "always" | "active" | "never"
    #[serde(default = "default_widget_visibility")]
    pub widget_visibility: String,
    /// "orange" | "red" | "amber" | "green" | "blue" | "purple" | "white"
    #[serde(default = "default_ink_color")]
    pub ink_color: String,
    /// "thin" | "regular" | "bold"
    #[serde(default = "default_ink_thickness")]
    pub ink_thickness: String,
    /// Always-on drawing; Phase 2 reads it.
    #[serde(default = "default_draw_anytime")]
    pub draw_anytime: bool,
    /// Save each mark as a screenshot (needs Screen Recording). Off: ink only.
    #[serde(default = "default_save_marks")]
    pub save_marks: bool,
    /// Kept history entries; None = forever.
    #[serde(default = "default_history_limit")]
    pub history_limit: Option<u32>,
    /// Dictations older than this many days are removed; None keeps any age.
    #[serde(default)]
    pub history_max_days: Option<u32>,
    #[serde(default)]
    pub launch_at_login: bool,
    /// Last onboarding step reached, for resume.
    #[serde(default)]
    pub onboarding_step: u8,
    /// Show the latest transcribed line under the HUD during a session.
    #[serde(default = "default_live_captions")]
    pub live_captions: bool,
    /// Keep a session's audio on disk.
    #[serde(default)]
    pub keep_session_audio: bool,
    /// "bottom_right" | "bottom_center" | "top_right" | "bottom_left" |
    /// "custom" (where the user last dragged it).
    #[serde(default = "default_widget_corner")]
    pub widget_corner: String,
}

fn default_live_captions() -> bool {
    true
}

fn default_widget_corner() -> String {
    "bottom_right".to_string()
}

fn default_paste_mode() -> String {
    "active".to_string()
}

fn default_post_paste_keys() -> Option<String> {
    Some("enter".to_string())
}

/// Whether a settings file has the `version` key. One without it predates v2.
fn file_has_version(json: &str) -> bool {
    let Ok(file) =
        serde_json::from_str::<std::collections::BTreeMap<String, serde_json::Value>>(json)
    else {
        return false;
    };
    file.contains_key("version")
}

fn default_save_recordings() -> bool {
    false
}

fn default_theme() -> String {
    "auto".to_string()
}

fn default_hallucination_guard() -> bool {
    true
}

fn default_hallucination_retries() -> u32 {
    2
}

fn default_quality_no_speech_prob() -> f32 {
    0.6
}

fn default_quality_avg_logprob() -> f32 {
    -1.0
}

fn default_quality_compression_ratio() -> f32 {
    2.4
}

/// A file written before the registry kept these four outside `shortcuts`.
#[derive(Deserialize)]
struct LegacyShortcuts {
    hotkey: String,
    #[serde(default = "default_legacy_screenshot")]
    scribe_screenshot_hotkey: String,
    #[serde(default = "default_session_hotkey")]
    session_hotkey: String,
    #[serde(default = "default_legacy_draw_key")]
    draw_key: String,
}

fn default_legacy_screenshot() -> String {
    "Super+Option+S".to_string()
}

fn default_legacy_draw_key() -> String {
    "right_option".to_string()
}

/// "Option+Super+K", in the app's modifier order (⌥⌘K).
pub(crate) fn default_session_hotkey() -> String {
    "Option+Super+K".to_string()
}

impl LegacyShortcuts {
    /// Keyed by the action's camelCase nickname, which is also how serde reads
    /// the enum in `shortcuts`.
    fn into_map(self) -> BTreeMap<ShortcutAction, String> {
        [
            (ShortcutAction::Dictation, self.hotkey),
            (ShortcutAction::Session, self.session_hotkey),
            (ShortcutAction::Screenshot, self.scribe_screenshot_hotkey),
            (ShortcutAction::DrawKey, self.draw_key),
        ]
        .into_iter()
        .map(|(action, keys)| {
            let stored = shortcuts::store(action, &keys)
                .unwrap_or_else(|| action.default_keys().to_string());
            (action, stored)
        })
        .collect()
    }
}

fn default_scribe_enabled() -> bool {
    false
}

fn default_scribe_clipboard_monitor() -> bool {
    true
}

fn default_scribe_max_screenshots() -> usize {
    50
}

fn default_scribe_template() -> String {
    "auto".to_string()
}

fn default_session_copy_preface() -> String {
    "none".to_string()
}

fn default_scribe_llm_base_url() -> String {
    String::new()
}

fn default_scribe_llm_model() -> String {
    String::new()
}

fn default_scribe_llm_use_local() -> bool {
    false
}

fn default_scribe_llm_local_ctx() -> usize {
    4096
}

fn default_version() -> u32 {
    2
}

fn default_sounds() -> bool {
    true
}

fn default_widget_size() -> String {
    "regular".to_string()
}

fn default_widget_visibility() -> String {
    "always".to_string()
}

fn default_ink_color() -> String {
    "orange".to_string()
}

fn default_ink_thickness() -> String {
    "regular".to_string()
}

fn default_save_marks() -> bool {
    true
}

fn default_draw_anytime() -> bool {
    true
}

fn default_history_limit() -> Option<u32> {
    Some(100)
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_API_URL.to_string(),
            model: "medium".to_string(),
            language: "auto".to_string(),
            window_x: None,
            window_y: None,
            onboarding_window_x: None,
            onboarding_window_y: None,
            settings_x: None,
            settings_y: None,
            settings_w: None,
            settings_h: None,
            paste_mode: "active".to_string(),
            post_paste_keys: None,
            save_recordings: false,
            theme: "auto".to_string(),
            hallucination_guard: default_hallucination_guard(),
            hallucination_retries: default_hallucination_retries(),
            quality_no_speech_prob: default_quality_no_speech_prob(),
            quality_avg_logprob: default_quality_avg_logprob(),
            quality_compression_ratio: default_quality_compression_ratio(),
            input_device: None,
            scribe_enabled: default_scribe_enabled(),
            scribe_clipboard_monitor: default_scribe_clipboard_monitor(),
            scribe_max_screenshots: default_scribe_max_screenshots(),
            scribe_template: default_scribe_template(),
            session_copy_preface: default_session_copy_preface(),
            scribe_llm_base_url: default_scribe_llm_base_url(),
            scribe_llm_api_key: String::new(),
            stt_provider: None,
            scribe_llm_model: default_scribe_llm_model(),
            scribe_llm_use_local: default_scribe_llm_use_local(),
            scribe_llm_local_model: String::new(),
            scribe_llm_local_ctx: default_scribe_llm_local_ctx(),
            version: default_version(),
            sounds: default_sounds(),
            widget_size: default_widget_size(),
            widget_visibility: default_widget_visibility(),
            ink_color: default_ink_color(),
            ink_thickness: default_ink_thickness(),
            draw_anytime: default_draw_anytime(),
            save_marks: default_save_marks(),
            history_limit: default_history_limit(),
            history_max_days: None,
            launch_at_login: false,
            onboarding_step: 0,
            live_captions: default_live_captions(),
            keep_session_audio: false,
            widget_corner: default_widget_corner(),
            shortcuts: shortcuts::default_bindings(&BTreeMap::new()),
        }
    }
}

/// Typed event: settings changed through any path that writes them (the
/// settings window, the widget's context menus). Windows refetch on it.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SettingsChanged {}

/// Where `SettingsChanged` goes: the app's emitter, or a counter in tests.
#[cfg_attr(test, allow(dead_code))]
pub(crate) trait SettingsEventSink {
    fn emit_settings_changed(&self) -> Result<(), String>;
}

impl SettingsEventSink for tauri::AppHandle {
    fn emit_settings_changed(&self) -> Result<(), String> {
        SettingsChanged {}.emit(self).map_err(|e| e.to_string())
    }
}

/// Best-effort: a window that missed the event refetches on its next open.
pub(crate) fn emit_settings_changed(sink: &impl SettingsEventSink) {
    if let Err(e) = sink.emit_settings_changed() {
        crate::logging::log_message(&format!("[settings] SettingsChanged emit failed: {e}"));
    }
}

/// Launch at login through `tauri-plugin-autostart` (a LaunchAgent on macOS),
/// then persist the flag: Rust is the single source of truth, so the settings
/// window reads it back rather than tracking its own copy. The handle is a
/// parameter so the not-running case is testable.
pub(crate) fn set_launch_at_login<R: tauri::Runtime>(
    app: Option<&tauri::AppHandle<R>>,
    on: bool,
    settings: &mut Settings,
) -> Result<(), IpcError> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app
        .ok_or_else(|| IpcError::new("launchAtLogin", "SpeakType is not running."))?
        .autolaunch();
    if on {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|e| IpcError::new("launchAtLogin", e.to_string()))?;
    settings.launch_at_login = on;
    settings
        .save()
        .map_err(|e| IpcError::new("launchAtLogin", e))
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
        let read = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => Some(content),
                Err(e) => {
                    crate::logging::log_message(&format!(
                        "[settings] Failed to read settings file at {:?}: {}",
                        path, e
                    ));
                    None
                }
            }
        } else {
            None
        };
        let keychain = speaktype_stt::KeychainStore;
        let file = match &read {
            Some(content) => Ok(Some(content.as_str())),
            None => Ok(None),
        };
        let (mut settings, changed) = match Self::load_from(file, &keychain) {
            Ok(migrated) => migrated,
            Err(e) => {
                crate::logging::log_message(&format!(
                    "[settings] Corrupt settings file at {:?}, backing up and using defaults: {}",
                    path, e
                ));
                let _ = fs::rename(&path, path.with_extension("json.corrupt"));
                (Self::default(), true)
            }
        };

        if changed {
            if let Err(e) = settings.save() {
                crate::logging::log_message(&format!("[settings] Failed to save migration: {}", e));
            }
        }
        settings.resolve_provider();
        settings
    }

    /// Parse the settings file, migrating out-of-date fields. The second value
    /// reports whether `settings` needs writing back. `Ok(None)` is a fresh
    /// install and `Err` a corrupt file: both fall back to [`Settings::default`],
    /// which pastes nothing after dictation (spec §6.7) and carries `version: 2`.
    /// A file without a `version` key is an existing install: it keeps its
    /// `post_paste_keys` and only gains the key.
    pub fn load_from(
        file: Result<Option<&str>, String>,
        secrets: &dyn SecretStore,
    ) -> Result<(Self, bool), String> {
        let json = file?;
        let (mut settings, mut changed) = match json {
            Some(json) => {
                let settings: Settings = serde_json::from_str(json).map_err(|e| e.to_string())?;
                let changed = !file_has_version(json);
                (settings, changed)
            }
            None => (Self::default(), true),
        };
        if settings.version < 2 {
            settings.version = 2;
            changed = true;
        }
        if settings.shortcuts.is_empty() {
            if let Some(json) = json {
                let legacy: LegacyShortcuts =
                    serde_json::from_str(json).map_err(|e| e.to_string())?;
                settings.shortcuts = legacy.into_map();
            }
            changed = true;
        }
        if settings.stt_provider.is_none() {
            settings.stt_provider = Some(if is_app_managed_url(&settings.api_url) {
                ProviderConfig::Local {
                    model: settings.model.clone(),
                }
            } else {
                ProviderConfig::Server {
                    base_url: settings.api_url.clone(),
                }
            });
            changed = true;
        }

        // The Small widget size was dropped (too close to Regular to matter).
        if settings.widget_size == "small" {
            settings.widget_size = "regular".to_string();
            changed = true;
        }

        let llm_key = settings.scribe_llm_api_key.trim().to_string();
        if !llm_key.is_empty() {
            secrets
                .set(LLM_KEY_ACCOUNT, &llm_key)
                .map_err(|e| e.to_string())?;
            settings.scribe_llm_api_key = String::new();
            changed = true;
        }

        Ok((settings, changed))
    }

    /// The provider in force. A missing setting can only come from a value the
    /// app built itself without loading (a test, a default); it reads as On
    /// this Mac rather than panicking.
    pub fn active_provider(&self) -> ProviderConfig {
        self.stt_provider.clone().unwrap_or(ProviderConfig::Local {
            model: self.model.clone(),
        })
    }

    /// The action's key, or its default when the file predates the migration.
    pub fn shortcut(&self, action: ShortcutAction) -> String {
        self.shortcuts
            .get(&action)
            .cloned()
            .unwrap_or_else(|| shortcuts::store(action, action.default_keys()).expect("defaults"))
    }

    fn resolve_provider(&mut self) {
        if self.stt_provider.is_none() {
            self.stt_provider = Some(self.active_provider());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Counts emits in place of the app's event loop.
    #[derive(Default)]
    struct SettingsEvents(AtomicUsize);

    impl SettingsEventSink for SettingsEvents {
        fn emit_settings_changed(&self) -> Result<(), String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    /// A settings.json written before the settings-window geometry fields existed.
    /// Failing to deserialize this would make `load()` rename it to `.corrupt` and
    /// silently reset every preference the user had.
    const LEGACY_JSON: &str = r#"{
        "hotkey": "Super+Control",
        "api_url": "http://127.0.0.1:8002/inference",
        "model": "large-v3",
        "language": "auto",
        "window_x": 1800.0,
        "window_y": 36.0,
        "onboarding_window_x": 861.0,
        "onboarding_window_y": 228.0,
        "paste_mode": "active",
        "post_paste_keys": "enter",
        "save_recordings": false,
        "theme": "auto",
        "hallucination_guard": true,
        "hallucination_retries": 2,
        "quality_no_speech_prob": 0.6,
        "quality_avg_logprob": -1.0,
        "quality_compression_ratio": 2.4
    }"#;

    #[test]
    fn deserializes_settings_file_without_window_geometry() {
        let settings: Settings =
            serde_json::from_str(LEGACY_JSON).expect("legacy settings.json must still parse");

        assert_eq!(settings.settings_x, None);
        assert_eq!(settings.settings_y, None);
        assert_eq!(settings.settings_w, None);
        assert_eq!(settings.settings_h, None);

        assert_eq!(settings.model, "large-v3");
        assert_eq!(settings.window_x, Some(1800.0));
    }

    #[test]
    fn round_trips_settings_window_geometry() {
        let settings = Settings {
            settings_x: Some(120.0),
            settings_y: Some(64.0),
            settings_w: Some(960.0),
            settings_h: Some(700.0),
            ..Settings::default()
        };

        let json = serde_json::to_string(&settings).expect("serialize");
        let parsed: Settings = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.settings_x, Some(120.0));
        assert_eq!(parsed.settings_y, Some(64.0));
        assert_eq!(parsed.settings_w, Some(960.0));
        assert_eq!(parsed.settings_h, Some(700.0));
    }

    /// A settings.json from before the explicit provider setting existed.
    fn legacy_json(api_url: &str, llm_key: &str) -> String {
        format!(
            r#"{{
                "hotkey": "Super+Control",
                "api_url": "{api_url}",
                "model": "large-v3",
                "language": "auto",
                "scribe_llm_base_url": "https://api.openai.com/v1",
                "scribe_llm_api_key": "{llm_key}",
                "scribe_llm_model": "gpt-4o-mini"
            }}"#
        )
    }

    fn load_legacy(json: &str, store: &dyn SecretStore) -> (Settings, bool) {
        Settings::load_from(Ok(Some(json)), store).expect("legacy settings must parse")
    }

    #[test]
    fn settings_migrates_default_url_to_local() {
        let store = speaktype_stt::MemoryStore::new();
        let (settings, changed) =
            load_legacy(&legacy_json("http://127.0.0.1:8002/inference", ""), &store);
        assert!(changed);
        assert_eq!(
            settings.active_provider(),
            ProviderConfig::Local {
                model: "large-v3".to_string()
            }
        );
    }

    #[test]
    fn settings_migrates_app_managed_port_to_local() {
        let store = speaktype_stt::MemoryStore::new();
        let (settings, _) =
            load_legacy(&legacy_json("http://127.0.0.1:8004/inference", ""), &store);
        assert!(matches!(
            settings.active_provider(),
            ProviderConfig::Local { .. }
        ));
    }

    #[test]
    fn settings_migrates_custom_url_to_server() {
        let store = speaktype_stt::MemoryStore::new();
        let (settings, changed) = load_legacy(
            &legacy_json("https://whisper.example.com/v1/inference", ""),
            &store,
        );
        assert!(changed);
        assert_eq!(
            settings.active_provider(),
            ProviderConfig::Server {
                base_url: "https://whisper.example.com/v1/inference".to_string()
            }
        );
    }

    #[test]
    fn llm_key_moves_to_keychain_and_blanks_json() {
        let store = speaktype_stt::MemoryStore::new();
        let (settings, _) = load_legacy(
            &legacy_json("http://127.0.0.1:8002/inference", "sk-llm"),
            &store,
        );
        assert_eq!(settings.scribe_llm_api_key, "");
        assert_eq!(store.get("llm.api_key").unwrap().as_deref(), Some("sk-llm"));
    }

    #[test]
    fn settings_defaults_for_new_install() {
        let store = speaktype_stt::MemoryStore::new();
        let (settings, changed) = Settings::load_from(Ok(None), &store).expect("no file");

        assert!(changed);

        assert_eq!(settings.version, 2);
        assert_eq!(settings.post_paste_keys, None);
        assert!(settings.sounds);
        assert_eq!(settings.widget_size, "regular");
        assert_eq!(settings.widget_visibility, "always");
        assert_eq!(settings.ink_color, "orange");
        assert_eq!(settings.ink_thickness, "regular");
        assert_eq!(settings.shortcut(ShortcutAction::DrawKey), "right_option");
        assert!(settings.draw_anytime);
        assert_eq!(settings.history_limit, Some(100));
        assert!(!settings.launch_at_login);
        assert_eq!(settings.onboarding_step, 0);
        assert_eq!(settings.shortcut(ShortcutAction::Session), "Option+Super+K");
        assert!(settings.live_captions);
        assert!(!settings.keep_session_audio);
        assert_eq!(settings.widget_corner, "bottom_right");
    }

    #[test]
    fn settings_reads_the_session_and_widget_fields_from_an_existing_file() {
        let store = speaktype_stt::MemoryStore::new();
        let json = serde_json::to_string_pretty(&Settings {
            shortcuts: BTreeMap::from([(ShortcutAction::Session, "Option+Super+J".to_string())]),
            live_captions: false,
            keep_session_audio: true,
            widget_corner: "custom".to_string(),
            ..Settings::default()
        })
        .expect("serialize");

        let (settings, _) = Settings::load_from(Ok(Some(&json)), &store).expect("parse");

        assert_eq!(settings.shortcut(ShortcutAction::Session), "Option+Super+J");
        assert!(!settings.live_captions);
        assert!(settings.keep_session_audio);
        assert_eq!(settings.widget_corner, "custom");
    }

    #[test]
    fn settings_roundtrip_session_and_widget_fields() {
        let settings = Settings {
            shortcuts: BTreeMap::from([(ShortcutAction::Session, "Option+Super+J".to_string())]),
            live_captions: false,
            keep_session_audio: true,
            widget_corner: "top_right".to_string(),
            ..Settings::default()
        };

        let parsed: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).expect("deserialize");

        assert_eq!(
            parsed.shortcut(ShortcutAction::Session),
            settings.shortcut(ShortcutAction::Session)
        );
        assert!(!parsed.live_captions);
        assert!(parsed.keep_session_audio);
        assert_eq!(parsed.widget_corner, "top_right");
    }

    #[test]
    fn settings_existing_install_keeps_post_paste() {
        let store = speaktype_stt::MemoryStore::new();
        let json = serde_json::to_string_pretty(&Settings {
            post_paste_keys: Some("enter".to_string()),
            ..Settings::default()
        })
        .expect("serialize")
        .replace(",\n  \"version\": 2", "");
        assert!(!json.contains(r#""version""#), "fixture must predate v2");

        let (settings, _) = Settings::load_from(Ok(Some(&json)), &store).expect("parse");

        assert_eq!(settings.post_paste_keys.as_deref(), Some("enter"));
        assert_eq!(settings.version, 2);
    }

    #[test]
    fn settings_roundtrip_new_fields() {
        let settings = Settings {
            version: 2,
            sounds: false,
            widget_size: "large".to_string(),
            widget_visibility: "active".to_string(),
            ink_color: "blue".to_string(),
            ink_thickness: "bold".to_string(),
            shortcuts: BTreeMap::from([(ShortcutAction::DrawKey, "fn".to_string())]),
            draw_anytime: false,
            history_limit: None,
            launch_at_login: true,
            onboarding_step: 3,
            ..Settings::default()
        };

        let parsed: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).expect("deserialize");

        assert_eq!(parsed.version, 2);
        assert!(!parsed.sounds);
        assert_eq!(parsed.widget_size, "large");
        assert_eq!(parsed.widget_visibility, "active");
        assert_eq!(parsed.ink_color, "blue");
        assert_eq!(parsed.ink_thickness, "bold");
        assert_eq!(parsed.shortcut(ShortcutAction::DrawKey), "fn");
        assert!(!parsed.draw_anytime);
        assert_eq!(parsed.history_limit, None);
        assert!(parsed.launch_at_login);
        assert_eq!(parsed.onboarding_step, 3);
    }

    #[test]
    fn a_small_widget_becomes_regular() {
        let store = speaktype_stt::MemoryStore::new();
        let json = serde_json::to_string(&Settings {
            widget_size: "small".to_string(),
            ..Settings::default()
        })
        .expect("serialize");
        let (settings, changed) = Settings::load_from(Ok(Some(&json)), &store).expect("load");
        assert_eq!(settings.widget_size, "regular");
        assert!(changed);
    }

    /// The owner's real file: dictation on F8, and every other shortcut changed
    /// from its default. Migration must keep all four values as they are.
    const OWNER_JSON: &str = r#"{
        "hotkey": "F8",
        "api_url": "http://127.0.0.1:8002/inference",
        "model": "small",
        "language": "auto",
        "scribe_screenshot_hotkey": "Super+Shift+Digit4",
        "session_hotkey": "Option+Super+J",
        "draw_key": "fn"
    }"#;

    fn shortcuts(settings: &Settings) -> Vec<(ShortcutAction, &str)> {
        settings
            .shortcuts
            .iter()
            .map(|(action, keys)| (*action, keys.as_str()))
            .collect()
    }

    #[test]
    fn migration_keeps_existing_values() {
        let store = speaktype_stt::MemoryStore::new();
        let (settings, changed) = load_legacy(OWNER_JSON, &store);
        assert!(changed, "a file without a shortcuts map is migrated");

        assert_eq!(
            shortcuts(&settings),
            vec![
                (ShortcutAction::Dictation, "F8"),
                (ShortcutAction::Session, "Option+Super+J"),
                (ShortcutAction::Screenshot, "Shift+Super+Digit4"),
                (ShortcutAction::DrawKey, "fn"),
            ],
            "the owner's F8 and every other changed value survive"
        );
    }

    #[test]
    fn a_migrated_file_is_written_without_the_old_fields() {
        let store = speaktype_stt::MemoryStore::new();
        let (settings, _) = load_legacy(OWNER_JSON, &store);

        let written = serde_json::to_value(&settings).expect("serialize");

        assert_eq!(written["shortcuts"]["dictation"], "F8");
        for old in [
            "hotkey",
            "scribe_screenshot_hotkey",
            "session_hotkey",
            "draw_key",
        ] {
            assert!(written.get(old).is_none(), "{old} is gone from the struct");
        }
    }

    #[test]
    fn an_unreadable_legacy_shortcut_falls_back_to_that_actions_default() {
        let store = speaktype_stt::MemoryStore::new();
        let json = OWNER_JSON.replace(r#""hotkey": "F8""#, r#""hotkey": "Hyper+F8""#);

        let (settings, _) = load_legacy(&json, &store);

        assert_eq!(
            settings.shortcut(ShortcutAction::Dictation),
            "Super+Control",
            "only the unreadable value resets"
        );
        assert_eq!(settings.shortcut(ShortcutAction::Session), "Option+Super+J");
    }

    #[test]
    fn a_file_that_already_has_shortcuts_is_not_migrated_again() {
        let store = speaktype_stt::MemoryStore::new();
        let json = serde_json::to_string_pretty(&Settings {
            shortcuts: BTreeMap::from([
                (ShortcutAction::Dictation, "F9".to_string()),
                (ShortcutAction::Session, "Option+Super+J".to_string()),
                (ShortcutAction::Screenshot, "Super+Option+S".to_string()),
                (ShortcutAction::DrawKey, "fn".to_string()),
            ]),
            ..Settings::default()
        })
        .expect("serialize");

        let (settings, _) = Settings::load_from(Ok(Some(&json)), &store).expect("parse");

        assert_eq!(settings.shortcuts[&ShortcutAction::Dictation], "F9");
        assert_eq!(settings.shortcuts[&ShortcutAction::DrawKey], "fn");
    }

    #[test]
    fn a_new_install_gets_the_spec_defaults() {
        assert_eq!(
            shortcuts(&Settings::default()),
            vec![
                (ShortcutAction::Dictation, "Super+Control"),
                (ShortcutAction::Session, "Option+Super+K"),
                (ShortcutAction::Screenshot, "Option+Super+S"),
                (ShortcutAction::DrawKey, "right_option"),
            ]
        );
    }

    #[test]
    fn a_missing_action_in_the_map_reads_back_its_default() {
        let settings = Settings {
            shortcuts: BTreeMap::from([(ShortcutAction::Dictation, "F8".to_string())]),
            ..Settings::default()
        };

        assert_eq!(settings.shortcut(ShortcutAction::Dictation), "F8");
        assert_eq!(
            settings.shortcut(ShortcutAction::Screenshot),
            "Option+Super+S"
        );
    }

    #[test]
    fn set_launch_at_login_fails_as_ipc_error_without_an_app_handle() {
        let mut settings = Settings::default();
        let e = set_launch_at_login::<tauri::Wry>(None, true, &mut settings)
            .expect_err("no AppHandle means no plugin");
        assert_eq!(e.kind, "launchAtLogin");
        assert!(
            !settings.launch_at_login,
            "a failed enable must not be recorded as on"
        );
    }

    #[test]
    fn save_settings_emits_settings_changed() {
        let sink = SettingsEvents::default();

        emit_settings_changed(&sink);

        assert_eq!(sink.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn an_explicit_provider_is_kept_as_is() {
        let store = speaktype_stt::MemoryStore::new();
        let explicit = Settings {
            stt_provider: Some(ProviderConfig::Cloud {
                service: speaktype_stt::CloudService::Groq,
                base_url: "https://api.groq.com/openai/v1".to_string(),
                model: "whisper-large-v3".to_string(),
                fallback_to_local: true,
            }),
            ..Settings::default()
        };
        let json = serde_json::to_string(&explicit).expect("serialize");
        let (settings, changed) = load_legacy(&json, &store);
        assert!(!changed);
        assert_eq!(settings.active_provider(), explicit.active_provider());
    }
}
