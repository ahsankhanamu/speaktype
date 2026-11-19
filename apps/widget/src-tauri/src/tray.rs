use crate::logging::log_message;
use std::sync::Mutex;
use tauri::menu::MenuItem;
use tauri::tray::TrayIconId;

/// Stores the record menu item so we can update its label from anywhere.
static RECORD_ITEM: Mutex<Option<MenuItem<tauri::Wry>>> = Mutex::new(None);

/// Stores the speech-model item so its text follows the server.
static MODEL_ITEM: Mutex<Option<MenuItem<tauri::Wry>>> = Mutex::new(None);

pub fn set_record_menu_item(item: MenuItem<tauri::Wry>) {
    *RECORD_ITEM.lock().unwrap() = Some(item);
}

pub fn set_model_menu_item(item: MenuItem<tauri::Wry>) {
    *MODEL_ITEM.lock().unwrap() = Some(item);
}

/// Brings the model item in line with the server: its loaded size while it
/// can be unloaded, "Speech model not loaded" (disabled) otherwise.
pub fn refresh_model_item(app: &tauri::AppHandle) {
    let item = match crate::commands::server_info(app.clone()) {
        Ok(info) => model_item(info.model.as_deref(), info.size.as_deref()),
        Err(e) => {
            log_message(&format!("[tray] Could not read the model state: {e}"));
            return;
        }
    };
    let Ok(guard) = MODEL_ITEM.lock() else {
        return;
    };
    let Some(model) = guard.as_ref() else {
        return;
    };
    let _ = model.set_text(&item.1);
    let _ = model.set_enabled(item.2);
}

pub const TRAY_ID: &str = "speaktype-tray";

static ICON_IDLE: &[u8] = include_bytes!("../icons/tray-icon.png");
static ICON_RECORDING: &[u8] = include_bytes!("../icons/tray-icon-recording.png");
static ICON_TRANSCRIBING: &[u8] = include_bytes!("../icons/tray-icon-transcribing.png");
static ICON_ERROR: &[u8] = include_bytes!("../icons/tray-icon-error.png");

#[derive(Debug, Clone, Copy)]
pub enum TrayState {
    Idle,
    Recording,
    Transcribing,
    Error,
}

pub fn set_tray_state(app: &tauri::AppHandle, state: TrayState) {
    let tray_id = TrayIconId::new(TRAY_ID);
    let Some(tray) = app.tray_by_id(&tray_id) else {
        log_message("[tray] Could not find tray icon");
        return;
    };

    let (icon_bytes, tooltip, record_label, as_template) = match state {
        TrayState::Idle => (ICON_IDLE, "SpeakType", "Start Recording", true),
        TrayState::Recording => (
            ICON_RECORDING,
            "SpeakType — Recording...",
            "Stop Recording",
            false,
        ),
        TrayState::Transcribing => (
            ICON_TRANSCRIBING,
            "SpeakType — Transcribing...",
            "Stop Recording",
            true,
        ),
        TrayState::Error => (ICON_ERROR, "SpeakType — Error", "Start Recording", false),
    };

    match tauri::image::Image::from_bytes(icon_bytes) {
        Ok(image) => {
            let _ = tray.set_icon(Some(image));
            let _ = tray.set_icon_as_template(as_template);
            let _ = tray.set_tooltip(Some(tooltip));
            // Update the record menu item label
            if let Ok(guard) = RECORD_ITEM.lock() {
                if let Some(ref item) = *guard {
                    let _ = item.set_text(record_label);
                }
            }
            log_message(&format!("[tray] State -> {:?}", state));
        }
        Err(e) => {
            log_message(&format!("[tray] Failed to load icon: {}", e));
        }
    }
}

/// Shows the error icon briefly, then resets to idle.
pub fn flash_error(app: &tauri::AppHandle) {
    set_tray_state(app, TrayState::Error);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        set_tray_state(&app, TrayState::Idle);
    });
}

pub const NOT_LOADED: &str = "Speech model not loaded";

/// The menu-bar item for the model, between "Paste last dictation again" and
/// "History…": `(id, text, enabled)`.
pub fn model_item(model: Option<&str>, size: Option<&str>) -> (String, String, bool) {
    match (model, size) {
        (Some(model), Some(size)) => (
            "unload".to_string(),
            format!("Unload {model} · frees {size}"),
            true,
        ),
        _ => ("unload".to_string(), NOT_LOADED.to_string(), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_model_item_offers_its_size_while_the_model_is_idle() {
        assert_eq!(
            model_item(Some("large-v3"), Some("~3 GB")),
            (
                "unload".to_string(),
                "Unload large-v3 · frees ~3 GB".to_string(),
                true
            )
        );
    }

    #[test]
    fn the_model_item_says_when_nothing_is_loaded() {
        assert_eq!(
            model_item(None, Some("~1.5 GB")),
            ("unload".to_string(), NOT_LOADED.to_string(), false)
        );
    }
}
