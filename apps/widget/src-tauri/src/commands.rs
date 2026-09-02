use crate::audio::{self, AudioRecorder, LevelMonitor};
use crate::chunk_session::{ChunkSession, transcribe_chunked};
use crate::download_queue;
use crate::downloader;
use crate::history::{self, History};
use crate::format::{catalog_model_size_bytes, format_byte_size, format_byte_size_approx};
use crate::logging::log_message;
use crate::paste::{self, WindowInfo};
use crate::settings::Settings;
use crate::tones::{self, Tone};
use crate::tray::{self, TrayState};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

static MODEL_OPERATION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static RECORDING_START_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

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
    pub level_monitor: Arc<Mutex<LevelMonitor>>,
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
    #[allow(unused_variables)] is_recording: bool,
) -> Result<(), String> {
    toggle_recording_impl(&app, &state)
}

pub fn toggle_recording_impl(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let currently_recording = state.is_recording.load(Ordering::SeqCst);
    log_message(&format!(
        "[toggle_recording] currently_recording={}",
        currently_recording
    ));

    if currently_recording {
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

        if crate::server::has_embedded_server(app) && !crate::server::is_running() {
            if RECORDING_START_IN_FLIGHT
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                log_message("[toggle_recording] Model load already in progress");
                return Ok(());
            }

            let app = app.clone();
            let model = model.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let result = async {
                    match crate::server::ensure_running(&app, &model).await {
                        Ok(Some(info)) => {
                            let api_url = format!("http://127.0.0.1:{}/inference", info.port);
                            if let Ok(mut s) = state.settings.lock() {
                                s.api_url = api_url;
                                s.model = info.model.clone();
                                if let Err(e) = s.save() {
                                    log_message(&format!(
                                        "[toggle_recording] Failed to save settings after load: {}",
                                        e
                                    ));
                                }
                            }
                            begin_recording(&app, &state)
                        }
                        Ok(None) => begin_recording(&app, &state),
                        Err(e) => Err(e),
                    }
                }
                .await;

                RECORDING_START_IN_FLIGHT.store(false, Ordering::SeqCst);

                if let Err(e) = result {
                    log_message(&format!("[toggle_recording] Model load failed: {}", e));
                    tones::play(Tone::Error);
                    let _ = app.emit("server:error", json!({"reason": e}));
                }
            });
            return Ok(());
        }

        begin_recording(app, state)?;
    }

    Ok(())
}

#[tauri::command]
pub fn cancel_recording(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if !state.is_recording.load(Ordering::SeqCst) {
        return Ok(());
    }

    log_message("[cancel_recording] Cancelling active recording");
    state.is_recording.store(false, Ordering::SeqCst);

    {
        let mut recorder = state.recorder.lock().map_err(|e| e.to_string())?;
        let _ = recorder.stop();
    }

    {
        let mut session = state.chunk_session.lock().map_err(|e| e.to_string())?;
        if let Some(active) = session.take() {
            active.abort();
        }
    }

    crate::debug::discard_session();

    tray::set_tray_state(&app, TrayState::Idle);
    tones::play(Tone::RecordingStop);
    let _ = app.emit("sidecar:recording_cancelled", json!({}));
    Ok(())
}

/// Upper bound on a single microphone check. The frontend releases the monitor
/// on its own, but a webview that is torn down mid-test cannot, and a stuck-open
/// input stream keeps the macOS microphone indicator lit.
pub const MIC_TEST_MAX_MS: u64 = 60_000;

fn stop_mic_test_internal(app: &AppHandle, state: &AppState, reason: &str) {
    let stopped = match state.level_monitor.lock() {
        Ok(mut monitor) => monitor.stop(),
        Err(_) => false,
    };
    if stopped {
        let _ = app.emit("mictest:stopped", json!({ "reason": reason }));
    }
}

/// Release the microphone monitor from a context that only has an `AppHandle`
/// (window teardown, process exit).
pub fn stop_mic_test_for_app(app: &AppHandle, reason: &str) {
    if let Some(state) = app.try_state::<AppState>() {
        stop_mic_test_internal(app, &state, reason);
    }
}

