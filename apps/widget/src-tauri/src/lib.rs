mod audio;
mod capture;
mod chunk_session;
mod clipboard;
mod commands;
mod controller;
mod debug;
mod download_queue;
mod downloader;
mod format;
mod history;
mod hotkey;
mod ipc_export;
// Kept compiled (spec §10: "llm.rs, llm_server.rs and templates.rs stay
// compiled behind Advanced for the later cleanup-modes feature"); their Scribe
// UI entry points were removed in Phase 0, so nothing calls them yet.
#[allow(dead_code)]
mod llm;
#[allow(dead_code)]
mod llm_server;
mod logging;
mod notice;
mod ocr;
mod paste;
mod paths;
mod permissions;
mod quality;
mod segmented_download;
mod server;
mod session_cmds;
mod settings;
mod shortcuts;
mod stt_cmds;
#[allow(dead_code)]
mod templates;
mod tones;
mod transcribe;
mod tray;
mod widget_cmds;
mod widget_frame;
mod window;

use audio::{AudioRecorder, LevelMonitor};
use commands::AppState;
use settings::Settings;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Emitter, Listener, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn object_setClass(
        obj: *mut objc2_foundation::NSObject,
        cls: *const objc2::runtime::AnyClass,
    ) -> *const objc2::runtime::AnyClass;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = ipc_export::builder();
    let session_muted: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .on_window_event(|window, event| {
            match (window.label(), event) {
                ("settings", WindowEvent::Moved(_))
                | ("settings", WindowEvent::Resized(_)) => {
                    if let Some(settings) = window.app_handle().get_webview_window("settings") {
                        commands::update_settings_window_geometry(&settings);
                    }
                }
                ("settings", WindowEvent::CloseRequested { .. })
                | ("settings", WindowEvent::Destroyed) => {
                    commands::persist_settings_window_geometry(window.app_handle());
                    commands::stop_mic_test_for_app(window.app_handle(), "window_closed");
                }
                ("main", WindowEvent::Moved(position)) => {
                    if let Some(widget) = window.app_handle().get_webview_window("main") {
                        if let Some(saved) = widget_frame::on_moved(&widget, *position) {
                            widget_cmds::save_dragged_position(&widget, saved);
                        }
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(specta_builder.invoke_handler())
        .on_page_load(|window, _payload| {
            if window.label() == "main" {
                commands::replay_dictation_state(window.app_handle());
            }
        })
        .setup(move |app| {
            logging::init_logging();
            #[cfg(not(test))]
            debug::load_persisted();

            specta_builder.mount_events(app);

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Regular);

            {
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    let mut last: Option<(bool, bool)> = None;
                    let mut onboarding_opened = false;

                    loop {
                        let has_mic = permissions::check_microphone();
                        let has_acc = permissions::check_accessibility(false);

                        if accessibility_became_granted(last.map(|(_, acc)| acc), has_acc) {
                            let state = app_handle.state::<commands::AppState>();
                            let keys = state
                                .settings
                                .lock()
                                .map(|s| s.shortcut(shortcuts::ShortcutAction::Dictation))
                                .unwrap_or_default();
                            hotkey::rearm_dictation(
                                &hotkey::AppShortcuts {
                                    app: app_handle.clone(),
                                    is_recording: state.is_recording.clone(),
                                },
                                &keys,
                            );
                            logging::log_message(
                                "[permissions] Accessibility granted — hotkey re-armed",
                            );
                        }

                        if last != Some((has_mic, has_acc)) {
                            last = Some((has_mic, has_acc));
                            if has_acc {
                                logging::log_message(&format!(
                                    "[permissions] microphone={} accessibility=true",
                                    has_mic
                                ));
                            } else {
                                logging::log_message(&format!(
                                    "[permissions] microphone={} accessibility=false exe={}",
                                    has_mic,
                                    permissions::accessibility_exe_path()
                                ));
                                let owner = permissions::responsible_app_name();
                                if permissions::is_dev_binary() && !permissions::is_bundled() {
                                    logging::log_message(&format!(
                                        "[permissions] This is a bare executable, so macOS attributes Accessibility \
                                         to the app that launched it ({}) rather than to SpeakType. Run `make dev` \
                                         instead — it launches a real SpeakType Dev.app that can hold its own grant.",
                                        owner.as_deref().unwrap_or("your terminal app")
                                    ));
                                } else if let Some(owner) = owner {
                                    logging::log_message(&format!(
                                        "[permissions] Enable \"{}\" under Privacy & Security → Accessibility, \
                                         then Quit & Reopen. If an old entry with the same name is present, \
                                         remove it first — stale entries point at a different signature.",
                                        owner
                                    ));
                                } else {
                                    logging::log_message(
                                        "[permissions] If System Settings shows SpeakType ON but this is still false: \
                                         the toggle is for a different binary/signature. Remove stale SpeakType entries \
                                         from Accessibility, quit ALL SpeakType processes, enable this install, \
                                         then Quit & Reopen once.",
                                    );
                                }
                            }
                            commands::emit_permissions(&app_handle);
                        }

                        if !onboarding_opened {
                            onboarding_opened = true;
                            let model = Settings::load().model;
                            let model_ok = crate::downloader::is_model_complete(&model);
                            if !(has_mic && has_acc && model_ok) {
                                logging::log_message(&format!(
                                    "[onboarding] Requirements unmet (mic={}, acc={}, model={}) — opening onboarding",
                                    has_mic, has_acc, model_ok
                                ));
                                std::thread::sleep(std::time::Duration::from_millis(400));
                                if let Err(e) = commands::open_onboarding_window(&app_handle) {
                                    logging::log_message(&format!("[onboarding] Failed to open window: {}", e));
                                }
                            }
                        }

                        std::thread::sleep(Duration::from_secs(1));
                    }
                });
            }

            let settings = Settings::load();

            if let Some(window) = app.get_webview_window("main") {
                let at_corner = match window::WidgetCorner::parse(&settings.widget_corner) {
                    Some(corner) => window::place_at_corner(&window, corner),
                    None => None,
                };
                if at_corner.is_none() {
                    match (settings.window_x, settings.window_y) {
                        (Some(x), Some(y)) => {
                            let (cx, cy) = window::apply_saved_position(&window, x, y);
                            if (cx, cy) != (x, y) {
                                let mut corrected = settings.clone();
                                corrected.window_x = Some(cx);
                                corrected.window_y = Some(cy);
                                if let Err(e) = corrected.save() {
                                    logging::log_message(&format!(
                                        "[window] Failed to save the corrected position: {}",
                                        e
                                    ));
                                }
                            }
                        }
                        _ => {
                            window::place_at_default_position(&window);
                        }
                    }
                }
            }

            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.with_webview(|webview| {
                    unsafe {
                        use objc2::runtime::AnyClass;
                        use objc2_app_kit::{NSPanel, NSWindowStyleMask};
                        use objc2_foundation::NSObject;

                        let ns_win_ptr = webview.ns_window();

                        let panel_class = match AnyClass::get(c"NSPanel") {
                            Some(cls) => cls,
                            None => {
                                logging::log_message("[panel] NSPanel class not found, skipping panel conversion");
                                return;
                            }
                        };
                        object_setClass(ns_win_ptr as *mut NSObject, panel_class);

                        let panel = &*(ns_win_ptr as *const NSPanel);
                        let mask = NSWindowStyleMask::Borderless
                            | NSWindowStyleMask::NonactivatingPanel;
                        panel.setStyleMask(mask);
                        panel.setFloatingPanel(true);
                        panel.setBecomesKeyOnlyIfNeeded(true);
                        panel.setHasShadow(false);

                        logging::log_message("[panel] Widget set as non-activating panel");
                    }
                });
            }

            let recorder = AudioRecorder::new()?;

            let is_recording = Arc::new(AtomicBool::new(false));
            let is_transcribing = Arc::new(AtomicBool::new(false));
            let last_active_window = Arc::new(Mutex::new(None));

            {
                let tracker = last_active_window.clone();
                std::thread::spawn(move || loop {
                    if let Some(win) = paste::get_frontmost_window() {
                        if let Ok(mut stored) = tracker.lock() {
                            *stored = Some(win);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(300));
                });
            }

            {
                widget_cmds::spawn_device_watcher(app.handle().clone());
            }

            // The session store (v2 schema; a pre-2 dev DB is reset by design,
            // spec §10). Managed on its own so every surface — commands, the
            // clipboard monitor, the screenshot hotkey — reaches the same one.
            let session_store = Arc::new(
                speaktype_session::Store::open(&paths::config_dir().join("scribe.db"))
                    .map_err(|e| e.to_string())?,
            );
            app.manage(session_muted.clone());
            app.manage(session_store.clone());

            let levels_ui = commands::levels_ui(app.handle());
            let app_state = AppState {
                settings: Arc::new(Mutex::new(settings.clone())),
                recorder: Arc::new(Mutex::new(recorder)),
                level_monitor: Arc::new(Mutex::new(LevelMonitor::new())),
                mic_levels: Arc::new(Mutex::new(audio::DeviceLevels::new(levels_ui))),
                dictation: Arc::new(Mutex::new(controller::idle())),
                last_dictation: Arc::new(Mutex::new(None)),
                model_load_failed: Arc::new(AtomicBool::new(false)),
                is_recording: is_recording.clone(),
                is_transcribing: is_transcribing.clone(),
                target_window: Arc::new(Mutex::new(None)),
                last_active_window,
                chunk_session: Arc::new(Mutex::new(None)),
                is_pasting: Arc::new(AtomicBool::new(false)),
                shortcuts: Arc::new(Mutex::new(shortcuts::LiveShortcuts::default())),
            };

            app.manage(app_state);

            app.manage(EscHolder(std::sync::Mutex::new(hotkey::EscShortcut::new(
                hotkey::EscRegistration {
                    app: app.handle().clone(),
                },
                is_recording.clone(),
            ))));

            if server::has_embedded_server(app.handle()) {
                let _ = app.emit("server:starting", serde_json::json!({}));

                if let Some(info) = server::start_server(app.handle(), &settings.model) {
                    let app_handle_for_server = app.handle().clone();
                    let port = info.port;
                    let model = info.model;

                    // Auto-restart server if it crashes unexpectedly (with backoff + cap)
                    let app_for_crash = app.handle().clone();
                    app.handle().listen("server:crash", move |_event| {
                        if !server::should_auto_restart() {
                            logging::log_message("[server] Crash restart limit reached — giving up");
                            let _ = app_for_crash.emit(
                                "server:error",
                                serde_json::json!({ "reason": "Server crashed repeatedly. Check Settings → Model." }),
                            );
                            return;
                        }

                        logging::log_message("[server] Crash detected, auto-restarting...");
                        let app = app_for_crash.clone();
                        tauri::async_runtime::spawn(async move {
                            server::stop_server();
                            tokio::time::sleep(Duration::from_millis(1500)).await;
                            let model = {
                                let state = app.state::<AppState>();
                                state.settings.lock().ok().map(|s| s.model.clone()).unwrap_or_default()
                            };
                            if let Some(info) = server::start_server(&app, &model) {
                                if server::wait_for_server(info.port).await {
                                    let state = app.state::<AppState>();
                                    if let Ok(mut settings) = state.settings.lock() {
                                        settings.api_url = format!("http://127.0.0.1:{}/inference", info.port);
                                        settings.model = info.model.clone();
                                        if let Err(e) = settings.save() {
                                            logging::log_message(&format!("[server] Failed to save settings: {}", e));
                                        }
                                    }
                                    logging::log_message(&format!("[server] Auto-restart succeeded on port {}", info.port));
                                    let _ = app.emit("server:ready", serde_json::json!({
                                        "port": info.port,
                                        "model": info.model,
                                    }));
                                }
                            }
                        });
                    });

                    tauri::async_runtime::spawn(async move {
                        if server::wait_for_server(port).await {
                            let state = app_handle_for_server.state::<AppState>();
                            if let Ok(mut settings) = state.settings.lock() {
                                settings.api_url = format!("http://127.0.0.1:{}/inference", port);
                                settings.model = model.clone();
                                if let Err(e) = settings.save() {
                                    logging::log_message(&format!("[server] Failed to save settings: {}", e));
                                } else {
                                    logging::log_message(&format!(
                                        "[server] Settings saved: api_url={}, model={}",
                                        settings.api_url, settings.model
                                    ));
                                }
                            }
                            let _ = app_handle_for_server.emit(
                                "server:ready",
                                serde_json::json!({ "port": port, "model": model }),
                            );
                        } else {
                            server::stop_server();
                            let _ = app_handle_for_server.emit(
                                "server:error",
                                serde_json::json!({ "reason": "Server failed to start within timeout" }),
                            );
                        }
                    });
                } else {
                    let _ = app.emit(
                        "server:error",
                        serde_json::json!({ "reason": "Failed to spawn server process" }),
                    );
                }
            } else {
                logging::log_message("[server] No embedded server found, using external API");
            }

            let is_rec_for_shortcut = is_recording.clone();

            {
                let platform = hotkey::AppShortcuts {
                    app: app.handle().clone(),
                    is_recording: is_rec_for_shortcut,
                };
                let state = app.state::<AppState>();
                let mut live = state.shortcuts.lock().map_err(|e| e.to_string())?;
                for (action, keys) in &settings.shortcuts {
                    // The draw key is a held modifier owned by the overlay's key tap (Phase 2).
                    if *action == shortcuts::ShortcutAction::DrawKey {
                        continue;
                    }
                    if !shortcuts::ShortcutsPlatform::arm(&platform, *action, keys) {
                        logging::log_message(&format!(
                            "[shortcut] Could not arm {:?} {:?} at startup",
                            action, keys
                        ));
                        continue;
                    }
                    if !shortcuts::uses_modifier_tap(*action, keys) {
                        live.set_armed(*action, keys);
                    }
                }
            }

            clipboard::spawn_clipboard_monitor(app.handle().clone());

            let record_item =
                MenuItemBuilder::with_id("record", "Start Recording").build(app)?;
            tray::set_record_menu_item(record_item.clone());
            let paste_item =
                MenuItemBuilder::with_id("paste", "Paste last dictation again").build(app)?;
            let unload_item = MenuItemBuilder::with_id("unload", tray::NOT_LOADED)
                .enabled(false)
                .build(app)?;
            tray::set_model_menu_item(unload_item.clone());
            let history_item =
                MenuItemBuilder::with_id("history", "History…").build(app)?;
            let settings_item =
                MenuItemBuilder::with_id("settings", "Settings…").build(app)?;
            let show_item =
                MenuItemBuilder::with_id("show", "Hide Widget").build(app)?;
            let about_item =
                MenuItemBuilder::with_id("about", "About SpeakType").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit SpeakType").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&record_item)
                .separator()
                .item(&paste_item)
                .item(&unload_item)
                .item(&history_item)
                .separator()
                .item(&settings_item)
                .item(&show_item)
                .separator()
                .item(&about_item)
                .item(&quit_item)
                .build()?;
            // The model item follows the server: Unload while the model sits in
            // memory, "Speech model not loaded" when nothing is loaded.
            for event in ["server:ready", "server:starting", "server:error", "server:stopped"] {
                let tray_app = app.handle().clone();
                app.handle().listen(event, move |_| tray::refresh_model_item(&tray_app));
            }
            tray::refresh_model_item(app.handle());

            let tray_icon = match tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png")) {
                Ok(img) => img,
                Err(e) => {
                    logging::log_message(&format!("[tray] Failed to load icon: {}", e));
                    return Err(Box::new(e));
                }
            };
            let _tray = TrayIconBuilder::with_id(tray::TRAY_ID)
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .tooltip("SpeakType")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "record" => commands::toggle_from_tray(app.clone(), record_item.clone()),
                    "paste" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app.state::<AppState>();
                            if let Err(e) = commands::paste_last(app.clone(), state).await {
                                logging::log_message(&format!("[tray] Nothing to paste: {e}"));
                            }
                        });
                    }
                    "unload" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = commands::unload_model(app.clone()).await {
                                logging::log_message(&format!("[tray] Could not unload: {e}"));
                            }
                            tray::refresh_model_item(&app);
                        });
                    }
                    "about" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = commands::open_about(app).await {
                                logging::log_message(&format!(
                                    "[tray] Could not open About: {}",
                                    e
                                ));
                            }
                        });
                    }
                    "history" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) =
                                commands::open_settings(app, Some("history".to_string())).await
                            {
                                logging::log_message(&format!(
                                    "[tray] Could not open History: {}",
                                    e
                                ));
                            }
                        });
                    }
                    "settings" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = commands::open_settings(app, None).await {
                                logging::log_message(&format!("[tray] Could not open Settings: {e}"));
                            }
                        });
                    }
                    "show" => {
                        let app = app.clone();
                        let show_item = show_item.clone();
                        tauri::async_runtime::spawn(async move {
                            let Some(window) = app.get_webview_window("main") else {
                                return;
                            };
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                                let _ = show_item.set_text("Show Widget");
                            } else if let Err(e) =
                                widget_cmds::show_widget_at(&app, window::WidgetCorner::BottomRight)
                            {
                                logging::log_message(&format!(
                                    "[tray] Could not show the widget: {}",
                                    e
                                ));
                            } else {
                                let _ = show_item.set_text("Hide Widget");
                            }
                        });
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            logging::log_message("[tray] Menu bar icon created (id=speaktype-tray)");

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building SpeakType")
        .run(|app, event| {
            if let tauri::RunEvent::Ready = event {
                handle_termination_signals(app);
            }
            match event {
                tauri::RunEvent::Exit => {
                    // The one app-wide wipe: every registration is about to die
                    // with the process, and a partial registry cannot survive.
                    if let Err(e) = app.global_shortcut().unregister_all() {
                        logging::log_message(&format!("[shortcut] Exit wipe failed: {e}"));
                    }
                    commands::stop_mic_test_for_app(app, "app_exit");
                    commands::stop_mic_levels(app);
                    server::stop_server();
                    llm_server::stop_llm_server();
                }
                // A normal walk through the main thread (⌘Q, the tray's Quit)
                // still ends with the server stopped. `Exit` alone is not
                // enough for a signal that unwinds through AppKit.
                tauri::RunEvent::ExitRequested { .. } => server::stop_server(),
                // Clicking the dock icon: reveal the floating widget and open
                // the settings window so the user always has a way back in.
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Reopen { .. } => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_always_on_top(true);
                        let _ = window.set_focus();
                    }
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = commands::open_settings(app_clone, None).await {
                            logging::log_message(&format!("[dock] Could not open Settings: {e}"));
                        }
                    });
                }
                _ => {}
            }
        });
}

