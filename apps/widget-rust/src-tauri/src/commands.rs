use crate::audio::{self, AudioRecorder};
use crate::chunk_session::{ChunkSession, transcribe_chunked};
use crate::download_queue;
use crate::downloader;
use crate::history::History;
use crate::logging::log_message;
use crate::paste::{self, WindowInfo};
use crate::settings::Settings;
use crate::tones::{self, Tone};
use crate::transcribe;
use crate::tray::{self, TrayState};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

static MODEL_OPERATION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

fn emit_model_progress(app: &AppHandle, model: &str, phase: &str, message: &str) {
    let _ = app.emit(
        "model:progress",
        json!({
            "model": model,
            "phase": phase,
            "message": message,
        }),
    );
}

pub struct AppState {
    pub settings: Arc<Mutex<Settings>>,
    pub recorder: Arc<Mutex<AudioRecorder>>,
    pub is_recording: Arc<AtomicBool>,
    pub is_transcribing: Arc<AtomicBool>,
    pub target_window: Arc<Mutex<Option<WindowInfo>>>,
    pub last_active_window: Arc<Mutex<Option<WindowInfo>>>,
    pub chunk_session: Arc<Mutex<Option<ChunkSession>>>,
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
        if !state.is_recording.load(Ordering::SeqCst) {
            return Ok(());
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

        let duration_secs = duration_ms as f64 / 1000.0;
        log_message(&format!("[toggle_recording] Recording duration: {:.1}s, samples: {}", duration_secs, samples.len()));

        tray::set_tray_state(&app, TrayState::Transcribing);
        tones::play(Tone::RecordingStop);
        let _ = app.emit(
            "sidecar:recording_stopped",
            json!({"duration_ms": duration_ms}),
        );

        let settings = {
            let s = state.settings.lock().map_err(|e| e.to_string())?;
            s.clone()
        };

        let chunk_session = {
            let mut session = state.chunk_session.lock().map_err(|e| e.to_string())?;
            session.take()
        };

        let target_window = if settings.paste_mode == "active" {
            let last = state.last_active_window.lock().map_err(|e| e.to_string())?;
            last.clone()
        } else {
            let tw = state.target_window.lock().map_err(|e| e.to_string())?;
            tw.clone()
        };

        let app_clone = app.clone();
        let is_transcribing = state.is_transcribing.clone();
        tauri::async_runtime::spawn(async move {
            process_recording(
                app_clone,
                samples,
                sample_rate,
                settings,
                target_window,
                is_transcribing,
                chunk_session,
            )
            .await;
        });
    } else {
        if state.is_recording.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Hard gate: refuse to record until microphone, accessibility, and a
        // complete model are all ready. Reopen onboarding on the failing step
        // instead of starting a recording that would fail abruptly.
        let model = {
            let s = state.settings.lock().map_err(|e| e.to_string())?;
            s.model.clone()
        };
        if !requirements_met(&model) {
            log_message("[toggle_recording] Requirements not met — opening onboarding");
            tones::play(Tone::Error);
            let _ = app.emit("sidecar:onboarding_required", json!({}));
            let _ = open_onboarding_window(&app);
            return Ok(());
        }

        if state.is_transcribing.load(Ordering::SeqCst) {
            log_message("[toggle_recording] Currently transcribing, cannot start new recording");
            tones::play(Tone::Error);
            let _ = app.emit("sidecar:busy", json!({"reason": "Currently transcribing"}));
            return Err("Currently transcribing, please wait".to_string());
        }

        {
            let mut tw = state.target_window.lock().map_err(|e| e.to_string())?;
            let last = state.last_active_window.lock().map_err(|e| e.to_string())?;
            *tw = last.clone();
        }

        {
            let settings = {
                let s = state.settings.lock().map_err(|e| e.to_string())?;
                s.clone()
            };
            let mut recorder = state.recorder.lock().map_err(|e| e.to_string())?;
            let buffer = recorder.buffer();
            let sample_rate = recorder.sample_rate();
            recorder.start(app.clone())?;

            let session = ChunkSession::start(settings, sample_rate, buffer);
            let mut chunk_session = state.chunk_session.lock().map_err(|e| e.to_string())?;
            *chunk_session = Some(session);
            log_message("[toggle_recording] Chunk session started");
        }

        state.is_recording.store(true, Ordering::SeqCst);
        tray::set_tray_state(&app, TrayState::Recording);
        tones::play(Tone::RecordingStart);
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
    is_transcribing: Arc<AtomicBool>,
    chunk_session: Option<ChunkSession>,
) {
    if is_transcribing.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        log_message("[process] Already transcribing, skipping");
        return;
    }

