use crate::commands::AppState;
use crate::session_cmds::IpcError;
use crate::settings::LLM_KEY_ACCOUNT;
use crate::transcribe::{self, Clip, TEST_CLIP};
use serde::{Deserialize, Serialize};
use speaktype_stt::{
    CloudService, Decode, KeychainStore, ProviderConfig, RemoteModel, SecretStore,
};
use tauri::{AppHandle, State};
use tokio_util::sync::CancellationToken;

/// What `stt_test` reports back.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SttTestResult {
    pub latency_ms: u32,
    pub text: String,
}

fn stt_error(e: speaktype_stt::SecretError) -> IpcError {
    IpcError::new("secrets", e.to_string())
}

/// The Keychain account for an STT service, plus the AI-writing key. Custom is
/// a service like any other from the Keychain's point of view.
fn account(service: &str) -> Result<String, IpcError> {
    let account = match service {
        "openai" => CloudService::OpenAi,
        "groq" => CloudService::Groq,
        "deepgram" => CloudService::Deepgram,
        "custom" => CloudService::Custom,
        "llm" => return Ok(LLM_KEY_ACCOUNT.to_string()),
        other => {
            return Err(IpcError::new(
                "secrets",
                format!("Unknown service \"{other}\"."),
            ))
        }
    };
    Ok(account.secret_account().to_string())
}

fn persist(state: &AppState, cfg: ProviderConfig) -> Result<ProviderConfig, IpcError> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|e| IpcError::new("settings", e.to_string()))?;
    settings.stt_provider = Some(cfg.clone());
    settings.save().map_err(|e| IpcError::new("settings", e))?;
    Ok(cfg)
}

/// Store the provider choice. The key itself never travels with it.
#[tauri::command]
#[specta::specta]
pub fn stt_set_provider(
    state: State<'_, AppState>,
    cfg: ProviderConfig,
) -> Result<ProviderConfig, IpcError> {
    persist(&state, cfg)
}

/// Keychain only: the key is never written to settings or returned.
#[tauri::command]
#[specta::specta]
pub fn stt_set_api_key(service: String, key: String) -> Result<(), IpcError> {
    KeychainStore
        .set(&account(&service)?, &key)
        .map_err(stt_error)
}

#[tauri::command]
#[specta::specta]
pub fn stt_has_api_key(service: String) -> Result<bool, IpcError> {
    let stored = KeychainStore.get(&account(&service)?).map_err(stt_error)?;
    Ok(stored.is_some_and(|key| !key.is_empty()))
}

/// Reads the Keychain after the user authenticates (spec §6.7: the eye toggle
/// shows the key, so it is gated behind Touch ID or the login password as well
/// as the Keychain's own ACL).
#[tauri::command]
#[specta::specta]
// async + spawn_blocking: a sync command runs on the main thread, where the
// Touch ID prompt must also run, so waiting for it there would deadlock.
pub async fn stt_reveal_api_key(app: AppHandle, service: String) -> Result<String, IpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        confirm_owner(&app)?;
        Ok(KeychainStore
            .get(&account(&service)?)
            .map_err(stt_error)?
            .unwrap_or_default())
    })
    .await
    .map_err(|e| IpcError::new("authCancelled", e.to_string()))?
}

/// Ask macOS to confirm the person in front of the screen is the account
/// owner. The prompt is UI, so evaluation starts on the main thread; the
/// reply block sends its verdict back through a channel.
#[cfg(target_os = "macos")]
fn confirm_owner(app: &AppHandle) -> Result<(), IpcError> {
    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSError, NSString};
    use objc2_local_authentication::{LAContext, LAPolicy};
    use std::sync::mpsc;
    use std::time::Duration;

    // Long enough for a careful Touch ID / password entry; a main thread that
    // never gets to the prompt must fail visibly instead of hanging the call.
    const PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    app.run_on_main_thread(move || {
        let context = unsafe { LAContext::new() };
        let policy = LAPolicy::DeviceOwnerAuthentication;
        if let Err(e) = unsafe { context.canEvaluatePolicy_error(policy) } {
            let _ = tx.send(Err(e.localizedDescription().to_string()));
            return;
        }
        let reason = NSString::from_str("Show your API key");
        let reply = RcBlock::new(move |success: Bool, error: *mut NSError| {
            let verdict = match (success == Bool::YES, unsafe { error.as_ref() }) {
                (true, _) => Ok(()),
                (false, Some(e)) => Err(e.localizedDescription().to_string()),
                (false, None) => Err("Authentication was cancelled.".to_string()),
            };
            let _ = tx.send(verdict);
        });
        unsafe {
            context.evaluatePolicy_localizedReason_reply(policy, &reason, &reply);
        }
    })
    .map_err(|e| IpcError::new("authCancelled", e.to_string()))?;

    rx.recv_timeout(PROMPT_TIMEOUT)
        .map_err(|_| IpcError::new("authCancelled", "Authentication was cancelled."))?
        .map_err(|message| IpcError::new("authCancelled", message))
}

