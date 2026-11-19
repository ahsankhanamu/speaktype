use serde::{Deserialize, Serialize};
use speaktype_stt::{ProviderConfig, SecretStore};
use std::fs;
use std::path::PathBuf;

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
    pub hotkey: String,
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
    /// Scribe: hotkey that triggers a screenshot capture into the active session.
    #[serde(default = "default_scribe_screenshot_hotkey")]
    pub scribe_screenshot_hotkey: String,
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
    /// Scribe: OpenAI-compatible LLM configuration. Empty base_url means the LLM
    /// assembly layer is disabled (MVP runs locally without it).
    #[serde(default = "default_scribe_llm_base_url")]
    pub scribe_llm_base_url: String,
    #[serde(default)]
    pub scribe_llm_api_key: String,
    /// Where transcription comes from. Missing in files written before the
    /// Providers page existed; [`Settings::from_json`] fills it from `api_url`.
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

fn default_scribe_screenshot_hotkey() -> String {
    "Super+Option+S".to_string()
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

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Super+Control".to_string(),
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
            post_paste_keys: Some("enter".to_string()),
            save_recordings: false,
            theme: "auto".to_string(),
            hallucination_guard: default_hallucination_guard(),
            hallucination_retries: default_hallucination_retries(),
            quality_no_speech_prob: default_quality_no_speech_prob(),
            quality_avg_logprob: default_quality_avg_logprob(),
            quality_compression_ratio: default_quality_compression_ratio(),
            input_device: None,
            scribe_screenshot_hotkey: default_scribe_screenshot_hotkey(),
            scribe_enabled: default_scribe_enabled(),
            scribe_clipboard_monitor: default_scribe_clipboard_monitor(),
            scribe_max_screenshots: default_scribe_max_screenshots(),
            scribe_template: default_scribe_template(),
            scribe_llm_base_url: default_scribe_llm_base_url(),
            scribe_llm_api_key: String::new(),
            stt_provider: None,
            scribe_llm_model: default_scribe_llm_model(),
            scribe_llm_use_local: default_scribe_llm_use_local(),
            scribe_llm_local_model: String::new(),
            scribe_llm_local_ctx: default_scribe_llm_local_ctx(),
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
        let (mut settings, mut changed) = match read {
            Some(content) => match Self::from_json(&content, &keychain) {
                Ok(migrated) => migrated,
                Err(e) => {
                    crate::logging::log_message(&format!(
                        "[settings] Corrupt settings file at {:?}, backing up and using defaults: {}",
                        path, e
                    ));
                    let _ = fs::rename(&path, path.with_extension("json.corrupt"));
                    (Self::default(), true)
                }
            },
            None => (Self::default(), false),
        };

        let normalized = Self::normalize_hotkey(&settings.hotkey);
        if normalized != settings.hotkey {
            settings.hotkey = normalized;
            changed = true;
        }
        if changed {
            if let Err(e) = settings.save() {
                crate::logging::log_message(&format!("[settings] Failed to save migration: {}", e));
            }
        }
        settings.resolve_provider();
        settings
    }

    /// Parse a settings file, migrating out-of-date fields. The second value
    /// reports whether `settings` needs writing back.
    pub fn from_json(json: &str, secrets: &dyn SecretStore) -> Result<(Self, bool), String> {
        let mut settings: Settings = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let mut changed = false;

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

    #[test]
    fn settings_migrates_default_url_to_local() {
        let store = speaktype_stt::MemoryStore::new();
        let (settings, changed) =
            Settings::from_json(&legacy_json("http://127.0.0.1:8002/inference", ""), &store)
                .expect("legacy settings must parse");
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
            Settings::from_json(&legacy_json("http://127.0.0.1:8004/inference", ""), &store)
                .expect("legacy settings must parse");
        assert!(matches!(
            settings.active_provider(),
            ProviderConfig::Local { .. }
        ));
    }

    #[test]
    fn settings_migrates_custom_url_to_server() {
        let store = speaktype_stt::MemoryStore::new();
        let (settings, changed) = Settings::from_json(
            &legacy_json("https://whisper.example.com/v1/inference", ""),
            &store,
        )
        .expect("legacy settings must parse");
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
        let (settings, _) = Settings::from_json(
            &legacy_json("http://127.0.0.1:8002/inference", "sk-llm"),
            &store,
        )
        .expect("legacy settings must parse");
        assert_eq!(settings.scribe_llm_api_key, "");
        assert_eq!(store.get("llm.api_key").unwrap().as_deref(), Some("sk-llm"));
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
        let (settings, changed) = Settings::from_json(&json, &store).expect("parse");
        assert!(!changed);
        assert_eq!(settings.active_provider(), explicit.active_provider());
    }
}