    let _guard = scopeguard::guard((), |_| {
        is_transcribing.store(false, Ordering::SeqCst);
    });

    let _ = app.emit("sidecar:transcribing", json!({}));

    let duration_secs = samples.len() as f64 / sample_rate as f64;

    if duration_secs > 120.0 {
        log_message(&format!("[process] Long audio ({:.0}s), using chunked transcription", duration_secs));
        let _ = app.emit("sidecar:long_audio", json!({"duration_secs": duration_secs}));
    }

    if samples.is_empty() || !audio::has_speech(&samples, sample_rate) {
        log_message("[process] No speech detected");
        tones::play(Tone::Error);
        tray::flash_error(&app);
        let _ = app.emit("sidecar:no_speech", json!({}));
        return;
    }

    let text = if let Some(session) = chunk_session {
        match session.finalize(samples.clone(), sample_rate).await {
            Some(t) if !t.is_empty() => t,
            _ => {
                log_message("[process] Chunk session produced no text, falling back");
                match transcribe_chunked(samples, sample_rate, &settings).await {
                    Ok(t) => t,
                    Err(e) => {
                        log_message(&format!("[process] Transcription error: {}", e));
                        tones::play(Tone::Error);
                        tray::flash_error(&app);
                        let _ = app.emit("sidecar:error", json!({"message": e}));
                        return;
                    }
                }
            }
        }
    } else {
        match transcribe_chunked(samples, sample_rate, &settings).await {
            Ok(t) => t,
            Err(e) => {
                log_message(&format!("[process] Transcription error: {}", e));
                tones::play(Tone::Error);
                tray::flash_error(&app);
                let _ = app.emit("sidecar:error", json!({"message": e}));
                return;
            }
        }
    };

    if text.is_empty() || transcribe::is_hallucination(&text) {
        log_message(&format!("[process] Empty or hallucination: {:?}", text));
        tones::play(Tone::Error);
        tray::flash_error(&app);
        let _ = app.emit("sidecar:no_speech", json!({}));
        return;
    }

    let paste_text = format!(" {}", text);
    log_message(&format!("[process] Pasting: {:?}", paste_text));

    let target = target_window.clone();
    let post_keys = settings.post_paste_keys.clone();
    let paste_result = tokio::task::spawn_blocking(move || {
        paste::paste_text(&paste_text, target.as_ref());
        paste::press_post_paste_keys(post_keys.as_deref());
    })
    .await;

    if let Err(e) = paste_result {
        log_message(&format!("[process] Paste task error: {}", e));
        tones::play(Tone::Error);
        tray::flash_error(&app);
        let _ = app.emit("sidecar:error", json!({"message": e.to_string()}));
        return;
    }

    if let Err(e) = History::add_entry(&text) {
        log_message(&format!("[history] Save error: {}", e));
    }

    tray::set_tray_state(&app, TrayState::Idle);
    let _ = app.emit("sidecar:pasted", json!({"text": text}));
}

pub fn emit_permissions(app: &AppHandle) {
    let microphone = crate::permissions::check_microphone();
    let accessibility = crate::permissions::check_accessibility(false);
    let _ = app.emit(
        "sidecar:permissions",
        json!({ "microphone": microphone, "accessibility": accessibility }),
    );
}

#[tauri::command]
pub fn check_permissions() -> Result<serde_json::Value, String> {
    let accessibility = crate::permissions::check_accessibility(false);
    let microphone = crate::permissions::check_microphone();
    Ok(json!({
        "accessibility": accessibility,
        "microphone": microphone,
    }))
}

