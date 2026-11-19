use crate::audio::{self, AudioRecorder, DeviceLevels, LevelMonitor};
use crate::chunk_session::{transcribe_chunked, ChunkSession};
use crate::controller::{self, DictationPhase, DictationState};
use crate::download_queue;
use crate::downloader;
use crate::format::{catalog_model_size_bytes, format_byte_size, format_byte_size_approx};
use crate::history::{self, DictationOutcome, History};
use crate::logging::log_message;
use crate::paste::{self, WindowInfo};
use crate::session_cmds::IpcError;
use crate::settings::Settings;
use crate::shortcuts::{self, ShortcutAction, ShortcutBinding, ShortcutsChanged};
use crate::tones::{self, Tone};
use crate::tray::{self, TrayState};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri_specta::Event;

use tauri::{
    AppHandle, Emitter, Manager, Runtime, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_opener::OpenerExt;

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
    pub level_monitor: Arc<Mutex<LevelMonitor>>,
    pub mic_levels: Arc<Mutex<DeviceLevels<LevelsUi>>>,
    pub dictation: Arc<Mutex<DictationState>>,
    /// The audio of the latest dictation, with the entry it belongs to. Onboarding's
    /// compare re-runs it without the recording being saved and without
    /// overwriting the entry.
    pub last_dictation: Arc<Mutex<Option<LastDictation>>>,
    /// Set when the first press's model load gave up, so "Getting ready…" can
    /// end in the server error instead of waiting out its bound.
    pub model_load_failed: Arc<AtomicBool>,
    pub is_recording: Arc<AtomicBool>,
    pub is_transcribing: Arc<AtomicBool>,
    pub target_window: Arc<Mutex<Option<WindowInfo>>>,
    pub last_active_window: Arc<Mutex<Option<WindowInfo>>>,
    pub chunk_session: Arc<Mutex<Option<ChunkSession<tauri::Wry>>>>,
    pub is_pasting: Arc<AtomicBool>,
    /// Each action's live registration, so a change touches only its own.
    pub shortcuts: Arc<Mutex<shortcuts::LiveShortcuts>>,
}

/// Kept in memory because *Save recording audio* may be off: the newest
/// dictation can still be re-run until the next one replaces it.
pub struct LastDictation {
    pub entry_id: String,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

/// Open the microphone and the chunk session. The dictation state is set by
/// the controller that calls this.
fn begin_recording(app: &AppHandle, state: &AppState) -> Result<(), DictationState> {
    let (settings, input_device) = state
        .settings
        .lock()
        .map(|s| (s.clone(), s.input_device.clone()))
        .map_err(|_| controller::mic_unavailable())?;

    // Hard gate (spec §12): without the model, the microphone or Accessibility
    // there is nothing to record into, so the failing step's notice and
    // onboarding show instead of a take that cannot land.
    if !crate::downloader::is_model_complete(&settings.model) {
        tones::play(Tone::Error);
        let _ = open_onboarding_window(app);
        return Err(controller::server_error());
    }
    if !crate::permissions::check_microphone() || !crate::permissions::check_accessibility(false) {
        log_message("[dictation] Requirements not met — opening onboarding");
        tones::play(Tone::Error);
        let _ = open_onboarding_window(app);
        return Err(controller::mic_unavailable());
    }

    // Dictation owns the input device. A settings-window meter must never be
    // able to block or race the hotkey, so it is dropped rather than consulted.
    stop_mic_test_internal(app, state, "recording");
    stop_mic_levels(app);

    {
        let last = state
            .last_active_window
            .lock()
            .map_err(|_| controller::mic_unavailable())?;
        let mut tw = state
            .target_window
            .lock()
            .map_err(|_| controller::mic_unavailable())?;
        *tw = last.clone();
    }

    crate::debug::start_session(&settings.model);
    let mut recorder = state
        .recorder
        .lock()
        .map_err(|_| controller::mic_unavailable())?;
    let buffer = recorder.buffer();
    let app_for_switch = app.clone();
    recorder
        .start_with_input(app.clone(), input_device.as_deref(), move || {
            emit_dictation(&app_for_switch, controller::mic_fallback());
        })
        .map_err(|e| {
            log_message(&format!("[dictation] Microphone: {}", e));
            controller::mic_unavailable()
        })?;
    let sample_rate = recorder.sample_rate();

    let session = ChunkSession::start(app.clone(), settings, sample_rate, buffer);
    let mut chunk_session = state
        .chunk_session
        .lock()
        .map_err(|_| controller::mic_unavailable())?;
    if let Some(old) = chunk_session.take() {
        old.abort();
    }
    *chunk_session = Some(session);

    state.is_recording.store(true, Ordering::SeqCst);
    tray::set_tray_state(app, TrayState::Recording);
    tones::play(Tone::RecordingStart);
    Ok(())
}

fn end_recording(app: &AppHandle, state: &AppState) -> Result<(Vec<f32>, u32), String> {
    state.is_recording.store(false, Ordering::SeqCst);
    let (samples, sample_rate) = {
        let mut recorder = state.recorder.lock().map_err(|e| e.to_string())?;
        recorder.stop()
    };
    tray::set_tray_state(app, TrayState::Transcribing);
    tones::play(Tone::RecordingStop);
    Ok((samples, sample_rate))
}

/// Keep the latest dictation's audio for `reprocess_preview`. Each new
/// dictation replaces the previous one, whether or not it was saved to disk.
fn replace_last_dictation(app: &AppHandle, next: Option<LastDictation>) {
    if let Some(current) = app.try_state::<AppState>() {
        if let Ok(mut last) = current.last_dictation.lock() {
            *last = next;
        }
    }
}

fn discard_recording(app: &AppHandle, state: &AppState) {
    state.is_recording.store(false, Ordering::SeqCst);
    if let Ok(mut recorder) = state.recorder.lock() {
        let _ = recorder.stop();
    }
    if let Ok(mut session) = state.chunk_session.lock() {
        if let Some(active) = session.take() {
            active.abort();
        }
    }
    crate::debug::discard_session();
    tray::set_tray_state(app, TrayState::Idle);
    tones::play(Tone::RecordingStop);
}

struct DictationHost<'a> {
    app: &'a AppHandle,
    state: &'a AppState,
}