fn spawn_mic_test_watchdog(app: AppHandle, generation: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(MIC_TEST_MAX_MS));
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        // A newer generation means this test was already stopped and another
        // started; only the session this watchdog was armed for may be killed.
        let expired = match state.level_monitor.lock() {
            Ok(monitor) => monitor.is_active() && monitor.generation() == generation,
            Err(_) => false,
        };
        if expired {
            log_message("[mictest] Time limit reached — releasing microphone");
            stop_mic_test_internal(&app, &state, "timeout");
        }
    });
}

#[tauri::command]
pub fn start_mic_test(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if !crate::permissions::check_microphone() {
        return Err("Microphone permission is not granted".to_string());
    }
    if state.is_recording.load(Ordering::SeqCst) {
        return Err("Dictation is recording — stop it before testing".to_string());
    }

    let info = {
        let mut monitor = state.level_monitor.lock().map_err(|e| e.to_string())?;
        monitor.start(app.clone())?
    };
    spawn_mic_test_watchdog(app.clone(), info.generation);

    Ok(json!({
        "device": info.device,
        "sample_rate": info.sample_rate,
        "channels": info.channels,
        "inputs": audio::count_input_devices(),
        "max_ms": MIC_TEST_MAX_MS,
    }))
}

#[tauri::command]
pub fn stop_mic_test(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    stop_mic_test_internal(&app, &state, "user");
    Ok(())
}

#[tauri::command]
pub fn get_audio_input_info() -> Result<serde_json::Value, String> {
    Ok(json!({
        "device": audio::get_default_input_device_name(),
        "inputs": audio::count_input_devices(),
    }))
}

/// Forward webview console errors/warnings into the app log so that a frozen
/// settings window or widget leaves a trace on the next occurrence.
#[tauri::command]
pub fn log_frontend(level: String, message: String) {
    crate::logging::log_message(&format!("[webview] {} {}", level, message));
}

fn begin_recording(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if state.is_recording.load(Ordering::SeqCst) {
        return Ok(());
    }

    // Dictation owns the input device. A settings-window meter must never be
    // able to block or race the hotkey, so it is dropped rather than consulted.
    stop_mic_test_internal(app, state, "recording");

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
        crate::debug::start_session(&settings.model);
        let mut recorder = state.recorder.lock().map_err(|e| e.to_string())?;
        let buffer = recorder.buffer();
        recorder.start(app.clone())?;
        let sample_rate = recorder.sample_rate();

        let session = ChunkSession::start(settings, sample_rate, buffer);
        let mut chunk_session = state.chunk_session.lock().map_err(|e| e.to_string())?;
        if let Some(old) = chunk_session.take() {
            log_message("[toggle_recording] Aborting stale chunk session before new recording");
            old.abort();
        }
        *chunk_session = Some(session);
        log_message("[toggle_recording] Chunk session started");
    }

    state.is_recording.store(true, Ordering::SeqCst);
    tray::set_tray_state(app, TrayState::Recording);
    tones::play(Tone::RecordingStart);
    let _ = app.emit("sidecar:recording_started", json!({}));
    Ok(())
}

async fn ensure_settings_api_ready(app: &AppHandle, settings: &mut Settings) -> Result<(), String> {
    let Some(info) = crate::server::ensure_running(app, &settings.model).await? else {
        return Ok(());
    };
    settings.api_url = format!("http://127.0.0.1:{}/inference", info.port);
    settings.model = info.model.clone();
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut s) = state.settings.lock() {
            s.api_url = settings.api_url.clone();
            s.model = settings.model.clone();
            let _ = s.save();
        }
    }
    Ok(())
}

