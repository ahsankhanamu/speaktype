mod audio;
mod chunk_session;
mod commands;
mod debug;
mod download_queue;
mod downloader;
mod format;
mod history;
mod hotkey;
mod logging;
mod paste;
mod paths;
mod permissions;
mod quality;
mod segmented_download;
mod server;
mod settings;
mod tones;
mod transcribe;
mod tray;
mod window;

use audio::{AudioRecorder, LevelMonitor};
use commands::AppState;
use settings::Settings;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Emitter, Listener, Manager, WindowEvent,
};
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn object_setClass(
        obj: *mut objc2_foundation::NSObject,
        cls: *const objc2::runtime::AnyClass,
    ) -> *const objc2::runtime::AnyClass;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .on_window_event(|window, event| {
            if window.label() != "onboarding" {
                return;
            }
            match event {
                WindowEvent::Moved(_) | WindowEvent::CloseRequested { .. } => {
                    if let Some(onboarding) = window.app_handle().get_webview_window("onboarding") {
                        commands::persist_onboarding_window_position(&onboarding);
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::toggle_recording,
            commands::cancel_recording,
            commands::check_permissions,
            commands::start_mic_test,
            commands::stop_mic_test,
            commands::get_audio_input_info,
            commands::get_input_devices,
            commands::log_frontend,
            commands::request_accessibility,
            commands::request_microphone_access,
            commands::get_onboarding_status,
            commands::open_onboarding,
            commands::finish_onboarding,
            commands::open_system_pane,
            commands::open_models_folder,
            commands::open_recordings_folder,
            commands::get_settings,
            commands::save_settings,
            commands::update_hotkey,
            commands::check_server,
            commands::get_server_status,
            commands::stop_whisper_server,
            commands::restart_whisper_server,
            commands::load_model,
            commands::queue_model_download,
            commands::restart_model_download,
            commands::cancel_model_download,
            commands::pause_model_download,
            commands::get_models,
            commands::get_model_progress,
            commands::open_about,
            commands::open_settings,
            commands::hide_widget,
            commands::minimize_widget,
            commands::quit_app,
            commands::save_window_position,
            commands::reset_widget_position,
            commands::get_history,
            commands::delete_history_entry,
            commands::delete_history_audio,
            commands::clear_history,
            commands::get_history_audio,
            commands::reprocess_history_entry,
            commands::get_debug_sessions,
            commands::clear_debug_sessions,
            commands::delete_debug_session,
            commands::get_debug_audio_slice,
        ])
        .setup(|app| {
            logging::init_logging();
            #[cfg(not(test))]
            debug::load_persisted();

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
                match (settings.window_x, settings.window_y) {
                    (Some(x), Some(y)) => {
                        let (cx, cy) = window::apply_saved_position(&window, x, y);
                        if (cx, cy) != (x, y) {
                            let mut corrected = settings.clone();
                            corrected.window_x = Some(cx);
                            corrected.window_y = Some(cy);
                            let _ = corrected.save();
                        }
                    }
                    _ => {
                        window::place_at_default_position(&window);
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
                let app_handle_for_device = app.handle().clone();
                std::thread::spawn(move || {
                    let mut last_name: Option<String> = None;
                    let mut last_count: usize = 0;
                    loop {
                        let current = audio::get_default_input_device_name();
                        let count = audio::count_input_devices();
                        if current != last_name || count != last_count {
                            let name = current.clone().unwrap_or_default();
                            logging::log_message(&format!(
                                "[audio] Device changed: {:?} (inputs={})",
                                name, count
                            ));
                            let _ = app_handle_for_device.emit(
                                "sidecar:device_changed",
                                serde_json::json!({ "name": name, "count": count }),
                            );
                            last_name = current;
                            last_count = count;
                        }
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                });
            }

            let app_state = AppState {
                settings: Arc::new(Mutex::new(settings.clone())),
                recorder: Arc::new(Mutex::new(recorder)),
                level_monitor: Arc::new(Mutex::new(LevelMonitor::new())),
                is_recording: is_recording.clone(),
                is_transcribing: is_transcribing.clone(),
                target_window: Arc::new(Mutex::new(None)),
                last_active_window,
                chunk_session: Arc::new(Mutex::new(None)),
            };

            app.manage(app_state);

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

            let hotkey_str = settings.hotkey.clone();
            let is_rec_for_shortcut = is_recording.clone();

            hotkey::register_app_hotkey(app.handle(), &hotkey_str, is_rec_for_shortcut);

            let record_item =
                MenuItemBuilder::with_id("record", "Start Recording").build(app)?;
            tray::set_record_menu_item(record_item.clone());
            let about_item =
                MenuItemBuilder::with_id("about", "About SpeakType").build(app)?;
            let settings_item =
                MenuItemBuilder::with_id("settings", "Settings").build(app)?;
            let show_item =
                MenuItemBuilder::with_id("show", "Hide Widget").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&record_item)
                .separator()
                .item(&settings_item)
                .item(&show_item)
                .separator()
                .item(&about_item)
                .item(&quit_item)
                .build()?;

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
                .title("ST")
                .menu(&menu)
                .tooltip("SpeakType")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "record" => {
                        let app_clone = app.clone();
                        let record_item = record_item.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_clone.state::<AppState>();
                            if let Err(e) = commands::toggle_recording_impl(
                                &app_clone,
                                &state,
                            ) {
                                logging::log_message(&format!(
                                    "[tray] toggle_recording error: {}",
                                    e
                                ));
                            }
                            let is_rec = app_clone.state::<AppState>().is_recording.load(Ordering::SeqCst);
                            let _ = record_item.set_text(if is_rec { "Stop Recording" } else { "Start Recording" });
                        });
                    }
                    "about" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = commands::open_about(app).await;
                        });
                    }
                    "settings" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = commands::open_settings(app).await;
                        });
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                                let _ = show_item.set_text("Show Widget");
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = show_item.set_text("Hide Widget");
                            }
                        }
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
            match event {
                tauri::RunEvent::Exit => {
                    commands::stop_mic_test_for_app(app, "app_exit");
                    server::stop_server();
                }
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
                        let _ = commands::open_settings(app_clone).await;
                    });
                }
                _ => {}
            }
        });
}