impl crate::controller::DictationHost for DictationHost<'_> {
    fn state(&self) -> DictationState {
        self.state
            .dictation
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| controller::idle())
    }

    fn set_state(&self, next: DictationState) {
        remember_dictation(self.app, next);
    }

    fn start_recording(&self) -> Result<(), DictationState> {
        begin_recording(self.app, self.state)
    }

    fn stop_recording(&self) -> Result<(Vec<f32>, u32), String> {
        end_recording(self.app, self.state)
    }

    fn cancel_recording(&self) {
        discard_recording(self.app, self.state);
    }

    fn transcribe(&self, samples: Vec<f32>, sample_rate: u32) {
        let settings = self
            .state
            .settings
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| Settings::default());
        let target_window = if settings.paste_mode == "active" {
            self.state
                .last_active_window
                .lock()
                .ok()
                .and_then(|w| w.clone())
        } else {
            self.state.target_window.lock().ok().and_then(|w| w.clone())
        };
        let chunk_session = self
            .state
            .chunk_session
            .lock()
            .map(|mut s| s.take())
            .ok()
            .flatten();
        let app = self.app.clone();
        let is_transcribing = self.state.is_transcribing.clone();
        tauri::async_runtime::spawn(async move {
            process_recording(
                app,
                samples,
                sample_rate,
                settings,
                target_window,
                is_transcribing,
                chunk_session,
            )
            .await;
        });
    }
}

/// Shared press latch: a second press while the first is starting a take is
/// dropped, so the hotkey, the tray and the command cannot start two.
static PRESSING: AtomicBool = AtomicBool::new(false);

/// Run one hotkey press through the controller on the app's runtime: a press
/// may wait for the model to load, which would stall the caller. The tray, the
/// hotkey and the command all arrive here, and the latch keeps a rapid double
/// press from starting two takes.
pub fn press_dictation(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        let host = DictationHost {
            app: &app,
            state: &state,
        };
        let gate = ServerGate { app: &app };
        let mut dictation = controller::Dictation::new(&host, &gate, &PRESSING);
        dictation.press().await;
    });
}

struct ServerGate<'a> {
    app: &'a AppHandle,
}