async fn process_recording(
    app: AppHandle,
    samples: Vec<f32>,
    sample_rate: u32,
    mut settings: Settings,
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

    if let Err(e) = ensure_settings_api_ready(&app, &mut settings).await {
        log_message(&format!("[process] Failed to load model for transcription: {}", e));
        tones::play(Tone::Error);
        tray::flash_error(&app);
        crate::debug::discard_session();
        let _ = app.emit("sidecar:error", json!({"reason": e}));
        return;
    }

    let duration_secs = samples.len() as f64 / sample_rate as f64;

    if duration_secs > 120.0 {
        log_message(&format!("[process] Long audio ({:.0}s), using chunked transcription", duration_secs));
        let _ = app.emit("sidecar:long_audio", json!({"duration_secs": duration_secs}));
    }

    if samples.is_empty() || !audio::has_speech(&samples, sample_rate) {
        log_message("[process] No speech detected");
        tones::play(Tone::Error);
        tray::flash_error(&app);
        crate::debug::discard_session();
        let _ = app.emit("sidecar:no_speech", json!({}));
        return;
    }

    let entry_id = history::new_entry_id();
    let saved_recording = if settings.save_recordings {
        match History::save_recording(&entry_id, &samples, sample_rate) {
            Ok((path, duration)) => Some((path, duration)),
            Err(e) => {
                log_message(&format!("[history] Failed to save recording: {}", e));
                None
            }
        }
    } else {
        None
    };

    let text = if let Some(session) = chunk_session {
        match session.finalize(samples.clone(), sample_rate).await {
            Some(t) if !t.is_empty() => t,
            _ => {
                log_message("[process] Chunk session produced no text, falling back");
                crate::debug::capture_audio(&samples, sample_rate);
                match transcribe_chunked(samples, sample_rate, &settings).await {
                    Ok(t) => {
                        crate::debug::update_meta(duration_secs, "post_hoc_fallback");
                        crate::debug::finish_session(&t);
                        t
                    }
                    Err(e) => {
                        History::discard_saved_recording(&saved_recording);
                        crate::debug::discard_session();
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
        crate::debug::capture_audio(&samples, sample_rate);
        match transcribe_chunked(samples, sample_rate, &settings).await {
            Ok(t) => {
                crate::debug::update_meta(duration_secs, "chunked");
                crate::debug::finish_session(&t);
                t
            }
            Err(e) => {
                History::discard_saved_recording(&saved_recording);
                crate::debug::discard_session();
                log_message(&format!("[process] Transcription error: {}", e));
                tones::play(Tone::Error);
                tray::flash_error(&app);
                let _ = app.emit("sidecar:error", json!({"message": e}));
                return;
            }
        }
    };

    if text.is_empty() {
        History::discard_saved_recording(&saved_recording);
        log_message("[process] No usable text left after hallucination scoring");
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
        History::discard_saved_recording(&saved_recording);
        log_message(&format!("[process] Paste task error: {}", e));
        tones::play(Tone::Error);
        tray::flash_error(&app);
        let _ = app.emit("sidecar:error", json!({"message": e.to_string()}));
        return;
    }

    if let Err(e) = History::add_entry(
        &entry_id,
        &text,
        saved_recording.as_ref().map(|(path, _)| path.clone()),
        saved_recording.as_ref().map(|_| sample_rate),
        saved_recording.map(|(_, duration)| duration),
    ) {
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
        "exe_path": crate::permissions::accessibility_exe_path(),
        "is_dev": crate::permissions::is_dev_binary(),
        "is_bundled": crate::permissions::is_bundled(),
        "responsible_app": crate::permissions::responsible_app_name(),
        "responsible_app_path": crate::permissions::responsible_app_path(),
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

pub fn persist_onboarding_window_position(window: &WebviewWindow) {
    let app = window.app_handle();
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Ok(pos) = window.outer_position() else {
        return;
    };
    let (cx, cy) = crate::window::clamp_to_visible_screens(window, pos.x as f64, pos.y as f64);
    let save_result = {
        let Ok(mut settings) = state.settings.lock() else {
            return;
        };
        settings.onboarding_window_x = Some(cx);
        settings.onboarding_window_y = Some(cy);
        settings.save()
    };
    let _ = save_result;
}

pub fn open_onboarding_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("onboarding") {
        let _ = window.show();
        window.set_focus().map_err(|e| e.to_string())?;
        position_widget_for_onboarding(app);
        let _ = app.emit("onboarding:opened", json!({}));
        return Ok(());
    }

    let (saved_x, saved_y) = onboarding_window_position(app);

    let window = WebviewWindowBuilder::new(app, "onboarding", WebviewUrl::App("onboarding.html".into()))
        .title("Welcome to SpeakType")
        .inner_size(440.0, 600.0)
        .resizable(false)
        .always_on_top(true)
        .build()
        .map_err(|e| e.to_string())?;

    if let (Some(x), Some(y)) = (saved_x, saved_y) {
        crate::window::apply_saved_position(&window, x, y);
    } else {
        let _ = window.center();
    }

    position_widget_for_onboarding(app);
    let _ = app.emit("onboarding:opened", json!({}));
    Ok(())
}

fn onboarding_window_position(app: &AppHandle) -> (Option<f64>, Option<f64>) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(settings) = state.settings.lock() {
            return (settings.onboarding_window_x, settings.onboarding_window_y);
        }
    }
    let settings = Settings::load();
    (settings.onboarding_window_x, settings.onboarding_window_y)
}

fn position_widget_for_onboarding(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_always_on_top(true);
        let _ = crate::window::place_at_default_position(&main);
    }
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
        persist_onboarding_window_position(&window);
        let _ = window.close();
    }
    let hotkey = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.hotkey.clone()
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app.emit("onboarding:finished", json!({ "hotkey": hotkey }));
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
pub fn open_models_folder(app: AppHandle) -> Result<(), String> {
    let dir = crate::paths::models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create models directory: {}", e))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Failed to open models folder: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn open_recordings_folder(app: AppHandle) -> Result<(), String> {
    let dir = History::recordings_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create recordings directory: {}", e))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Failed to open recordings folder: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut settings: Settings,
) -> Result<(), String> {
    settings.hotkey = Settings::normalize_hotkey(&settings.hotkey);

    settings.theme = match settings.theme.as_str() {
        "light" | "dark" | "auto" => settings.theme.clone(),
        _ => "auto".to_string(),
    };

    let old_hotkey = {
        let current = state.settings.lock().map_err(|e| e.to_string())?;
        current.hotkey.clone()
    };
    let theme = settings.theme.clone();
    let hotkey_changed = settings.hotkey != old_hotkey;
    let new_hotkey = settings.hotkey.clone();

    settings.save()?;
    {
        let mut current = state.settings.lock().map_err(|e| e.to_string())?;
        *current = settings;
    }

    let _ = app.emit("theme:changed", json!({ "theme": theme }));

    if hotkey_changed {
        let ok = crate::hotkey::reregister_app_hotkey(
            &app,
            &new_hotkey,
            state.is_recording.clone(),
        );
        let _ = app.emit(
            "hotkey:registered",
            json!({ "hotkey": new_hotkey, "success": ok }),
        );
    }

    Ok(())
}

#[tauri::command]
pub fn update_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    hotkey: String,
) -> Result<(), String> {
    let normalized = Settings::normalize_hotkey(&hotkey);
    let old_hotkey = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.hotkey.clone()
    };
    if normalized == old_hotkey {
        return Ok(());
    }

    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.hotkey = normalized.clone();
        settings.save()?;
    }

    let ok = crate::hotkey::reregister_app_hotkey(
        &app,
        &normalized,
        state.is_recording.clone(),
    );
    let _ = app.emit(
        "hotkey:registered",
        json!({ "hotkey": normalized, "success": ok }),
    );
    Ok(())
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