#[cfg(not(target_os = "macos"))]
fn confirm_owner(_app: &AppHandle) -> Result<(), IpcError> {
    Err(IpcError::new(
        "authCancelled",
        "Showing the key needs macOS.",
    ))
}

/// Models the configured provider offers. Local returns the one model the
/// server runs; a user's own server has no listing API, so an empty list is
/// not "nothing configured" (F4 report, open question 3).
#[tauri::command]
#[specta::specta]
pub async fn stt_list_models(state: State<'_, AppState>) -> Result<Vec<RemoteModel>, IpcError> {
    let (cfg, api_url) = {
        let settings = state
            .settings
            .lock()
            .map_err(|e| IpcError::new("settings", e.to_string()))?;
        (settings.active_provider(), settings.api_url.clone())
    };
    let provider = transcribe::provider(&cfg, &api_url, &KeychainStore)
        .map_err(|e| IpcError::new("stt", e.user_message(transcribe::service_name(&cfg))))?;
    provider
        .list_models(CancellationToken::new())
        .await
        .map_err(|e| IpcError::new("stt", e.user_message(transcribe::service_name(&cfg))))
}

/// Send the bundled 3 s clip through the provider and report latency and text.
#[tauri::command]
#[specta::specta]
pub async fn stt_test(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SttTestResult, IpcError> {
    let (cfg, api_url) = {
        let settings = state
            .settings
            .lock()
            .map_err(|e| IpcError::new("settings", e.to_string()))?;
        (settings.active_provider(), settings.api_url.clone())
    };
    let clip = Clip {
        wav: TEST_CLIP,
        duration_ms: clip_duration_ms(TEST_CLIP),
        language: None,
        prompt: None,
        decode: Decode::default(),
    };
    let transcript = transcribe::run_provider(&cfg, &api_url, &KeychainStore, &clip, || {
        transcribe::emit_fallback(&app, &cfg)
    })
    .await
    .map_err(|e| IpcError::new("stt", e))?;
    Ok(SttTestResult {
        latency_ms: transcript.latency_ms,
        text: transcript.text,
    })
}

/// A 16 kHz mono s16 WAV's audio length, read from the file itself so the clip
/// can be replaced without touching this code.
fn clip_duration_ms(wav: &[u8]) -> u32 {
    if wav.len() <= 44 {
        return 0;
    }
    (wav.len() as u64 - 44)
        .saturating_mul(1000)
        .div_ceil(32_000) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::LLM_KEY_ACCOUNT;

    #[test]
    fn accounts_cover_each_service_and_the_llm_key() {
        assert_eq!(account("openai").unwrap(), "stt.openai");
        assert_eq!(account("groq").unwrap(), "stt.groq");
        assert_eq!(account("deepgram").unwrap(), "stt.deepgram");
        assert_eq!(account("custom").unwrap(), "stt.custom");
        assert_eq!(account("llm").unwrap(), LLM_KEY_ACCOUNT);
        assert_eq!(account("nope").unwrap_err().kind, "secrets");
    }

    #[test]
    fn test_clip_is_three_seconds_of_16khz_mono() {
        let wav = TEST_CLIP;
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        let channels = u16::from_le_bytes([wav[22], wav[23]]);
        let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
        let bits = u16::from_le_bytes([wav[34], wav[35]]);
        assert_eq!((channels, rate, bits), (1, 16_000, 16));
        let ms = clip_duration_ms(wav);
        assert!((2_500..=3_500).contains(&ms), "{ms} ms");
    }
}