impl crate::controller::ServerGate for ServerGate<'_> {
    fn ready(&self) -> bool {
        !crate::server::has_embedded_server(self.app) || crate::server::is_running()
    }

    /// Bounded by the controller's `SERVER_WAIT_MAX`, and it gives up as soon
    /// as the load task reports a failure, so "Getting ready…" always ends.
    async fn wait(&self) -> bool {
        load_model_for_dictation(self.app);
        let load_failed = || {
            self.app
                .try_state::<AppState>()
                .is_some_and(|state| state.model_load_failed.load(Ordering::SeqCst))
        };
        let deadline = tokio::time::Instant::now() + crate::controller::SERVER_WAIT_MAX;
        loop {
            if self.ready() {
                return true;
            }
            if load_failed() {
                log_message("[dictation] Speech model failed to load");
                return false;
            }
            if tokio::time::Instant::now() >= deadline {
                log_message("[dictation] Speech model still not running after 65 s");
                return false;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Store the state and tell every window. The Esc shortcut follows recording
/// exactly: armed on recording, released on every other phase.
pub fn remember_dictation<R: Runtime>(app: &AppHandle<R>, state: DictationState) {
    if let Some(app_state) = app.try_state::<AppState>() {
        if let Ok(mut current) = app_state.dictation.lock() {
            *current = state.clone();
        }
    }
    emit_dictation(app, state);
}

/// Tell the pill without touching the stored state. The mic fallback uses it
/// from the recorder's stream-error callback, so the recording state with its
/// notice reaches the pill while the take carries on.
pub fn emit_dictation<R: Runtime>(app: &AppHandle<R>, state: DictationState) {
    let _ = state.emit(app);
    if let Some(esc) = app.try_state::<crate::EscHolder>() {
        if let Ok(mut esc) = esc.0.lock() {
            esc.sync(state.phase.is_recording());
        }
    }
}

/// A freshly loaded window missed any event before it mounted; the pill starts
/// from the current phase instead of waiting for the next press.
pub fn replay_dictation_state(app: &AppHandle) {
    let Some(app_state) = app.try_state::<AppState>() else {
        return;
    };
    let state = app_state
        .dictation
        .lock()
        .map(|s| s.clone())
        .unwrap_or_else(|_| controller::idle());
    let _ = state.emit(app);
}

#[tauri::command]
#[specta::specta]
pub fn toggle_recording(
    app: AppHandle,
    #[allow(unused_variables)] state: State<'_, AppState>,
    #[allow(unused_variables)] is_recording: bool,
) -> Result<(), String> {
    press_dictation(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn cancel_recording(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let host = DictationHost {
        app: &app,
        state: &state,
    };
    let gate = ServerGate { app: &app };
    let mut dictation = controller::Dictation::new(&host, &gate, &PRESSING);
    dictation.cancel();
    Ok(())
}

/// Esc while recording (the temporary global shortcut).
pub fn cancel_dictation(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let host = DictationHost { app, state: &state };
    let gate = ServerGate { app };
    let mut dictation = controller::Dictation::new(&host, &gate, &PRESSING);
    dictation.cancel();
}

/// The tray's Start/Stop Recording item: press, then label the item for the
/// phase the take actually reached.
pub fn toggle_from_tray(app: AppHandle, item: tauri::menu::MenuItem<tauri::Wry>) {
    tauri::async_runtime::spawn(async move {
        press_dictation(&app);
        let recording = app
            .state::<AppState>()
            .dictation
            .lock()
            .map(|s| s.phase.is_recording())
            .unwrap_or(false);
        let _ = item.set_text(if recording {
            "Stop Recording"
        } else {
            "Start Recording"
        });
    });
}

/// Runs while the model server starts on the first press. Mirrors the states
/// the controller already set, so the pill is honest about a failed load.
pub fn load_model_for_dictation(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.model_load_failed.store(false, Ordering::SeqCst);
    }
    let model = app
        .state::<AppState>()
        .settings
        .lock()
        .map(|s| s.model.clone())
        .unwrap_or_default();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match crate::server::ensure_running(&app, &model).await {
            Ok(Some(info)) => {
                if let Ok(mut s) = app.state::<AppState>().settings.lock() {
                    s.api_url = format!("http://127.0.0.1:{}/inference", info.port);
                    s.model = info.model.clone();
                    if let Err(e) = s.save() {
                        log_message(&format!("[dictation] Failed to save settings: {}", e));
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                log_message(&format!("[dictation] Model load failed: {}", e));
                // A waiting press ends "Getting ready…" from this flag; a
                // press that already gave up leaves the pill on its error.
                app.state::<AppState>()
                    .model_load_failed
                    .store(true, Ordering::SeqCst);
            }
        }
    });
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
#[specta::specta]
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
        let input_device = state
            .settings
            .lock()
            .ok()
            .and_then(|s| s.input_device.clone());
        monitor.start_with_input(app.clone(), input_device.as_deref())?
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
#[specta::specta]
pub fn stop_mic_test(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    stop_mic_test_internal(&app, &state, "user");
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_audio_input_info() -> Result<serde_json::Value, String> {
    Ok(json!({
        "device": audio::get_default_input_device_name(),
        "inputs": audio::count_input_devices(),
    }))
}

#[tauri::command]
#[specta::specta]
pub fn get_input_devices() -> Result<serde_json::Value, String> {
    Ok(json!({
        "devices": audio::list_input_devices(),
        "default": audio::get_default_input_device_name(),
    }))
}

/// Forward webview console errors/warnings into the app log so that a frozen
/// settings window or widget leaves a trace on the next occurrence.
#[tauri::command]
#[specta::specta]
pub fn log_frontend(level: String, message: String) {
    crate::logging::log_message(&format!("[webview] {} {}", level, message));
}

/// Copy text to the system clipboard.
///
/// Tauri serves the UI from a custom protocol origin, where `navigator.clipboard`
/// is not a secure context and may be undefined. UI copy buttons therefore need
/// this native path.
#[tauri::command]
#[specta::specta]
pub fn copy_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())
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
    chunk_session: Option<ChunkSession<tauri::Wry>>,
) {
    if is_transcribing
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        log_message("[process] Already transcribing, skipping");
        remember_dictation(&app, controller::busy());
        return;
    }

    let _guard = scopeguard::guard((), |_| {
        is_transcribing.store(false, Ordering::SeqCst);
    });

    remember_dictation(&app, controller::transcribing());

    if let Err(e) = ensure_settings_api_ready(&app, &mut settings).await {
        log_message(&format!(
            "[process] Failed to load model for transcription: {}",
            e
        ));
        tones::play(Tone::Error);
        tray::flash_error(&app);
        crate::debug::discard_session();
        remember_dictation(&app, controller::server_error());
        return;
    }

    let duration_secs = samples.len() as f64 / sample_rate as f64;

    if duration_secs > 120.0 {
        log_message(&format!(
            "[process] Long audio ({:.0}s), using chunked transcription",
            duration_secs
        ));
        let _ = app.emit(
            "dictation:long-audio",
            json!({"duration_secs": duration_secs}),
        );
    }

    if samples.is_empty() || !audio::has_speech(&samples, sample_rate) {
        log_message("[process] No speech detected");
        tones::play(Tone::Error);
        tray::flash_error(&app);
        crate::debug::discard_session();
        remember_dictation(&app, controller::no_speech());
        return;
    }

    let entry_id = history::new_entry_id();
    let saved_recording = if settings.save_recordings {
        match History::save_recording(&entry_id, &samples, sample_rate) {
            Ok(saved) => Some(saved),
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
                match transcribe_chunked(&app, samples.clone(), sample_rate, &settings).await {
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
                        remember_dictation(&app, controller::server_error());
                        return;
                    }
                }
            }
        }
    } else {
        crate::debug::capture_audio(&samples, sample_rate);
        match transcribe_chunked(&app, samples.clone(), sample_rate, &settings).await {
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
                remember_dictation(&app, controller::server_error());
                return;
            }
        }
    };

    if text.is_empty() {
        History::discard_saved_recording(&saved_recording);
        log_message("[process] No usable text left after hallucination scoring");
        tones::play(Tone::Error);
        tray::flash_error(&app);
        remember_dictation(&app, controller::no_speech());
        return;
    }

    let paste_text = format!(" {}", text);
    log_message(&format!("[process] Pasting: {:?}", paste_text));

    let target = target_window.clone();
    let post_keys = settings.post_paste_keys.clone();
    let app_state_pasting = app.state::<AppState>().is_pasting.clone();
    app_state_pasting.store(true, Ordering::SeqCst);
    let paste_result = tokio::task::spawn_blocking(move || {
        paste::paste_text(&paste_text, target.as_ref(), post_keys.as_deref())
    })
    .await;
    app_state_pasting.store(false, Ordering::SeqCst);

    let (outcome, state) = controller::finish_outcome(match paste_result {
        Ok(true) => controller::PasteResult::Pasted,
        Ok(false) => {
            log_message("[process] Paste not delivered — transcript kept on clipboard");
            tones::play(Tone::Error);
            tray::flash_error(&app);
            controller::PasteResult::Failed
        }
        Err(e) => {
            log_message(&format!("[process] Paste task error: {}", e));
            tones::play(Tone::Error);
            tray::flash_error(&app);
            controller::PasteResult::Copied
        }
    });

    let recording_duration_ms = saved_recording
        .as_ref()
        .map(|saved| (saved.duration_secs * 1000.0) as i64);

    let mut history = History::load();
    history.add_entry(
        &entry_id,
        &text,
        outcome,
        history::retention_limit(&settings),
        saved_recording,
    );
    if let Err(e) = history.save() {
        log_message(&format!("[history] Save error: {}", e));
    }
    replace_last_dictation(
        &app,
        Some(LastDictation {
            entry_id: entry_id.clone(),
            samples,
            sample_rate,
        }),
    );

    // Dictation inside an active session becomes a speech item; the session
    // stays open (spec §12: dictations no longer close sessions).
    let session_store = app.state::<Arc<speaktype_session::Store>>();
    if let Ok(Some(session)) = session_store.active_session() {
        let now = chrono::Utc::now().timestamp_millis();
        let duration_ms = recording_duration_ms.unwrap_or(0);
        crate::session_cmds::add_dictation_item(
            &app,
            &session_store,
            session.id,
            &text,
            now - duration_ms,
            now,
        );
    }

    tray::set_tray_state(&app, TrayState::Idle);
    crate::history::emit_changed(&app);
    let state = app
        .try_state::<AppState>()
        .and_then(|current| current.dictation.lock().ok().map(|s| s.clone()))
        .filter(|s| s.phase == DictationPhase::Done)
        .map(|s| {
            // A mic switch happened before the paste, so keep that notice and
            // take only the outcome from it.
            DictationState::new(state.phase, s.notice)
        })
        .unwrap_or(state);
    remember_dictation(&app, state);
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
#[specta::specta]
pub fn check_permissions() -> Result<serde_json::Value, String> {
    let accessibility = crate::permissions::check_accessibility(false);
    let microphone = crate::permissions::check_microphone();
    Ok(json!({
        "accessibility": accessibility,
        "microphone": microphone,
    }))
}

/// Onboarding status used by the onboarding window to drive its serial steps.
#[tauri::command]
#[specta::specta]
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
#[specta::specta]
pub fn request_microphone_access(app: AppHandle) -> Result<bool, String> {
    let granted = crate::permissions::request_microphone();
    emit_permissions(&app);
    Ok(granted)
}

/// Open (or focus) the onboarding window.
#[tauri::command]
#[specta::specta]
pub async fn open_onboarding(app: AppHandle) -> Result<(), String> {
    open_onboarding_window(&app)
}

/// Onboarding is centred every launch, so it has no saved position.
pub fn open_onboarding_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("onboarding") {
        let _ = window.show();
        window.set_focus().map_err(|e| e.to_string())?;
        position_widget_for_onboarding(app);
        let _ = app.emit("onboarding:opened", json!({}));
        return Ok(());
    }

    let _ = hidden_title_bar(WebviewWindowBuilder::new(
        app,
        "onboarding",
        WebviewUrl::App("onboarding.html".into()),
    ))
    .title("SpeakType")
    .inner_size(ONBOARDING_WINDOW_W, ONBOARDING_WINDOW_H)
    .resizable(false)
    .center()
    .build()
    .map_err(|e| e.to_string())?;

    position_widget_for_onboarding(app);
    let _ = app.emit("onboarding:opened", json!({}));
    Ok(())
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
#[specta::specta]
pub async fn finish_onboarding(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let model = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.model.clone()
    };

    if crate::downloader::is_model_complete(&model) && crate::server::current_port().is_none() {
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
    let hotkey = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        s.shortcut(ShortcutAction::Dictation)
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
#[specta::specta]
pub fn request_accessibility(app: AppHandle) -> Result<bool, String> {
    let granted = crate::permissions::check_accessibility(true);
    emit_permissions(&app);
    Ok(granted)
}

#[tauri::command]
#[specta::specta]
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
#[specta::specta]
pub fn open_models_folder(app: AppHandle) -> Result<(), String> {
    let dir = crate::paths::models_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create models directory: {}", e))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Failed to open models folder: {}", e))?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn open_recordings_folder(app: AppHandle) -> Result<(), String> {
    let dir = History::recordings_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create recordings directory: {}", e))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Failed to open recordings folder: {}", e))?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