fn model_catalog_entry(id: &str, desc: &str, speed: &str) -> serde_json::Value {
    let size_bytes = catalog_model_size_bytes(id);
    json!({
        "id": id,
        "size": format_byte_size_approx(size_bytes),
        "size_bytes": size_bytes,
        "desc": desc,
        "speed": speed,
    })
}

#[tauri::command]
pub async fn get_models() -> Result<serde_json::Value, String> {
    let mut available: Vec<serde_json::Value> = vec![
        model_catalog_entry("tiny.en", "Tiny (English only)", "fastest"),
        model_catalog_entry("tiny", "Tiny (multilingual)", "fastest"),
        model_catalog_entry("base.en", "Base (English only)", "fast"),
        model_catalog_entry("base", "Base (multilingual)", "fast"),
        model_catalog_entry("small.en", "Small (English only) — recommended", "medium"),
        model_catalog_entry("small", "Small (multilingual)", "medium"),
        model_catalog_entry("medium.en", "Medium (English only) — best for 16/24GB Mac", "slow"),
        model_catalog_entry("medium", "Medium (multilingual)", "slow"),
        model_catalog_entry("large-v3", "Large v3 (most accurate)", "slowest"),
        model_catalog_entry("large-v3-turbo", "Large v3 Turbo (fast + accurate)", "medium"),
    ];

    for model in available.iter_mut() {
        let id = model["id"].as_str().unwrap_or("");
        if crate::downloader::is_model_complete(id) {
            let size = crate::downloader::get_model_size(id);
            model["downloaded"] = json!(true);
            model["downloaded_size"] = json!(format_byte_size(size));
            model["size_bytes"] = json!(size);
        } else {
            // Incomplete: leftover `.part` / `.parts.json` / truncated `.bin`.
            let part_bytes = crate::downloader::get_partial_download_size(id);
            let bin_bytes = crate::downloader::get_model_size(id);
            let have = part_bytes.max(bin_bytes);
            if crate::downloader::has_partial_download(id) {
                let expected = crate::downloader::expected_size(id);
                let partial_label = if expected > 0 && have > 0 {
                    format!(
                        "Incomplete — {:.0}%",
                        (have as f64 / expected as f64 * 100.0).min(99.0)
                    )
                } else if have > 0 {
                    format!("Incomplete ({})", format_byte_size(have))
                } else {
                    "Incomplete — Resume".to_string()
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
pub fn get_server_status(app: AppHandle) -> Result<serde_json::Value, String> {
    Ok(json!({
        "embedded": crate::server::has_embedded_server(&app),
        "running": crate::server::is_running(),
        "port": crate::server::current_port(),
    }))
}

#[tauri::command]
pub fn stop_whisper_server(app: AppHandle) -> Result<serde_json::Value, String> {
    if !crate::server::has_embedded_server(&app) {
        return Err("No embedded whisper server in this build".to_string());
    }
    let was_running = crate::server::is_running();
    crate::server::stop_server();
    if was_running {
        log_message("[server] Model unloaded from memory (model files unchanged on disk)");
    }
    let _ = app.emit("server:stopped", json!({ "unloaded": was_running }));
    Ok(json!({ "stopped": was_running, "unloaded": was_running }))
}

#[tauri::command]
pub async fn restart_whisper_server(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if !crate::server::has_embedded_server(&app) {
        return Err("No embedded whisper server in this build".to_string());
    }
    if MODEL_OPERATION_IN_PROGRESS.load(Ordering::SeqCst) {
        return Err("A model operation is already in progress".to_string());
    }
    if state.is_recording.load(Ordering::SeqCst) {
        return Err("Stop recording before restarting the server".to_string());
    }

    let model = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.model.clone()
    };

    if !downloader::is_model_complete(&model) {
        return Err(format!(
            "Model '{}' is not fully downloaded",
            model
        ));
    }

    let _ = app.emit("server:starting", json!({}));
    crate::server::stop_server();
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    let Some(info) = crate::server::start_server(&app, &model) else {
        let _ = app.emit(
            "server:error",
            json!({ "reason": "Failed to start server — check that the model is fully downloaded." }),
        );
        return Err("Failed to start server".to_string());
    };

    if !crate::server::wait_for_server(info.port).await {
        crate::server::stop_server();
        let _ = app.emit(
            "server:error",
            json!({ "reason": "Server failed to start within timeout" }),
        );
        return Err("Server failed to start within timeout".to_string());
    }

    let api_url = format!("http://127.0.0.1:{}/inference", info.port);
    if let Ok(mut s) = state.settings.lock() {
        s.api_url = api_url.clone();
        s.model = info.model.clone();
        if let Err(e) = s.save() {
            log_message(&format!("[server] Failed to save settings after restart: {}", e));
        }
    }

    log_message(&format!("[server] Manual restart succeeded on port {}", info.port));
    let _ = app.emit(
        "server:ready",
        json!({ "port": info.port, "model": info.model }),
    );
    Ok(json!({
        "status": "ready",
        "port": info.port,
        "api_url": api_url,
        "model": info.model,
    }))
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
pub fn delete_history_audio(index: usize) -> Result<(), String> {
    History::delete_entry_audio(index)
}

#[tauri::command]
pub fn clear_history() -> Result<(), String> {
    History::clear()
}

#[tauri::command]
pub fn get_history_audio(index: usize) -> Result<Vec<u8>, String> {
    History::read_entry_audio_bytes(index)
}

#[tauri::command]
pub async fn reprocess_history_entry(
    app: AppHandle,
    state: State<'_, AppState>,
    index: usize,
) -> Result<serde_json::Value, String> {
    if state.is_recording.load(Ordering::SeqCst) {
        return Err("Cannot reprocess while recording".to_string());
    }
    if state.is_transcribing.load(Ordering::SeqCst) {
        return Err("Cannot reprocess while transcribing".to_string());
    }

    let mut settings = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.clone()
    };

    ensure_settings_api_ready(&app, &mut settings).await?;

    let (samples, sample_rate) = History::read_entry_audio(index)?;
    let reprocess_duration = samples.len() as f64 / sample_rate as f64;

    crate::debug::start_session(&settings.model);
    crate::debug::capture_audio(&samples, sample_rate);
    let result = transcribe_chunked(samples, sample_rate, &settings).await;
    let text = match result {
        Ok(t) => {
            crate::debug::update_meta(reprocess_duration, "reprocess");
            crate::debug::finish_session(&t);
            t
        }
        Err(e) => {
            crate::debug::discard_session();
            return Err(format!("Transcription failed: {}", e));
        }
    };

    if text.is_empty() {
        return Err("No speech detected in recording".to_string());
    }

    History::update_text(index, &text)?;

    Ok(json!({ "text": text, "index": index }))
}

fn show_and_focus_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_about(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("about") {
        return show_and_focus_window(&window);
    }

    let window = WebviewWindowBuilder::new(&app, "about", WebviewUrl::App("about.html".into()))
        .title("About SpeakType")
        .inner_size(320.0, 380.0)
        .resizable(false)
        .center()
        .build()
        .map_err(|e| e.to_string())?;

    show_and_focus_window(&window)
}

const SETTINGS_WINDOW_W: f64 = 880.0;
const SETTINGS_WINDOW_H: f64 = 640.0;
const SETTINGS_WINDOW_MIN_W: f64 = 720.0;
const SETTINGS_WINDOW_MIN_H: f64 = 540.0;

fn settings_window_geometry(app: &AppHandle) -> (Option<f64>, Option<f64>, f64, f64) {
    let settings = match app.try_state::<AppState>() {
        Some(state) => match state.settings.lock() {
            Ok(s) => s.clone(),
            Err(_) => Settings::load(),
        },
        None => Settings::load(),
    };
    let w = settings
        .settings_w
        .unwrap_or(SETTINGS_WINDOW_W)
        .max(SETTINGS_WINDOW_MIN_W);
    let h = settings
        .settings_h
        .unwrap_or(SETTINGS_WINDOW_H)
        .max(SETTINGS_WINDOW_MIN_H);
    (settings.settings_x, settings.settings_y, w, h)
}

/// Copy the live settings-window geometry into the in-memory settings. Disk
/// writes are deferred to close/quit so dragging does not hammer settings.json.
fn update_settings_window_geometry(window: &WebviewWindow) {
    let app = window.app_handle();
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Ok(scale) = window.scale_factor() else {
        return;
    };
    let Ok(pos) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.inner_size() else {
        return;
    };
    if size.width == 0 || size.height == 0 {
        return;
    }
    let logical = size.to_logical::<f64>(scale);

    let Ok(mut settings) = state.settings.lock() else {
        return;
    };
    // Recorded unclamped: macOS lets a window hang off an edge or straddle two
    // monitors while it is being dragged, and clamping here would overwrite the
    // real position with a corner fallback. `open_settings` validates instead.
    settings.settings_x = Some(pos.x as f64);
    settings.settings_y = Some(pos.y as f64);
    settings.settings_w = Some(logical.width);
    settings.settings_h = Some(logical.height);
}

pub fn persist_settings_window_geometry(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        update_settings_window_geometry(&window);
    }
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let save_result = {
        let Ok(settings) = state.settings.lock() else {
            return;
        };
        settings.save()
    };
    let _ = save_result;
}

#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        return show_and_focus_window(&window);
    }

    let (saved_x, saved_y, width, height) = settings_window_geometry(&app);

    let window = WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("settings.html".into()))
        .title("SpeakType Settings")
        .inner_size(width, height)
        .min_inner_size(SETTINGS_WINDOW_MIN_W, SETTINGS_WINDOW_MIN_H)
        .resizable(true)
        .build()
        .map_err(|e| e.to_string())?;

    // A position the clamp rejects falls back to the widget's top-right corner,
    // which is meaningless for a window this size — centre it instead.
    let restored = match (saved_x, saved_y) {
        (Some(x), Some(y)) => {
            let (cx, cy) = crate::window::apply_saved_position(&window, x, y);
            (cx - x).abs() < 1.0 && (cy - y).abs() < 1.0
        }
        _ => false,
    };
    if !restored {
        let _ = window.center();
    }

    {
        let geometry_app = app.clone();
        window.on_window_event(move |event| match event {
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                if let Some(w) = geometry_app.get_webview_window("settings") {
                    update_settings_window_geometry(&w);
                }
            }
            tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed => {
                persist_settings_window_geometry(&geometry_app);
                stop_mic_test_for_app(&geometry_app, "window_closed");
            }
            _ => {}
        });
    }

    show_and_focus_window(&window)
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
    if let Some(window) = app.get_webview_window("onboarding") {
        persist_onboarding_window_position(&window);
    }
    if app.get_webview_window("settings").is_some() {
        persist_settings_window_geometry(&app);
    }
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

#[tauri::command]
pub async fn reset_widget_position(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("Widget window not found".to_string());
    };

    let _ = crate::window::reset_to_default_position(&window);

    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.window_x = None;
        settings.window_y = None;
        settings.save()?;
    }

    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    let _ = window.set_always_on_top(true);

    let _ = window.emit(
        "widget:highlight",
        json!({ "duration_ms": 2500 }),
    );

    Ok(())
}

/// Latest debug sessions (newest first) captured by the Debug panel.
#[tauri::command]
pub fn get_debug_sessions() -> Result<Vec<crate::debug::Session>, String> {
    Ok(crate::debug::sessions())
}

#[tauri::command]
pub fn clear_debug_sessions() -> Result<(), String> {
    crate::debug::clear();
    Ok(())
}

/// Return the audio between `start_secs` and `end_secs` of a debug session's
/// recording as a mono WAV, for per-chunk/per-segment playback in the Debug
/// panel. Empty when the session kept no audio.
#[tauri::command]
pub fn get_debug_audio_slice(
    session_id: u64,
    start_secs: f64,
    end_secs: f64,
) -> Result<Vec<u8>, String> {
    Ok(crate::debug::slice_audio(session_id, start_secs, end_secs).unwrap_or_default())
}