/// SIGTERM, SIGINT and SIGHUP (logout, `kill`, a dev rebuild's parent) must
/// stop the server before the process goes.
///
/// A signal handler may only call async-signal-safe functions, and Tauri's
/// `exit()` is not one. So the handler only forwards the signal to the main
/// thread, which stops the server and asks AppKit to terminate normally. Should
/// that not end the process within two seconds, it stops the server anyway and
/// exits outright, so a server never outlives us.
#[cfg(unix)]
fn handle_termination_signals(app: &tauri::AppHandle) {
    use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGTERM};

    let signals = signal_hook::iterator::Signals::new([SIGTERM, SIGINT, SIGHUP]);
    let mut signals = match signals {
        Ok(signals) => signals,
        Err(e) => {
            logging::log_message(&format!("[signals] Could not watch for termination: {e}"));
            return;
        }
    };
    let handle = app.clone();
    MAIN_APP.get_or_init(|| handle);
    std::thread::spawn(move || {
        if let Some(signal) = signals.forever().next() {
            logging::log_message(&format!(
                "[signals] Received signal {signal} — stopping the server and quitting"
            ));
            server::stop_server();
            terminate_in_place();
            std::thread::sleep(Duration::from_secs(2));
            unsafe { libc::_exit(0) };
        }
    });
}