#[specta::specta]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut settings: Settings,
) -> Result<(), String> {
    settings.theme = match settings.theme.as_str() {
        "light" | "dark" | "auto" => settings.theme.clone(),
        _ => "auto".to_string(),
    };
    let theme = settings.theme.clone();

    settings.save()?;
    {
        let mut current = state.settings.lock().map_err(|e| e.to_string())?;
        *current = settings;
    }
    crate::settings::emit_settings_changed(&app);

    let _ = app.emit("theme:changed", json!({ "theme": theme }));
    Ok(())
}

/// The registry's rows for Settings → Keyboard.
#[tauri::command]
#[specta::specta]
pub fn shortcuts_get(state: State<'_, AppState>) -> Result<Vec<ShortcutBinding>, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(shortcuts::bindings(&settings.shortcuts))
}

/// Change one shortcut: validate, re-register only that action, then persist
/// and tell every window. A refused change leaves the saved value alone.
#[tauri::command]
#[specta::specta]
pub fn shortcut_set(
    app: AppHandle,
    state: State<'_, AppState>,
    action: ShortcutAction,
    keys: String,
    force: bool,
) -> Result<(), IpcError> {
    let platform = crate::hotkey::AppShortcuts {
        app: app.clone(),
        is_recording: state.is_recording.clone(),
    };
    let mut live = state.shortcuts.lock().map_err(|e| storage(e.to_string()))?;
    let mut settings = state.settings.lock().map_err(|e| storage(e.to_string()))?;
    let stored = shortcuts::apply(
        &platform,
        &mut live,
        &settings.shortcuts,
        action,
        &keys,
        force,
    )?;
    settings.shortcuts.insert(action, stored);
    settings.save().map_err(storage)?;
    drop(settings);
    drop(live);
    crate::settings::emit_settings_changed(&app);
    ShortcutsChanged {}
        .emit(&app)
        .map_err(|e| storage(e.to_string()))?;
    Ok(())
}

