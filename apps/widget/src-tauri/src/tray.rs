use crate::logging::log_message;
use std::sync::Mutex;
use tauri::menu::MenuItem;
use tauri::tray::TrayIconId;

/// Stores the record menu item so we can update its label from anywhere.
static RECORD_ITEM: Mutex<Option<MenuItem<tauri::Wry>>> = Mutex::new(None);

pub fn set_record_menu_item(item: MenuItem<tauri::Wry>) {
    *RECORD_ITEM.lock().unwrap() = Some(item);
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
        TrayState::Recording => (ICON_RECORDING, "SpeakType — Recording...", "Stop Recording", false),
        TrayState::Transcribing => (ICON_TRANSCRIBING, "SpeakType — Transcribing...", "Stop Recording", true),
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