/// The app's handle for the signal thread, the one place without one.
static MAIN_APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

#[cfg(unix)]
fn terminate_in_place() {
    let Some(app) = MAIN_APP.get() else {
        return;
    };
    let queued = app.run_on_main_thread(move || {
        #[cfg(target_os = "macos")]
        {
            use objc2::MainThreadMarker;
            use objc2_app_kit::NSApplication;
            let main = MainThreadMarker::new().expect("run_on_main_thread runs on the main thread");
            NSApplication::sharedApplication(main).terminate(None);
        }
        #[cfg(not(target_os = "macos"))]
        app.exit(0);
    });
    if queued.is_err() {
        app.exit(0);
    }
}

/// The temporary Esc shortcut, owned by the app so `remember_dictation` can
/// arm and release it from the phase it just published.
struct EscHolder(std::sync::Mutex<hotkey::EscShortcut<hotkey::EscRegistration>>);

/// Accessibility granted while running must arm the dictation hotkey at once:
/// the Super+Control tap only exists once macOS allows event taps. The first
/// poll has no previous value; startup already registered the hotkey.
fn accessibility_became_granted(prev: Option<bool>, now: bool) -> bool {
    prev == Some(false) && now
}

// IPC command surface contract.
//
// The Svelte frontend calls commands by (snake_case) name via `src/lib/ipc.ts`.
// If a command is renamed, removed, or its registration is dropped here, this
// test fails to compile AND the runtime set assertion fails — guarding against
// the single biggest "don't break the UI" risk.
#[cfg(test)]
mod ipc_surface {
    use super::accessibility_became_granted;