/// Whether all three launch requirements (mic, accessibility, a complete model)
/// are satisfied for the given model.
pub fn requirements_met(model: &str) -> bool {
    crate::permissions::check_microphone()
        && crate::permissions::check_accessibility(false)
        && crate::downloader::is_model_complete(model)
}

/// Onboarding status used by the onboarding window to drive its serial steps.
#[tauri::command]
pub fn get_onboarding_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let model = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.model.clone()
    };
    Ok(json!({
        "microphone": crate::permissions::check_microphone(),
        "accessibility": crate::permissions::check_accessibility(false),
        "model": crate::downloader::is_model_complete(&model),
        "active_model": model,
    }))
}

/// Trigger the macOS microphone permission prompt (step 1 of onboarding).
#[tauri::command]
pub fn request_microphone_access(app: AppHandle) -> Result<bool, String> {
    let granted = crate::permissions::request_microphone();
    emit_permissions(&app);
    Ok(granted)
}

/// Open (or focus) the onboarding window.
#[tauri::command]
pub async fn open_onboarding(app: AppHandle) -> Result<(), String> {
    open_onboarding_window(&app)
}

pub fn open_onboarding_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("onboarding") {
        let _ = window.show();
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, "onboarding", WebviewUrl::App("onboarding.html".into()))
        .title("Welcome to SpeakType")
        .inner_size(440.0, 600.0)
        .resizable(false)
        .center()
        .always_on_top(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Called when the user finishes onboarding: ensure the server is running for the
/// selected model, then close the onboarding window and reveal the widget.
#[tauri::command]
pub async fn finish_onboarding(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let model = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.model.clone()
    };

    if crate::downloader::is_model_complete(&model)
        && crate::server::current_port().is_none()
    {
        if let Some(info) = crate::server::start_server(&app, &model) {
            let port = info.port;
            crate::server::wait_for_server(port).await;
            if let Ok(mut s) = state.settings.lock() {
                s.api_url = format!("http://127.0.0.1:{}/inference", port);
                let _ = s.save();
            }
        }
    }

    if let Some(window) = app.get_webview_window("onboarding") {
        let _ = window.close();
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    emit_permissions(&app);
    Ok(())
}

#[tauri::command]
pub fn request_accessibility(app: AppHandle) -> Result<bool, String> {
    let granted = crate::permissions::check_accessibility(true);
    emit_permissions(&app);
    Ok(granted)
}

#[tauri::command]
#[cfg(target_os = "macos")]
pub fn open_system_pane(pane: String) -> Result<(), String> {
    let url = format!("x-apple.systempreferences:{}", pane);
    std::process::Command::new("open")
        .arg(&url)
        .output()
        .map_err(|e| format!("Failed to open system pane: {}", e))?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn open_system_pane(_pane: String) -> Result<(), String> {
    Err("System preferences not supported on this platform".to_string())
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

async fn restart_server_with_model(
    app: &AppHandle,
    settings: &Arc<Mutex<Settings>>,
    model: &str,
) -> Result<serde_json::Value, String> {
    emit_model_progress(app, model, "stopping", "Stopping current server...");
    crate::server::stop_server();

    if let Ok(mut s) = settings.lock() {
        s.model = model.to_string();
        if let Err(e) = s.save() {
            log_message(&format!("[load_model] Failed to save model setting: {}", e));
        }
    }

    emit_model_progress(app, model, "starting", "Starting whisper server...");
    if let Some(info) = crate::server::start_server(app, model) {
        let port = info.port;
        emit_model_progress(
            app,
            model,
            "loading",
            "Loading model into memory (this can take a minute)...",
        );
        if crate::server::wait_for_server(port).await {
            log_message(&format!("[load_model] Server restarted on port {}", port));
            if let Ok(mut s) = settings.lock() {
                s.api_url = format!("http://127.0.0.1:{}/inference", port);
                if let Err(e) = s.save() {
                    log_message(&format!("[load_model] Failed to save API URL setting: {}", e));
                }
            }
            emit_model_progress(app, model, "done", &format!("{} model ready", model));
            Ok(json!({"status": "loaded", "model": model}))
        } else {
            crate::server::stop_server();
            let msg = "Server failed to start — the model file may be corrupt. Try Restart Download.";
            emit_model_progress(app, model, "error", msg);
            Ok(json!({"status": "error", "message": msg}))
        }
    } else {
        let msg = "Failed to start server — check that the model is fully downloaded.";
        emit_model_progress(app, model, "error", msg);
        Ok(json!({"status": "error", "message": msg}))
    }
}

fn start_server_model_operation(
    app: AppHandle,
    settings: Arc<Mutex<Settings>>,
    model: String,
) -> Result<serde_json::Value, String> {
    if MODEL_OPERATION_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        return Ok(json!({
            "status": "busy",
            "message": "Another model operation is already in progress",
        }));
    }

    emit_model_progress(&app, &model, "preparing", &format!("Preparing {}...", model));

    let model_for_task = model.clone();
    tauri::async_runtime::spawn(async move {
        let _ = restart_server_with_model(&app, &settings, &model_for_task).await;
        MODEL_OPERATION_IN_PROGRESS.store(false, Ordering::SeqCst);
    });

    Ok(json!({"status": "started", "model": model}))
}

#[tauri::command]
pub async fn load_model(app: AppHandle, state: State<'_, AppState>, model: String) -> Result<serde_json::Value, String> {
    if downloader::is_model_complete(&model) {
        return start_server_model_operation(app, state.settings.clone(), model);
    }
    download_queue::enqueue(app, state.settings.clone(), model, false, true)
}

#[tauri::command]
pub async fn queue_model_download(
    app: AppHandle,
    state: State<'_, AppState>,
    model: String,
    restart: Option<bool>,
) -> Result<serde_json::Value, String> {
    let restart = restart.unwrap_or(false);
    if restart {
        downloader::clear_model_files(&model)?;
    }
    download_queue::enqueue(app, state.settings.clone(), model, restart, false)
}

#[tauri::command]
pub async fn restart_model_download(
    app: AppHandle,
    state: State<'_, AppState>,
    model: String,
) -> Result<serde_json::Value, String> {
    downloader::clear_model_files(&model)?;
    download_queue::enqueue(app, state.settings.clone(), model, true, true)
}

#[tauri::command]
pub async fn cancel_model_download(app: AppHandle, model: String) -> Result<serde_json::Value, String> {
    let cancelled = download_queue::cancel_waiting_with_app(&app, &model);
    Ok(json!({"cancelled": cancelled, "model": model}))
}

#[tauri::command]
pub async fn pause_model_download(app: AppHandle, model: String) -> Result<serde_json::Value, String> {
    let paused = download_queue::pause_with_app(&app, &model)
        || (downloader::is_download_active(&model) && downloader::pause_download(&model));
    Ok(json!({"paused": paused, "model": model}))
}

#[tauri::command]
pub async fn get_models() -> Result<serde_json::Value, String> {
    let mut available: Vec<serde_json::Value> = vec![
        json!({"id": "tiny.en", "size": "~75MB", "desc": "Tiny (English only)", "speed": "fastest"}),
        json!({"id": "tiny", "size": "~75MB", "desc": "Tiny (multilingual)", "speed": "fastest"}),
        json!({"id": "base.en", "size": "~150MB", "desc": "Base (English only)", "speed": "fast"}),
        json!({"id": "base", "size": "~150MB", "desc": "Base (multilingual)", "speed": "fast"}),
        json!({"id": "small.en", "size": "~500MB", "desc": "Small (English only) — recommended", "speed": "medium"}),
        json!({"id": "small", "size": "~500MB", "desc": "Small (multilingual)", "speed": "medium"}),
        json!({"id": "medium.en", "size": "~1.5GB", "desc": "Medium (English only) — best for 16/24GB Mac", "speed": "slow"}),
        json!({"id": "medium", "size": "~1.5GB", "desc": "Medium (multilingual)", "speed": "slow"}),
        json!({"id": "large-v3", "size": "~3GB", "desc": "Large v3 (most accurate)", "speed": "slowest"}),
        json!({"id": "large-v3-turbo", "size": "~1.5GB", "desc": "Large v3 Turbo (fast + accurate)", "speed": "medium"}),
    ];

    for model in available.iter_mut() {
        let id = model["id"].as_str().unwrap_or("");
        if crate::downloader::is_model_complete(id) {
            let size = crate::downloader::get_model_size(id);
            let size_mb = size as f64 / (1024.0 * 1024.0);
            model["downloaded"] = json!(true);
            model["downloaded_size"] = json!(format!("{:.1}MB", size_mb));
        } else {
            // Incomplete: either a leftover `.part` or a truncated `.bin`.
            let part_bytes = crate::downloader::get_partial_download_size(id);
            let bin_bytes = crate::downloader::get_model_size(id);
            let have = part_bytes.max(bin_bytes);
            if have > 0 {
                let expected = crate::downloader::expected_size(id);
                let have_mb = have as f64 / (1024.0 * 1024.0);
                let partial_label = if expected > 0 {
                    format!("Incomplete — {:.0}%", (have as f64 / expected as f64) * 100.0)
                } else {
                    format!("Incomplete ({:.1}MB)", have_mb)
                };
                model["downloaded"] = json!(false);
                model["partial"] = json!(true);
                model["partial_size"] = json!(have);
                model["expected_size"] = json!(expected);
                model["partial_label"] = json!(partial_label);
            } else {
                model["downloaded"] = json!(false);
            }
        }
    }

    let (mut downloading, waiting) = download_queue::queue_status();
    for model in downloader::get_downloading_models() {
        if !downloading.contains(&model) {
            downloading.push(model);
        }
    }

    let live = downloader::get_live_download_progress();
    let progress: Vec<serde_json::Value> = live
        .iter()
        .map(|(model, (bytes, total))| {
            let pct = if *total > 0 {
                (*bytes as f64 / *total as f64 * 100.0).round()
            } else {
                0.0
            };
            json!({
                "model": model,
                "bytes_downloaded": bytes,
                "total_bytes": total,
                "percent": pct,
                "phase": "downloading",
            })
        })
        .collect();

    Ok(json!({
        "available": available,
        "downloading": downloading,
        "waiting": waiting,
        "progress": progress,
    }))
}

#[tauri::command]
pub async fn get_model_progress() -> Result<serde_json::Value, String> {
    let (mut downloading, waiting) = download_queue::queue_status();
    for model in downloader::get_downloading_models() {
        if !downloading.contains(&model) {
            downloading.push(model);
        }
    }
    let live = downloader::get_live_download_progress();
    let progress: Vec<serde_json::Value> = live
        .iter()
        .map(|(model, (bytes, total))| {
            let pct = if *total > 0 {
                (*bytes as f64 / *total as f64 * 100.0).round()
            } else {
                0.0
            };
            json!({
                "model": model,
                "bytes_downloaded": bytes,
                "total_bytes": total,
                "percent": pct,
                "phase": "downloading",
            })
        })
        .collect();
    Ok(json!({"downloading": downloading, "waiting": waiting, "progress": progress}))
}

#[tauri::command]
pub async fn check_server(api_url: String) -> Result<serde_json::Value, String> {
    // Derive health endpoint: strip the last path segment and append /health
    let base = api_url.trim_end_matches('/');
    let health_url = if let Some(pos) = base.rfind('/') {
        format!("{}/health", &base[..pos])
    } else {
        format!("{}/health", base)
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
        .inner_size(320.0, 380.0)
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
        .inner_size(520.0, 640.0)
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
        window.minimize().map_err(|e| e.to_string())?;
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
        let (cx, cy) = crate::window::clamp_to_visible_screens(
            &window,
            pos.x as f64,
            pos.y as f64,
        );
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.window_x = Some(cx);
        settings.window_y = Some(cy);
        settings.save()?;
    }
    Ok(())
}
