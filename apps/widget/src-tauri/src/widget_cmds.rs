//! Commands and events the widget and the permissions page need: what the OS
//! says about screen recording, a typed keystroke that proves Accessibility
//! works, and re-running the last dictation without saving it.

use crate::commands::AppState;
use crate::history::History;
use crate::paste;
use crate::session_cmds::IpcError;
use crate::window::{self, WidgetCorner};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

/// One change to where the widget sits: a corner (from Settings, the widget's
/// menu or the tray) or `custom` after a drag. Every window shows it the same.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct WidgetCornerChanged {
    /// A [`crate::window::WidgetCorner`] wire name.
    pub corner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct MicDevicesChanged {
    /// The system default input's name; empty when there is none.
    pub name: String,
    pub count: usize,
}

/// The widget is being dragged (the webview goes non-interactive during a drag
/// and reports its end). `save_dragged_position` reads it to know a native move
/// was the user's own.
pub static DRAGGING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Store the corner, then place the widget wherever it now is (the active
/// display's corner, or where the user dropped it).
pub fn apply_corner(
    widget: &tauri::WebviewWindow,
    settings: &std::sync::Arc<std::sync::Mutex<crate::settings::Settings>>,
    corner: WidgetCorner,
) -> Result<(), IpcError> {
    let placed = match corner {
        WidgetCorner::Custom => widget
            .outer_position()
            .map_err(|e| IpcError::new("widgetCorner", e.to_string()))?,
        corner => window_at_corner(widget, corner)?,
    };
    let mut settings = settings
        .lock()
        .map_err(|e| IpcError::new("settings", e.to_string()))?;
    settings.widget_corner = corner.as_str().to_string();
    settings.window_x = Some(placed.x as f64);
    settings.window_y = Some(placed.y as f64);
    settings.save().map_err(|e| IpcError::new("settings", e))
}

fn window_at_corner(
    widget: &tauri::WebviewWindow,
    corner: WidgetCorner,
) -> Result<tauri::PhysicalPosition<i32>, IpcError> {
    match window::place_at_corner(widget, corner) {
        Some(point) => Ok(tauri::PhysicalPosition::new(point.x, point.y)),
        None => Err(IpcError::new(
            "widgetCorner",
            "No display to place the widget on.",
        )),
    }
}

/// The widget was dragged: keep where the user let go. A native move that the
/// widget did not report as a drag (a programmatic corner place) keeps the
/// chosen corner.
pub fn save_dragged_position(
    widget: &tauri::WebviewWindow,
    position: tauri::PhysicalPosition<i32>,
) {
    let app = widget.app_handle();
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Ok(mut settings) = state.settings.lock() else {
        return;
    };
    // `position` is the square around the bubble (widget_frame), not the
    // window; it is clamped on screen when it is restored at startup.
    let (x, y) = (position.x as f64, position.y as f64);
    if settings.window_x == Some(x) && settings.window_y == Some(y) {
        return;
    }
    settings.window_x = Some(x);
    settings.window_y = Some(y);
    if DRAGGING.load(Ordering::SeqCst) {
        settings.widget_corner = WidgetCorner::Custom.as_str().to_string();
    }
    if let Err(e) = settings.save() {
        crate::logging::log_message(&format!("[window] Could not save the widget position: {e}"));
    }
}

#[tauri::command]
#[specta::specta]
pub fn set_widget_corner(
    app: AppHandle,
    state: State<'_, AppState>,
    corner: String,
) -> Result<(), IpcError> {
    let corner = WidgetCorner::parse(&corner)
        .ok_or_else(|| IpcError::new("widgetCorner", "Unknown corner."))?;
    if state.is_recording.load(Ordering::SeqCst) {
        return Err(IpcError::new(
            "widgetCorner",
            "The widget stays put while you dictate.",
        ));
    }
    let widget = app
        .get_webview_window("main")
        .ok_or_else(|| IpcError::new("widgetCorner", "The widget is not open."))?;
    apply_corner(&widget, &state.settings, corner)?;
    WidgetCornerChanged {
        corner: corner.as_str().to_string(),
    }
    .emit(&app)
    .map_err(|e| IpcError::new("widgetCorner", e.to_string()))
}

/// Shown/hidden from the tray and Settings: reveal the widget at `corner`.
pub fn show_widget_at(app: &AppHandle, corner: WidgetCorner) -> Result<(), String> {
    let widget = app
        .get_webview_window("main")
        .ok_or_else(|| "The widget is not open.".to_string())?;
    let state = app.state::<AppState>();
    apply_corner(&widget, &state.settings, corner).map_err(|e| e.message.clone())?;
    widget.show().map_err(|e| e.to_string())?;
    WidgetCornerChanged {
        corner: corner.as_str().to_string(),
    }
    .emit(app)
    .map_err(|e| e.to_string())
}

/// True when macOS has granted screen recording to this build.
#[tauri::command]
#[specta::specta]
pub fn screen_recording_status() -> bool {
    crate::permissions::screen_recording_granted()
}

/// Show the system prompt, which also adds SpeakType to the Screen Recording list.
#[tauri::command]
#[specta::specta]
pub fn request_screen_recording() -> bool {
    crate::permissions::request_screen_recording()
}

/// Type "SpeakType test" into the focused element (the permissions page's own
/// field). Sent from a worker thread: posting is cheap, but doing it on the IPC
/// thread would make the page wait on the main loop that delivers the keys.
#[tauri::command]
#[specta::specta]
pub async fn test_typing(app: AppHandle) -> Result<(), IpcError> {
    // The keystrokes go to whatever has focus, so only type into our own window.
    let focused = app
        .get_webview_window("settings")
        .and_then(|w| w.is_focused().ok())
        .unwrap_or(false);
    if !focused {
        return Err(IpcError::new(
            "focus",
            "Click the test field in SpeakType, then try again.",
        ));
    }
    if !crate::permissions::check_accessibility(false) {
        return Err(IpcError::new(
            "accessibility",
            "Turn on Accessibility for SpeakType first.",
        ));
    }
    tauri::async_runtime::spawn_blocking(|| paste::type_test_text(paste::TEST_TYPING_TEXT))
        .await
        .map_err(|e| IpcError::new("typing", e.to_string()))?
        .map_err(|e| IpcError::new("typing", e))
}

/// Re-run an entry with the active model and return the text without saving it
/// or touching the entry (onboarding's compare).
#[tauri::command]
#[specta::specta]
pub async fn reprocess_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    index: u32,
) -> Result<String, IpcError> {
    if state.is_recording.load(Ordering::SeqCst) || state.is_transcribing.load(Ordering::SeqCst) {
        return Err(IpcError::new("busy", "Finish the current dictation first."));
    }
    let settings = state
        .settings
        .lock()
        .map_err(|e| IpcError::new("settings", e.to_string()))?
        .clone();
    let newest = state
        .last_dictation
        .lock()
        .map_err(|e| IpcError::new("history", e.to_string()))?
        .as_ref()
        .map(|last| {
            (
                last.entry_id.clone(),
                last.samples.clone(),
                last.sample_rate,
            )
        });
    History::reprocess(&app, index as usize, &settings, newest)
        .await
        .map_err(|e| IpcError::new("reprocess", e))
}