    // Exactly the set registered in `invoke_handler(generate_handler![...])`.
    // Keeping the two in sync is intentional and enforced by this test.
    const REGISTERED: &[&str] = &[
        "toggle_recording",
        "cancel_recording",
        "check_permissions",
        "start_mic_test",
        "stop_mic_test",
        "get_audio_input_info",
        "get_input_devices",
        "log_frontend",
        "copy_to_clipboard",
        "request_accessibility",
        "request_microphone_access",
        "get_onboarding_status",
        "open_onboarding",
        "finish_onboarding",
        "open_system_pane",
        "open_models_folder",
        "open_recordings_folder",
        "get_settings",
        "save_settings",
        "set_launch_at_login",
        "shortcuts_get",
        "shortcut_set",
        "shortcuts_reset",
        "check_server",
        "get_server_status",
        "stop_whisper_server",
        "unload_model",
        "load_server",
        "server_info",
        "restart_whisper_server",
        "load_model",
        "queue_model_download",
        "restart_model_download",
        "cancel_model_download",
        "pause_model_download",
        "get_models",
        "get_model_progress",
        "open_about",
        "open_settings",
        "hide_widget",
        "minimize_widget",
        "quit_app",
        "save_window_position",
        "layout_widget",
        "reset_widget_position",
        "get_history",
        "delete_history_entry",
        "delete_history_audio",
        "clear_history",
        "get_history_audio",
        "reprocess_history_entry",
        "paste_last",
        "delete_model",
        "mic_levels_start",
        "mic_levels_stop",
        "get_debug_sessions",
        "clear_debug_sessions",
        "delete_debug_session",
        "get_debug_audio_slice",
        "reveal_path",
        "set_widget_corner",
        "screen_recording_status",
        "request_screen_recording",
        "test_typing",
        "reprocess_preview",
        "session_start",
        "session_stop",
        "session_active",
        "session_set_mute",
        "session_list",
        "session_get",
        "session_rename",
        "session_delete",
        "item_update",
        "item_delete",
        "item_restore",
        "item_move",
        "item_split",
        "prompt_render",
        "prompt_copy",
        "bundle_reveal",
        "overlay_pen",
        "overlay_mark_complete",
    ];

