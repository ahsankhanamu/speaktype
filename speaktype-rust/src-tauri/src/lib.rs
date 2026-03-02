mod audio;
mod commands;
mod history;
mod logging;
mod paste;
mod permissions;
mod settings;
mod tones;
mod transcribe;
mod tray;

use audio::AudioRecorder;
use commands::AppState;
use settings::Settings;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::toggle_recording,
            commands::check_permissions,
            commands::request_accessibility,
            commands::get_settings,
            commands::save_settings,
            commands::update_hotkey,
            commands::check_server,
            commands::open_about,
            commands::open_settings,
            commands::hide_widget,
            commands::minimize_widget,
            commands::quit_app,
            commands::save_window_position,
            commands::get_history,
            commands::delete_history_entry,
            commands::clear_history,
        ])
        .setup(|app| {
            logging::init_logging();

            // Hide from dock — widget lives in tray only
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Check macOS permissions at startup
            let has_accessibility = permissions::check_accessibility(true);
            let has_microphone = permissions::request_microphone();
            logging::log_message(&format!(
                "[permissions] accessibility={}, microphone={}",
                has_accessibility, has_microphone
            ));

            let settings = Settings::load();

            // Restore window position
            if let (Some(x), Some(y)) = (settings.window_x, settings.window_y) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_position(tauri::Position::Physical(
                        tauri::PhysicalPosition {
                            x: x as i32,
                            y: y as i32,
                        },
                    ));
                }
            }

            // Make widget a non-activating panel so clicking it
            // never steals focus from the user's active app
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.with_webview(|webview| {
                    unsafe {
                        use objc2::runtime::AnyClass;
                        use objc2_app_kit::{NSPanel, NSWindowStyleMask};
                        use objc2_foundation::NSObject;

                        unsafe extern "C" {
                            fn object_setClass(
                                obj: *mut NSObject,
                                cls: *const AnyClass,
                            ) -> *const AnyClass;
                        }

                        let ns_win_ptr = webview.ns_window();

                        // Swizzle the NSWindow to NSPanel
                        let panel_class = AnyClass::get(c"NSPanel").unwrap();
                        object_setClass(ns_win_ptr as *mut NSObject, panel_class);

                        // Configure as non-activating floating panel
                        let panel = &*(ns_win_ptr as *const NSPanel);
                        let mask = NSWindowStyleMask::Borderless
                            | NSWindowStyleMask::NonactivatingPanel;
                        panel.setStyleMask(mask);
                        panel.setFloatingPanel(true);
                        panel.setBecomesKeyOnlyIfNeeded(true);

                        logging::log_message("[panel] Widget set as non-activating panel");
                    }
                });
            }

            // Create audio recorder
            let recorder = AudioRecorder::new()?;

            let is_recording = Arc::new(AtomicBool::new(false));
            let last_active_window = Arc::new(Mutex::new(None));

            // Background thread: track the last focused non-SpeakType app.
            // This runs every 300ms so we always know where to paste,
            // even if the widget stole focus via a click.
            {
                let tracker = last_active_window.clone();
                std::thread::spawn(move || loop {
                    if let Some(win) = paste::get_frontmost_window() {
                        let mut stored = tracker.lock().unwrap();
                        *stored = Some(win);
                    }
                    // None means SpeakType is frontmost — keep previous value
                    std::thread::sleep(std::time::Duration::from_millis(300));
                });
            }

            let app_state = AppState {
                settings: Arc::new(Mutex::new(settings.clone())),
                recorder: Arc::new(Mutex::new(recorder)),
                is_recording: is_recording.clone(),
                target_window: Arc::new(Mutex::new(None)),
                last_active_window,
            };

            app.manage(app_state);

            // Register global shortcut
            let hotkey_str = settings.hotkey.clone();
            let app_handle = app.handle().clone();
            let is_rec_for_shortcut = is_recording.clone();

            if let Ok(shortcut) = hotkey_str.parse::<Shortcut>() {
                app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
                    // Only handle key-down, ignore key-up to prevent double-fire
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let currently_recording = is_rec_for_shortcut.load(Ordering::SeqCst);

                    // Call toggle_recording via the app handle
                    let app_clone = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // Invoke the toggle_recording command logic directly
                        let state = app_clone.state::<AppState>();
                        if let Err(e) = commands::toggle_recording(
                            app_clone.clone(),
                            state,
                            currently_recording,
                        ) {
                            logging::log_message(&format!(
                                "[shortcut] toggle_recording error: {}",
                                e
                            ));
                        }
                    });
                })?;
                logging::log_message(&format!("[shortcut] Registered: {}", hotkey_str));
            } else {
                logging::log_message(&format!("[shortcut] Invalid hotkey: {}", hotkey_str));
            }

            // System tray
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

            let _tray = TrayIconBuilder::with_id(tray::TRAY_ID)
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png")).expect("failed to load tray icon"))
                .icon_as_template(false)
                .menu(&menu)
                .tooltip("SpeakType")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "record" => {
                        let currently_recording = is_recording.load(Ordering::SeqCst);
                        let app_clone = app.clone();
                        let record_item = record_item.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_clone.state::<AppState>();
                            if let Err(e) = commands::toggle_recording(
                                app_clone.clone(),
                                state,
                                currently_recording,
                            ) {
                                logging::log_message(&format!(
                                    "[tray] toggle_recording error: {}",
                                    e
                                ));
                            }
                            // Update menu label
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running SpeakType");
}