/// Emit `MicDevicesChanged` when the default input or the device count moves,
/// so pickers and the permissions page follow a headset being plugged in.
pub fn spawn_device_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last: Option<(String, usize)> = None;
        loop {
            let name = crate::audio::get_default_input_device_name().unwrap_or_default();
            let count = crate::audio::count_input_devices();
            if last.as_ref() != Some(&(name.clone(), count)) {
                crate::logging::log_message(&format!(
                    "[audio] Device changed: {:?} (inputs={})",
                    name, count
                ));
                let changed = MicDevicesChanged {
                    name: name.clone(),
                    count,
                };
                if let Err(e) = changed.emit(&app) {
                    crate::logging::log_message(&format!(
                        "[audio] MicDevicesChanged emit failed: {e}"
                    ));
                }
                last = Some((name, count));
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::window::WidgetCorner;

    #[test]
    fn unknown_corners_are_rejected() {
        for value in ["middle", "custom", "bottomRight"] {
            assert_eq!(WidgetCorner::parse(value), None, "{value} must not parse");
        }
        for value in ["bottom_right", "bottom_center", "top_right", "bottom_left"] {
            assert!(WidgetCorner::parse(value).is_some(), "{value} must parse");
        }
    }

    #[test]
    fn widget_corner_changed_keeps_its_wire_shape() {
        let json = serde_json::to_value(super::WidgetCornerChanged {
            corner: "bottom_center".to_string(),
        })
        .unwrap();
        assert_eq!(json, serde_json::json!({ "corner": "bottom_center" }));
    }
}
