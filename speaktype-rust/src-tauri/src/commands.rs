use crate::audio::{self, AudioRecorder};
use crate::history::History;
use crate::logging::log_message;
use crate::paste::{self, WindowInfo};
use crate::settings::Settings;
use crate::transcribe;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

pub struct AppState {
    pub settings: Arc<Mutex<Settings>>,
    pub recorder: Arc<Mutex<AudioRecorder>>,
    pub is_recording: Arc<AtomicBool>,
    pub target_window: Arc<Mutex<Option<WindowInfo>>>,
}

#[tauri::command]
pub fn toggle_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    is_recording: bool,
) -> Result<(), String> {
    log_message(&format!(
        "[toggle_recording] is_recording={}",
        is_recording
    ));

    if is_recording {
        // Stop recording
        if !state.is_recording.load(Ordering::SeqCst) {
            return Ok(()); // Not recording, nothing to do
        }
        state.is_recording.store(false, Ordering::SeqCst);

        let (samples, sample_rate) = {
            let mut recorder = state.recorder.lock().map_err(|e| e.to_string())?;
            recorder.stop()
        };

        let duration_ms = if sample_rate > 0 {
            (samples.len() as f64 / sample_rate as f64 * 1000.0) as u64
        } else {
            0
        };

        let _ = app.emit(
            "sidecar:recording_stopped",
            json!({"duration_ms": duration_ms}),
        );

        // Get settings and target window for async task
        let settings = {
            let s = state.settings.lock().map_err(|e| e.to_string())?;
            s.clone()
        };
        let target_window = {
            let tw = state.target_window.lock().map_err(|e| e.to_string())?;
            tw.clone()
        };

        // Spawn async task for transcription + paste
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            process_recording(app_clone, samples, sample_rate, settings, target_window).await;
        });
    } else {
        // Start recording
        if state.is_recording.load(Ordering::SeqCst) {
            return Ok(()); // Already recording
        }

        // Capture active window before recording starts
        {
            let mut tw = state.target_window.lock().map_err(|e| e.to_string())?;
            *tw = paste::get_active_window();
        }

        {
            let mut recorder = state.recorder.lock().map_err(|e| e.to_string())?;
            recorder.start(app.clone())?;
        }

        state.is_recording.store(true, Ordering::SeqCst);
        let _ = app.emit("sidecar:recording_started", json!({}));
    }

    Ok(())
}

async fn process_recording(
    app: AppHandle,
    samples: Vec<f32>,
    sample_rate: u32,
    settings: Settings,
    target_window: Option<WindowInfo>,
) {
    let _ = app.emit("sidecar:transcribing", json!({}));

    // Check for speech
    if samples.is_empty() || !audio::has_speech(&samples, sample_rate) {
        log_message("[process] No speech detected");
        let _ = app.emit("sidecar:no_speech", json!({}));
        return;
    }

    // Encode to WAV
    let wav_data = match audio::to_wav(&samples, sample_rate) {
        Ok(data) => data,
        Err(e) => {
            log_message(&format!("[process] WAV encoding error: {}", e));
            let _ = app.emit("sidecar:error", json!({"message": e}));
            return;
        }
    };

    // Transcribe
    let text = match transcribe::transcribe(wav_data, &settings).await {
        Ok(text) => text,
        Err(e) => {
            log_message(&format!("[process] Transcription error: {}", e));
            let _ = app.emit("sidecar:error", json!({"message": e}));
            return;
        }
    };

    // Check hallucination
    if text.is_empty() || transcribe::is_hallucination(&text) {
        log_message(&format!(
            "[process] Empty or hallucination: {:?}",
            text
        ));
        let _ = app.emit("sidecar:no_speech", json!({}));
        return;
    }

    // Paste text (prepend space to separate from previous text)
    let paste_text = format!(" {}", text);
    log_message(&format!("[process] Pasting: {:?}", paste_text));

    // Paste runs on a blocking thread (clipboard + keystroke simulation)
    let target = target_window.clone();
    let paste_result = tokio::task::spawn_blocking(move || {
        paste::paste_text(&paste_text, target.as_ref());
    })
    .await;

    if let Err(e) = paste_result {
        log_message(&format!("[process] Paste task error: {}", e));
        let _ = app.emit("sidecar:error", json!({"message": e.to_string()}));
        return;
    }

    // Save to history
    if let Err(e) = History::add_entry(&text) {
        log_message(&format!("[history] Save error: {}", e));
    }

    let _ = app.emit("sidecar:pasted", json!({"text": text}));
}

#[tauri::command]
pub fn check_permissions() -> Result<serde_json::Value, String> {
    let accessibility = crate::permissions::check_accessibility(false);
    let microphone = crate::permissions::request_microphone();
    Ok(json!({
        "accessibility": accessibility,
        "microphone": microphone,
    }))
}

#[tauri::command]
pub fn request_accessibility() -> Result<bool, String> {
    Ok(crate::permissions::check_accessibility(true))
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    settings.save()?;
    let mut current = state.settings.lock().map_err(|e| e.to_string())?;
    *current = settings;
    Ok(())
}

#[tauri::command]
pub fn update_hotkey(state: State<'_, AppState>, hotkey: String) -> Result<(), String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.hotkey = hotkey;
    settings.save()
}

#[tauri::command]
pub async fn check_server(api_url: String) -> Result<serde_json::Value, String> {
    let health_url = if api_url.ends_with("/transcribe") {
        api_url.replace("/transcribe", "/health")
    } else {
        format!("{}/health", api_url.trim_end_matches('/'))
    };

    let client = reqwest::Client::new();
    match client
        .get(&health_url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => Ok(json!({"status": "connected", "info": body})),
                    Err(_) => Ok(json!({"status": "connected", "info": null})),
                }
            } else {
                Ok(json!({"status": "error", "message": format!("HTTP {}", resp.status())}))
            }
        }
        Err(e) => Ok(json!({"status": "disconnected", "message": e.to_string()})),
    }
}

#[tauri::command]
pub fn get_history() -> Result<History, String> {
    Ok(History::load())
}

#[tauri::command]
pub fn delete_history_entry(index: usize) -> Result<(), String> {
    History::delete_entry(index)
}

#[tauri::command]
pub fn clear_history() -> Result<(), String> {
    History::clear()
}

#[tauri::command]
pub async fn open_about(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("about") {
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "about", WebviewUrl::App("about.html".into()))
        .title("About SpeakType")
        .inner_size(320.0, 280.0)
        .resizable(false)
        .center()
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("settings.html".into()))
        .title("SpeakType Settings")
        .inner_size(480.0, 580.0)
        .resizable(false)
        .center()
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn hide_widget(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn minimize_widget(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn save_window_position(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let pos = window.outer_position().map_err(|e| e.to_string())?;
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.window_x = Some(pos.x as f64);
        settings.window_y = Some(pos.y as f64);
        settings.save()?;
    }
    Ok(())
}