    // The frontend `ipc.ts` surface (mirrors src/settings/../lib/ipc.ts).
    // Every command the UI calls must exist in REGISTERED.
    const FRONTEND_USED: &[&str] = &[
        "log_frontend",
        "copy_to_clipboard",
        "toggle_recording",
        "cancel_recording",
        "hide_widget",
        "minimize_widget",
        "quit_app",
        "save_window_position",
        "layout_widget",
        "reset_widget_position",
        "open_about",
        "open_settings",
        "open_onboarding",
        "finish_onboarding",
        "get_onboarding_status",
        "request_microphone_access",
        "get_settings",
        "save_settings",
        "set_launch_at_login",
        "shortcuts_get",
        "shortcut_set",
        "shortcuts_reset",
        "check_server",
        "get_server_status",
        "stop_whisper_server",
        "unload_model",
        "load_server",
        "server_info",
        "restart_whisper_server",
        "get_history",
        "delete_history_entry",
        "delete_history_audio",
        "clear_history",
        "reprocess_history_entry",
        "get_history_audio",
        "get_debug_sessions",
        "clear_debug_sessions",
        "delete_debug_session",
        "get_debug_audio_slice",
        "check_permissions",
        "request_accessibility",
        "start_mic_test",
        "stop_mic_test",
        "get_audio_input_info",
        "get_input_devices",
        "open_system_pane",
        "open_models_folder",
        "open_recordings_folder",
        "load_model",
        "queue_model_download",
        "restart_model_download",
        "get_models",
        "get_model_progress",
        "cancel_model_download",
        "pause_model_download",
        "reveal_path",
        "set_widget_corner",
        "screen_recording_status",
        "request_screen_recording",
        "test_typing",
        "reprocess_preview",
        "session_start",
        "session_stop",
        "session_active",
        "session_set_mute",
        "session_list",
        "session_get",
        "session_rename",
        "session_delete",
        "item_update",
        "item_delete",
        "item_restore",
        "item_move",
        "item_split",
        "prompt_render",
        "prompt_copy",
        "bundle_reveal",
        "overlay_pen",
        "overlay_mark_complete",
    ];

