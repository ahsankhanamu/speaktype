mod commands;
mod history;
mod permissions;
mod settings;
mod sidecar;

use commands::{AppSettings, SidecarStdin};
use settings::Settings;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Listener, Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::toggle_recording,
            commands::send_sidecar_command,
            commands::get_settings,
            commands::save_settings,
            commands::update_hotkey,
            commands::check_server,
            commands::open_settings,
            commands::hide_widget,
            commands::minimize_widget,
            commands::quit_app,
            commands::save_window_position,
            commands::get_history,
            commands::clear_history,
        ])
        .setup(|app| {
            sidecar::init_logging();

            // Check macOS permissions at startup — prompts user if not granted
            let has_accessibility = permissions::check_accessibility(true);
            let has_microphone = permissions::request_microphone();
            sidecar::log_message(&format!(
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

            // Spawn Python sidecar
            let script_path = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .map(|p| {
                    // In dev mode, the exe is in target/debug, so navigate to client/speaktype.py
                    let dev_path = p
                        .ancestors()
                        .find(|a| a.join("client").join("speaktype.py").exists())
                        .map(|a| a.join("client").join("speaktype.py"));
                    dev_path.unwrap_or_else(|| p.join("client").join("speaktype.py"))
                })
                .unwrap_or_else(|| std::path::PathBuf::from("client/speaktype.py"));

            // Also check relative to widget/src-tauri
            let script_path = if script_path.exists() {
                script_path
            } else {
                // During development, client/speaktype.py is two dirs up from src-tauri
                let alt = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join("client/speaktype.py"))
                    .unwrap_or_else(|| std::path::PathBuf::from("../../client/speaktype.py"));
                if alt.exists() {
                    alt
                } else {
                    script_path
                }
            };

            let script_str = script_path.to_string_lossy().to_string();
            sidecar::log_message(&format!("[speaktype] Using script: {}", script_str));

            let sidecar = sidecar::Sidecar::spawn(
                &settings.python_path,
                &script_str,
                &settings.api_url,
            )?;

            let stdin = sidecar.start_reader(app.handle().clone());
            app.manage(SidecarStdin(stdin.clone()));
            app.manage(AppSettings(Arc::new(Mutex::new(settings.clone()))));

            // Register global shortcut
            let hotkey_str = settings.hotkey.clone();
            let stdin_for_shortcut = stdin.clone();

            if let Ok(shortcut) = hotkey_str.parse::<Shortcut>() {
                let is_recording = Arc::new(Mutex::new(false));
                let is_recording_clone = is_recording.clone();

                // Listen for sidecar events to keep shortcut state in sync
                let is_rec_for_started = is_recording.clone();
                let is_rec_for_stopped = is_recording.clone();
                let is_rec_for_pasted = is_recording.clone();
                let is_rec_for_nospeech = is_recording.clone();
                let is_rec_for_error = is_recording.clone();
                app.listen("sidecar:recording_started", move |_| {
                    *is_rec_for_started.lock().unwrap() = true;
                });
                app.listen("sidecar:recording_stopped", move |_| {
                    *is_rec_for_stopped.lock().unwrap() = false;
                });
                app.listen("sidecar:pasted", move |_| {
                    *is_rec_for_pasted.lock().unwrap() = false;
                });
                app.listen("sidecar:no_speech", move |_| {
                    *is_rec_for_nospeech.lock().unwrap() = false;
                });
                app.listen("sidecar:error", move |_| {
                    *is_rec_for_error.lock().unwrap() = false;
                });

                app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, _event| {
                    let recording = is_recording_clone.lock().unwrap();
                    let cmd = if *recording {
                        json!({"cmd": "stop_recording"})
                    } else {
                        json!({"cmd": "start_recording"})
                    };

                    use std::io::Write;
                    if let Ok(mut stdin) = stdin_for_shortcut.lock() {
                        if let Ok(line) = serde_json::to_string(&cmd) {
                            let _ = stdin.write_all(format!("{}\n", line).as_bytes());
                            let _ = stdin.flush();
                        }
                    }
                })?;
            } else {
                eprintln!("[speaktype] Invalid hotkey: {}", hotkey_str);
            }

            // System tray
            let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
            let show_item = MenuItemBuilder::with_id("show", "Show Widget").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&settings_item)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("SpeakType")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "settings" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = commands::open_settings(app).await;
                        });
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
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
