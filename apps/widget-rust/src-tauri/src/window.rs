use crate::logging;
use tauri::{PhysicalPosition, WebviewWindow};

const DEFAULT_MARGIN: i32 = 100;

/// Return a position guaranteed to be visible on at least one connected monitor.
pub fn clamp_to_visible_screens(window: &WebviewWindow, x: f64, y: f64) -> (f64, f64) {
    let outer = window.outer_size().ok();
    let win_w = outer.map(|s| s.width as i32).unwrap_or(96);
    let win_h = outer.map(|s| s.height as i32).unwrap_or(96);

    let px = x.round() as i32;
    let py = y.round() as i32;

    let monitors = window.available_monitors().unwrap_or_default();
    if monitors.is_empty() {
        return (DEFAULT_MARGIN as f64, DEFAULT_MARGIN as f64);
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

    if let Ok(Some(primary)) = window.primary_monitor() {
        let pos = primary.position();
        let clamped = (
            pos.x as f64 + DEFAULT_MARGIN as f64,
            pos.y as f64 + DEFAULT_MARGIN as f64,
        );
        logging::log_message(&format!(
            "[window] Saved position ({}, {}) is off-screen — resetting to ({}, {})",
            x, y, clamped.0, clamped.1
        ));
        return clamped;
    }

    (DEFAULT_MARGIN as f64, DEFAULT_MARGIN as f64)
}

pub fn apply_saved_position(window: &WebviewWindow, x: f64, y: f64) -> (f64, f64) {
    let (cx, cy) = clamp_to_visible_screens(window, x, y);
    let _ = window.set_position(tauri::Position::Physical(PhysicalPosition {
        x: cx as i32,
        y: cy as i32,
    }));
    (cx, cy)
}
