use crate::logging;
use tauri::{PhysicalPosition, WebviewWindow};

const CORNER_MARGIN_X: i32 = 20;
#[cfg(target_os = "macos")]
const CORNER_MARGIN_Y: i32 = 36;
#[cfg(not(target_os = "macos"))]
const CORNER_MARGIN_Y: i32 = 20;

fn monitor_for_window(window: &WebviewWindow) -> Option<(i32, i32, i32, i32)> {
    let monitor = window
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten())?;
    let pos = monitor.position();
    let size = monitor.size();
    Some((
        pos.x,
        pos.y,
        size.width as i32,
        size.height as i32,
    ))
}

fn window_outer_size(window: &WebviewWindow) -> (i32, i32) {
    window
        .outer_size()
        .ok()
        .map(|s| (s.width as i32, s.height as i32))
        .unwrap_or((96, 96))
}

/// Platform default: top-right on macOS, top-left on Windows/Linux.
pub fn default_widget_position(window: &WebviewWindow) -> (f64, f64) {
    let (win_w, _) = window_outer_size(window);

    let Some((mon_x, mon_y, mon_w, _mon_h)) = monitor_for_window(window) else {
        return (CORNER_MARGIN_X as f64, CORNER_MARGIN_Y as f64);
    };

    #[cfg(target_os = "macos")]
    let x = mon_x + mon_w - win_w - CORNER_MARGIN_X;
    #[cfg(not(target_os = "macos"))]
    let x = mon_x + CORNER_MARGIN_X;

    let y = mon_y + CORNER_MARGIN_Y;
    (x as f64, y as f64)
}

/// Return a position guaranteed to be visible on at least one connected monitor.
pub fn clamp_to_visible_screens(window: &WebviewWindow, x: f64, y: f64) -> (f64, f64) {
    let (win_w, win_h) = window_outer_size(window);

    let px = x.round() as i32;
    let py = y.round() as i32;

    let monitors = window.available_monitors().unwrap_or_default();
    if monitors.is_empty() {
        return default_widget_position(window);
    }

    for monitor in &monitors {
        let pos = monitor.position();
        let size = monitor.size();
        let left = pos.x;
        let top = pos.y;
        let right = pos.x + size.width as i32;
        let bottom = pos.y + size.height as i32;

        if px >= left && py >= top && px + win_w <= right && py + win_h <= bottom {
            return (x, y);
        }
    }

    let fallback = default_widget_position(window);
    logging::log_message(&format!(
        "[window] Saved position ({}, {}) is off-screen — resetting to ({}, {})",
        x, y, fallback.0, fallback.1
    ));
    fallback
}

pub fn apply_saved_position(window: &WebviewWindow, x: f64, y: f64) -> (f64, f64) {
    let (cx, cy) = clamp_to_visible_screens(window, x, y);
    let _ = window.set_position(tauri::Position::Physical(PhysicalPosition {
        x: cx as i32,
        y: cy as i32,
    }));
    (cx, cy)
}

pub fn place_at_default_position(window: &WebviewWindow) -> (f64, f64) {
    let (x, y) = default_widget_position(window);
    logging::log_message(&format!(
        "[window] Placing widget at default corner ({}, {})",
        x, y
    ));
    apply_saved_position(window, x, y)
}

pub fn reset_to_default_position(window: &WebviewWindow) -> (f64, f64) {
    place_at_default_position(window)
}
