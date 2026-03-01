use crate::history::History;
use crate::settings::Settings;
use serde_json::json;
use std::process::ChildStdin;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

pub struct SidecarStdin(pub Arc<Mutex<ChildStdin>>);
pub struct AppSettings(pub Arc<Mutex<Settings>>);

fn send_to_sidecar(stdin: &Arc<Mutex<ChildStdin>>, cmd: &serde_json::Value) -> Result<(), String> {
    use std::io::Write;
    let mut stdin = stdin.lock().map_err(|e| e.to_string())?;
    let line = serde_json::to_string(cmd).map_err(|e| e.to_string())?;
    stdin
        .write_all(format!("{}\n", line).as_bytes())
        .map_err(|e| e.to_string())?;
    stdin.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn toggle_recording(state: State<'_, SidecarStdin>, is_recording: bool) -> Result<(), String> {
    let cmd = if is_recording {
        json!({"cmd": "stop_recording"})
    } else {
        json!({"cmd": "start_recording"})
    };
    crate::sidecar::log_message(&format!("[toggle_recording] is_recording={}, sending: {}", is_recording, cmd));
    send_to_sidecar(&state.0, &cmd)
}

#[tauri::command]
pub fn send_sidecar_command(state: State<'_, SidecarStdin>, command: String) -> Result<(), String> {
    let cmd: serde_json::Value =
        serde_json::from_str(&command).map_err(|e| format!("Invalid JSON: {}", e))?;
    send_to_sidecar(&state.0, &cmd)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppSettings>) -> Result<Settings, String> {
    let settings = state.0.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppSettings>,
    settings: Settings,
) -> Result<(), String> {
    settings.save()?;
    let mut current = state.0.lock().map_err(|e| e.to_string())?;
    *current = settings;
    Ok(())
}

#[tauri::command]
pub fn update_hotkey(
    state: State<'_, AppSettings>,
    hotkey: String,
) -> Result<(), String> {
    let mut settings = state.0.lock().map_err(|e| e.to_string())?;
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
    match client.get(&health_url).timeout(std::time::Duration::from_secs(3)).send().await {
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
pub fn clear_history() -> Result<(), String> {
    History::clear()
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
    // "Minimize" for a widget = hide to tray
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
    state: State<'_, AppSettings>,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let pos = window.outer_position().map_err(|e| e.to_string())?;
        let mut settings = state.0.lock().map_err(|e| e.to_string())?;
        settings.window_x = Some(pos.x as f64);
        settings.window_y = Some(pos.y as f64);
        settings.save()?;
    }
    Ok(())
}