/// Back to the spec's defaults, keeping any key another action holds; returns
/// the rows and re-registers every action that changed.
#[tauri::command]
#[specta::specta]
pub fn shortcuts_reset(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<ShortcutBinding>, IpcError> {
    let platform = crate::hotkey::AppShortcuts {
        app: app.clone(),
        is_recording: state.is_recording.clone(),
    };
    let mut live = state.shortcuts.lock().map_err(|e| storage(e.to_string()))?;
    let mut settings = state.settings.lock().map_err(|e| storage(e.to_string()))?;
    let restored = shortcuts::default_bindings(&settings.shortcuts);
    for action in ShortcutAction::ALL {
        let keys = restored[&action].clone();
        if settings.shortcuts.get(&action) == Some(&keys) {
            continue;
        }
        shortcuts::apply(
            &platform,
            &mut live,
            &settings.shortcuts,
            action,
            &keys,
            true,
        )?;
        settings.shortcuts.insert(action, keys);
    }
    settings.save().map_err(storage)?;
    let rows = shortcuts::bindings(&settings.shortcuts);
    drop(settings);
    drop(live);
    crate::settings::emit_settings_changed(&app);
    ShortcutsChanged {}
        .emit(&app)
        .map_err(|e| storage(e.to_string()))?;
    Ok(rows)
}

fn storage(message: impl Into<String>) -> IpcError {
    IpcError::new("storage", message.into())
}

#[tauri::command]
#[specta::specta]
pub fn set_launch_at_login(
    app: AppHandle,
    state: State<'_, AppState>,
    on: bool,
) -> Result<(), crate::session_cmds::IpcError> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|e| IpcError::new("launchAtLogin", e.to_string()))?;
    crate::settings::set_launch_at_login(Some(&app), on, &mut settings)?;
    drop(settings);
    crate::settings::emit_settings_changed(&app);
    Ok(())
}

/// The session shortcut: start a session, or stop the one that is running.
pub(crate) fn press_session(app: &AppHandle) {
    let active = app
        .state::<Arc<speaktype_session::Store>>()
        .active_session();
    let result = match active {
        Ok(Some(_)) => {
            crate::session_cmds::session_stop(app.clone(), app.state(), app.state()).map(|_| ())
        }
        Ok(None) => crate::session_cmds::session_start(
            None,
            app.clone(),
            app.state(),
            app.state(),
            app.state(),
        )
        .map(|_| ()),
        Err(e) => Err(e.into()),
    };
    if let Err(e) = result {
        log_message(&format!("[session] Shortcut: {}", e.message));
    }
}

/// The screenshot shortcut: capture into the active session.
pub(crate) fn press_screenshot(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::session_cmds::capture_screenshot(app).await {
            log_message(&format!("[capture] Screenshot shortcut: {}", e.message));
        }
    });
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
                    log_message(&format!(
                        "[load_model] Failed to save API URL setting: {}",
                        e
                    ));
                }
            }
            emit_model_progress(app, model, "done", &format!("{} model ready", model));
            Ok(json!({"status": "loaded", "model": model}))
        } else {
            crate::server::stop_server();
            let msg =
                "Server failed to start — the model file may be corrupt. Try Restart Download.";
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

    emit_model_progress(
        &app,
        &model,
        "preparing",
        &format!("Preparing {}...", model),
    );

    let model_for_task = model.clone();
    tauri::async_runtime::spawn(async move {
        let _ = restart_server_with_model(&app, &settings, &model_for_task).await;
        MODEL_OPERATION_IN_PROGRESS.store(false, Ordering::SeqCst);
    });

    Ok(json!({"status": "started", "model": model}))
}

#[tauri::command]
#[specta::specta]
pub async fn load_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model: String,
) -> Result<serde_json::Value, String> {
    if downloader::is_model_complete(&model) {
        return start_server_model_operation(app, state.settings.clone(), model);
    }
    download_queue::enqueue(app, state.settings.clone(), model, false, true)
}