    #[test]
    fn the_widget_window_opens_at_the_idle_pill_plus_its_halo_margin() {
        let config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json"),
            )
            .expect("tauri.conf.json"),
        )
        .expect("valid JSON");
        let window = &config["app"]["windows"][0];
        assert_eq!(window["label"], "main");
        // WindowFit: a 44 pt idle pill inside a 24 pt margin on every side.
        assert_eq!(window["width"], 92);
        assert_eq!(window["height"], 92);
    }

    #[test]
    fn accessibility_became_granted_only_on_the_false_to_true_flip() {
        assert!(accessibility_became_granted(Some(false), true));
        assert!(!accessibility_became_granted(Some(true), true));
        assert!(!accessibility_became_granted(Some(false), false));
        assert!(!accessibility_became_granted(Some(true), false));
        assert!(
            !accessibility_became_granted(None, true),
            "the first poll is not a grant"
        );
    }

    #[test]
    fn every_frontend_command_is_registered() {
        let missing: Vec<&str> = FRONTEND_USED
            .iter()
            .copied()
            .filter(|name| !REGISTERED.contains(name))
            .collect();
        assert!(
            missing.is_empty(),
            "frontend calls commands not registered in invoke_handler: {:?}",
            missing
        );
    }

    /// Event names a source file emits: quoted literals starting with an
    /// event prefix, so `log_message` prose about the sidecar is not counted.
    fn emitted_event_names(text: &str) -> Vec<String> {
        const PREFIXES: &[&str] = &[
            concat!("si", "decar:"),
            concat!("ser", "ver:"),
            concat!("dicta", "tion:"),
            concat!("onboar", "ding:"),
            concat!("mic-devices-", "changed"),
        ];
        text.split('"')
            .skip(1)
            .step_by(2)
            .filter(|literal| PREFIXES.iter().any(|prefix| literal.starts_with(prefix)))
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn removed_event_names_are_recognised_as_events() {
        // Built at run time: the literal would otherwise count as an emit.
        let speech = ["sidecar", "no_speech"].join(":");
        let text = format!(
            "app.emit(\"{speech}\", json!({{}})); log_message(\"Started sidecar: whisper-server\");"
        );
        assert_eq!(emitted_event_names(&text), vec![speech.to_string()]);
        assert!(emitted_event_names("fn open_settings() {}").is_empty());
    }

    #[test]
    fn only_the_events_the_new_widget_listens_to_are_emitted() {
        const KEPT: &[&str] = &[
            "dictation:long-audio",
            "onboarding:finished",
            "onboarding:opened",
            "server:crash",
            "server:error",
            "server:model_needed",
            "server:ready",
            "server:starting",
            "server:stopped",
            "sidecar:audio_level",
            "sidecar:permissions",
            "mic-devices-changed",
        ];
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut leftovers = Vec::new();
        for entry in std::fs::read_dir(src).unwrap() {
            let path = entry.unwrap().path();
            if path.extension() != Some("rs".as_ref()) {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for name in emitted_event_names(&text) {
                if !KEPT.contains(&name.as_str()) {
                    leftovers.push(format!("{}: {name}", path.display()));
                }
            }
        }
        assert!(
            leftovers.is_empty(),
            "events the new widget does not listen to are still emitted: {leftovers:#?}"
        );
    }

    #[test]
    fn open_settings_is_registered_with_its_section_argument() {
        let bindings = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../webui/src/lib/bindings.ts"),
        )
        .expect("bindings");
        assert!(
            bindings.contains("openSettings: (section: string | null"),
            "bindings.ts must take the section open_settings navigates to"
        );
    }

    #[test]
    fn registered_sets_have_no_duplicates() {
        let mut sorted = REGISTERED.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            REGISTERED.len(),
            "duplicate command registration"
        );
    }
}