#[tauri::command]
#[specta::specta]
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
#[specta::specta]
pub async fn restart_model_download(
    app: AppHandle,
    state: State<'_, AppState>,
    model: String,
) -> Result<serde_json::Value, String> {
    downloader::clear_model_files(&model)?;
    download_queue::enqueue(app, state.settings.clone(), model, true, true)
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_model_download(
    app: AppHandle,
    model: String,
) -> Result<serde_json::Value, String> {
    let cancelled = download_queue::cancel_waiting_with_app(&app, &model);
    Ok(json!({"cancelled": cancelled, "model": model}))
}

#[tauri::command]
#[specta::specta]
pub async fn pause_model_download(
    app: AppHandle,
    model: String,
) -> Result<serde_json::Value, String> {
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
#[specta::specta]
pub async fn get_models() -> Result<serde_json::Value, String> {
    let mut available: Vec<serde_json::Value> = vec![
        model_catalog_entry("tiny.en", "Tiny (English only)", "fastest"),
        model_catalog_entry("tiny", "Tiny (multilingual)", "fastest"),
        model_catalog_entry("base.en", "Base (English only)", "fast"),
        model_catalog_entry("base", "Base (multilingual)", "fast"),
        model_catalog_entry("small.en", "Small (English only) — recommended", "medium"),
        model_catalog_entry("small", "Small (multilingual)", "medium"),
        model_catalog_entry(
            "medium.en",
            "Medium (English only) — best for 16/24GB Mac",
            "slow",
        ),
        model_catalog_entry("medium", "Medium (multilingual)", "slow"),
        model_catalog_entry("large-v3", "Large v3 (most accurate)", "slowest"),
        model_catalog_entry(
            "large-v3-turbo",
            "Large v3 Turbo (fast + accurate)",
            "medium",
        ),
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

/// Remove a downloaded model the user no longer wants; the active model stays.
#[tauri::command]
#[specta::specta]
pub fn delete_model(state: State<'_, AppState>, id: String) -> Result<(), IpcError> {
    if id.is_empty() {
        return Err(IpcError::new("download", "Unknown model"));
    }
    let active = state
        .settings
        .lock()
        .map_err(|e| IpcError::new("settings", e.to_string()))?
        .model
        .clone();
    downloader::delete_model_file(&id, &active).map_err(|e| match e {
        downloader::DeleteError::InUse => IpcError::new(
            "modelInUse",
            "This model is in use. Switch to another model first.",
        ),
        downloader::DeleteError::Failed(message) => IpcError::new("download", message),
    })
}

const MIC_LEVELS_EVENT: &str = "mic:levels";

fn emit_mic_levels(app: &AppHandle, levels: &[audio::DeviceLevel]) {
    let _ = app.emit(MIC_LEVELS_EVENT, json!({ "levels": levels }));
}

/// Metering streams for every input device while the microphone picker is
/// open. `start` replaces the round, which is also how the picker's 20 s
/// keep-alive re-arms the 30 s auto-stop.
#[tauri::command]
#[specta::specta]
pub fn mic_levels_start(app: AppHandle, state: State<'_, AppState>) -> Result<(), IpcError> {
    let ui = levels_ui(&app);
    let mut levels = state
        .mic_levels
        .lock()
        .map_err(|e| IpcError::new("audio", e.to_string()))?;
    *levels = audio::DeviceLevels::new(ui);
    levels.start();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn mic_levels_stop(state: State<'_, AppState>) -> Result<(), IpcError> {
    state
        .mic_levels
        .lock()
        .map_err(|e| IpcError::new("audio", e.to_string()))?
        .stop();
    Ok(())
}

/// Release metering streams from a context with only an `AppHandle` (menu
/// close, window hide, exit) — the picker cannot call the command then.
pub fn stop_mic_levels(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut levels) = state.mic_levels.lock() {
            levels.stop();
        }
    }
}

pub(crate) fn levels_ui(app: &AppHandle) -> LevelsUi {
    let emitter = app.clone();
    LevelsUi {
        emit: Arc::new(move |levels: &[audio::DeviceLevel]| emit_mic_levels(&emitter, levels)),
        app: app.clone(),
    }
}

type EmitLevels = Arc<dyn Fn(&[audio::DeviceLevel]) + Send + Sync>;

#[derive(Clone)]
pub(crate) struct LevelsUi {
    emit: EmitLevels,
    app: AppHandle,
}

impl audio::MicLevelsUi for LevelsUi {
    fn emit_levels(&self, levels: &[audio::DeviceLevel]) {
        (self.emit)(levels);
    }

    fn emit_error(&self, message: &str) {
        let _ = self
            .app
            .emit(MIC_LEVELS_EVENT, json!({ "levels": [], "error": message }));
    }

    fn stop(&self) {
        stop_mic_levels(&self.app);
    }

    fn sleep(
        &self,
        duration: std::time::Duration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(tokio::time::sleep(duration))
    }

    fn now(&self) -> std::time::Duration {
        mic_levels_origin().elapsed()
    }
}

/// Shared by the round's ticks, which always run on the same async worker.
fn mic_levels_origin() -> Instant {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    *ORIGIN.get_or_init(Instant::now)
}

#[tauri::command]
#[specta::specta]
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
#[specta::specta]
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
#[specta::specta]
pub fn get_server_status(app: AppHandle) -> Result<serde_json::Value, String> {
    Ok(json!({
        "embedded": crate::server::has_embedded_server(&app),
        "running": crate::server::is_running(),
        "port": crate::server::current_port(),
        "stopped": crate::server::is_stopped(),
    }))
}

/// What the menu bar has to say about the model: its name and size from the
/// catalog, or nothing while the server runs or loads.
#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub(crate) model: Option<String>,
    pub(crate) size: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub fn server_info(app: AppHandle) -> Result<ServerInfo, String> {
    let idle = !crate::server::is_running()
        && !MODEL_OPERATION_IN_PROGRESS.load(Ordering::SeqCst)
        && crate::server::has_embedded_server(&app);
    if !idle {
        return Ok(ServerInfo {
            model: None,
            size: None,
        });
    }
    let model = Settings::load().model;
    let bytes = crate::format::catalog_model_size_bytes(&model);
    let name = model.split('.').next().unwrap_or(&model).to_string();
    Ok(ServerInfo {
        model: Some(name),
        size: if bytes > 0 {
            Some(crate::format::format_byte_size_approx(bytes))
        } else {
            None
        },
    })
}

#[tauri::command]
#[specta::specta]
pub async fn unload_model(app: AppHandle) -> Result<(), IpcError> {
    if !crate::server::has_embedded_server(&app) {
        return Err(IpcError::new("server", "This build has no bundled server"));
    }
    crate::server::unload();
    let _ = app.emit("server:stopped", json!({}));
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn stop_whisper_server(app: AppHandle) -> Result<serde_json::Value, String> {
    if !crate::server::has_embedded_server(&app) {
        return Err("No embedded whisper server in this build".to_string());
    }
    let was_running = crate::server::is_running();
    crate::server::unload();
    if was_running {
        log_message("[server] Model unloaded from memory (model files unchanged on disk)");
    }
    let _ = app.emit("server:stopped", json!({ "unloaded": was_running }));
    Ok(json!({ "stopped": was_running, "unloaded": was_running }))
}

/// Load now, after an unload: the one path that clears the unloaded state, so
/// dictation loads the model lazily from then on.
#[tauri::command]
#[specta::specta]
pub async fn load_server(app: AppHandle, state: State<'_, AppState>) -> Result<(), IpcError> {
    if !crate::server::has_embedded_server(&app) {
        return Err(IpcError::new("server", "This build has no bundled server"));
    }
    if crate::server::is_running() {
        return Ok(());
    }
    let model = state
        .settings
        .lock()
        .map_err(|e| IpcError::new("server", e.to_string()))?
        .model
        .clone();
    if MODEL_OPERATION_IN_PROGRESS.load(Ordering::SeqCst) {
        return Err(IpcError::new(
            "server",
            "A model operation is already in progress",
        ));
    }
    crate::server::mark_loaded();
    crate::server::ensure_running(&app, &model)
        .await
        .map(|_| ())
        .map_err(|e| IpcError::new("server", e))
}

#[tauri::command]
#[specta::specta]
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
        return Err(format!("Model '{}' is not fully downloaded", model));
    }

    let _ = app.emit("server:starting", json!({}));
    crate::server::mark_loaded();
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
            log_message(&format!(
                "[server] Failed to save settings after restart: {}",
                e
            ));
        }
    }

    log_message(&format!(
        "[server] Manual restart succeeded on port {}",
        info.port
    ));
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

/// Paste again: re-deliver the newest history text into the current app.
#[tauri::command]
#[specta::specta]
pub async fn paste_last(app: AppHandle, state: State<'_, AppState>) -> Result<(), IpcError> {
    let Some(text) = History::load().newest_text() else {
        return Err(IpcError::new("history", "Nothing to paste yet"));
    };
    let settings = state
        .settings
        .lock()
        .map_err(|e| IpcError::new("settings", e.to_string()))?
        .clone();
    let target = state
        .last_active_window
        .lock()
        .map_err(|e| IpcError::new("paste", e.to_string()))?
        .clone();

    state.is_pasting.store(true, Ordering::SeqCst);
    let paste_text = format!(" {}", text);
    let pasted = tokio::task::spawn_blocking(move || {
        paste::paste_text(
            &paste_text,
            target.as_ref(),
            settings.post_paste_keys.as_deref(),
        )
    })
    .await
    .map_err(|e| IpcError::new("paste", e.to_string()))?;
    state.is_pasting.store(false, Ordering::SeqCst);

    let outcome = if pasted {
        DictationOutcome::Pasted
    } else {
        DictationOutcome::PasteFailed
    };
    let mut history = History::load();
    if let Some(entry) = history.entries.first_mut() {
        entry.outcome = Some(outcome);
        if let Err(e) = history.save() {
            log_message(&format!(
                "[history] Could not record the re-paste outcome: {}",
                e
            ));
        }
        crate::history::emit_changed(&app);
    }
    if pasted {
        Ok(())
    } else {
        Err(IpcError::new(
            "paste",
            "Could not paste; the text is on your clipboard",
        ))
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_history() -> Result<History, String> {
    Ok(History::load())
}

#[tauri::command]
#[specta::specta]
pub fn delete_history_entry(app: AppHandle, index: usize) -> Result<(), String> {
    History::delete_entry(index)?;
    crate::history::emit_changed(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn delete_history_audio(app: AppHandle, index: usize) -> Result<(), String> {
    History::delete_entry_audio(index)?;
    crate::history::emit_changed(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn clear_history(app: AppHandle) -> Result<(), String> {
    History::clear()?;
    crate::history::emit_changed(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_history_audio(index: usize) -> Result<Vec<u8>, String> {
    History::read_entry_audio_bytes(index)
}

#[tauri::command]
#[specta::specta]
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

    let text = History::reprocess(&app, index, &settings, None).await?;

    History::update_text(index, &text)?;
    crate::history::emit_changed(&app);

    Ok(json!({ "text": text, "index": index }))
}

fn show_and_focus_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    // On macOS the very first `set_focus` on a freshly-created window can be
    // dropped before the window is actually key ([activateAndOrderFront] has
    // nothing to activate yet). The window then appears but stays non-key, so
    // the user's first click is eaten purely to focus it — everything needs a
    // double-click until the window is re-opened. Re-assert focus a moment late.
    let w = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let _ = w.set_focus();
    });
    Ok(())
}

/// About is a page of the main SpeakType window, so its URL is the window's own
/// with the `#/about` route.
pub(crate) fn about_url(window_url: &tauri::Url) -> tauri::Url {
    main_window_url(window_url, Some("about"))
}

#[tauri::command]
#[specta::specta]
pub async fn open_about(app: AppHandle) -> Result<(), String> {
    let window = match app.get_webview_window("settings") {
        Some(window) => window,
        None => {
            open_settings(app.clone(), None).await?;
            app.get_webview_window("settings")
                .ok_or_else(|| "The SpeakType window did not open".to_string())?
        }
    };
    let url = about_url(&window.url().map_err(|e| e.to_string())?);
    window.navigate(url).map_err(|e| e.to_string())?;
    show_and_focus_window(&window)
}

const SETTINGS_WINDOW_W: f64 = 880.0;
const SETTINGS_WINDOW_H: f64 = 640.0;
const SETTINGS_WINDOW_MIN_W: f64 = 720.0;
const SETTINGS_WINDOW_MIN_H: f64 = 540.0;
const ONBOARDING_WINDOW_W: f64 = 960.0;
const ONBOARDING_WINDOW_H: f64 = 640.0;

/// Both document windows use macOS's overlay title bar with a hidden title, so
/// the sidebar (and onboarding's page) runs to the top of the window while the
/// traffic lights float over it.
#[cfg(target_os = "macos")]
fn hidden_title_bar(
    builder: WebviewWindowBuilder<'_, tauri::Wry, AppHandle>,
) -> WebviewWindowBuilder<'_, tauri::Wry, AppHandle> {
    builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
}

#[cfg(not(target_os = "macos"))]
fn hidden_title_bar(
    builder: WebviewWindowBuilder<'_, tauri::Wry, AppHandle>,
) -> WebviewWindowBuilder<'_, tauri::Wry, AppHandle> {
    builder
}

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
pub(crate) fn update_settings_window_geometry(window: &WebviewWindow) {
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

pub(crate) fn persist_settings_window_geometry(app: &AppHandle) {
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

/// The window's page for `section`; no section keeps the page the user is on.
fn main_window_url(window_url: &tauri::Url, section: Option<&str>) -> tauri::Url {
    let mut url = window_url.clone();
    if let Some(section) = section {
        url.set_fragment(Some(&format!("/{section}")));
    }
    url
}

/// The pages of the one SpeakType window (spec §6.3): History, General, …, About.
const MAIN_SECTIONS: &[&str] = &[
    "history",
    "general",
    "appearance",
    "dictation",
    "sessions",
    "providers",
    "permissions",
    "storage",
    "advanced",
    "about",
];

#[tauri::command]
#[specta::specta]
pub async fn open_settings(app: AppHandle, section: Option<String>) -> Result<(), String> {
    if let Some(section) = section.as_deref() {
        if !MAIN_SECTIONS.contains(&section) {
            return Err(format!("Unknown section: {section}"));
        }
    }
    if let Some(window) = app.get_webview_window("settings") {
        if let Some(section) = section.as_deref() {
            let url = main_window_url(&window.url().map_err(|e| e.to_string())?, Some(section));
            window.navigate(url).map_err(|e| e.to_string())?;
        }
        return show_and_focus_window(&window);
    }

    let (saved_x, saved_y, width, height) = settings_window_geometry(&app);

    let window = hidden_title_bar(WebviewWindowBuilder::new(
        &app,
        "settings",
        WebviewUrl::App("settings.html".into()),
    ))
    .title("SpeakType")
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
    if let Some(section) = section.as_deref() {
        let url = main_window_url(&window.url().map_err(|e| e.to_string())?, Some(section));
        window.navigate(url).map_err(|e| e.to_string())?;
    }
    show_and_focus_window(&window)
}

#[tauri::command]
#[specta::specta]
pub fn hide_widget(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn minimize_widget(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn quit_app(app: AppHandle) {
    if app.get_webview_window("settings").is_some() {
        persist_settings_window_geometry(&app);
    }
    app.exit(0);
}

#[tauri::command]
#[specta::specta]
pub async fn save_window_position(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let pos = window.outer_position().map_err(|e| e.to_string())?;
        let (cx, cy) = crate::window::clamp_to_visible_screens(&window, pos.x as f64, pos.y as f64);
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.window_x = Some(cx);
        settings.window_y = Some(cy);
        settings.widget_corner = crate::window::WidgetCorner::Custom.as_str().to_string();
        settings.save()?;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn reset_widget_position(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("Widget window not found".to_string());
    };

    crate::widget_cmds::apply_corner(
        &window,
        &state.settings,
        crate::window::WidgetCorner::BottomRight,
    )
    .map_err(|e| e.message)?;
    crate::widget_cmds::WidgetCornerChanged {
        corner: crate::window::WidgetCorner::BottomRight
            .as_str()
            .to_string(),
    }
    .emit(&app)
    .map_err(|e| e.to_string())?;

    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    let _ = window.set_always_on_top(true);

    let _ = window.emit("widget:highlight", json!({ "duration_ms": 2500 }));

    Ok(())
}

/// Latest debug sessions (newest first) captured by the Debug panel.
#[tauri::command]
#[specta::specta]
pub fn get_debug_sessions() -> Result<Vec<crate::debug::DebugSession>, String> {
    Ok(crate::debug::sessions())
}

#[tauri::command]
#[specta::specta]
pub fn clear_debug_sessions() -> Result<(), String> {
    crate::debug::clear();
    Ok(())
}

/// Remove a single debug session (and its persisted audio) by id.
#[tauri::command]
#[specta::specta]
pub fn delete_debug_session(session_id: u64) -> Result<(), String> {
    crate::debug::delete_session(session_id);
    Ok(())
}

/// Return the audio between `start_secs` and `end_secs` of a debug session's
/// recording as a mono WAV, for per-chunk/per-segment playback in the Debug
/// panel. Empty when the session kept no audio.
#[tauri::command]
#[specta::specta]
pub fn get_debug_audio_slice(
    session_id: u64,
    start_secs: f64,
    end_secs: f64,
) -> Result<Vec<u8>, String> {
    Ok(crate::debug::slice_audio(session_id, start_secs, end_secs).unwrap_or_default())
}

/// Reveal a file (e.g. an exported `.md` / `.docx`) in the file manager.
#[tauri::command]
#[specta::specta]
pub fn reveal_path(path: String) -> Result<(), String> {
    tauri_plugin_opener::reveal_item_in_dir(&path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_url_is_the_windows_route() {
        for (page, expected) in [
            (
                "http://tauri.localhost/settings.html",
                "http://tauri.localhost/settings.html#/about",
            ),
            (
                "http://localhost:5173/settings.html",
                "http://localhost:5173/settings.html#/about",
            ),
        ] {
            let url = about_url(&tauri::Url::parse(page).unwrap());
            assert_eq!(url.as_str(), expected);
            assert_eq!(url.fragment(), Some("/about"));
        }
    }

    #[test]
    fn open_settings_lands_on_the_section_it_is_given() {
        let page = tauri::Url::parse("http://tauri.localhost/settings.html").unwrap();
        assert_eq!(
            main_window_url(&page, Some("history")).as_str(),
            "http://tauri.localhost/settings.html#/history"
        );
        let elsewhere = tauri::Url::parse("http://tauri.localhost/settings.html#/general").unwrap();
        assert_eq!(
            main_window_url(&elsewhere, Some("history")).as_str(),
            "http://tauri.localhost/settings.html#/history"
        );
        assert_eq!(
            main_window_url(&elsewhere, None).as_str(),
            "http://tauri.localhost/settings.html#/general",
            "no section keeps the page the user is on"
        );
    }

    #[test]
    fn open_settings_sections_are_the_windows_pages() {
        for section in MAIN_SECTIONS {
            assert!(
                [
                    "history",
                    "general",
                    "appearance",
                    "dictation",
                    "sessions",
                    "providers",
                    "permissions",
                    "storage",
                    "advanced",
                    "about"
                ]
                .contains(section),
                "{section} is not a page of the SpeakType window"
            );
        }
        assert!(MAIN_SECTIONS.contains(&"history"));
    }

    #[test]
    fn about_url_replaces_another_route() {
        let url = tauri::Url::parse("http://tauri.localhost/settings.html#/general").unwrap();
        assert_eq!(
            about_url(&url).as_str(),
            "http://tauri.localhost/settings.html#/about"
        );
    }
}
