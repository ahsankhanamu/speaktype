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
///
/// On multi-monitor setups the widget may span two screens. Rather than requiring
/// full containment, we find the monitor with the most overlap and clamp within it.
pub fn clamp_to_visible_screens(window: &WebviewWindow, x: f64, y: f64) -> (f64, f64) {
    let (win_w, win_h) = window_outer_size(window);

    let px = x.round() as i32;
    let py = y.round() as i32;

    let monitors = window.available_monitors().unwrap_or_default();
    if monitors.is_empty() {
        return default_widget_position(window);
    }

    // First: exact containment on any monitor
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

    // Multi-monitor: find the monitor with the most overlap and clamp within it
    let mut best_overlap = 0i32;
    let mut best_monitor: Option<(i32, i32, i32, i32)> = None;
    for monitor in &monitors {
        let pos = monitor.position();
        let size = monitor.size();
        let m_left = pos.x;
        let m_top = pos.y;
        let m_right = pos.x + size.width as i32;
        let m_bottom = pos.y + size.height as i32;

        let ix_left = px.max(m_left);
        let ix_top = py.max(m_top);
        let ix_right = (px + win_w).min(m_right);
        let ix_bottom = (py + win_h).min(m_bottom);

        if ix_left < ix_right && ix_top < ix_bottom {
            let area = (ix_right - ix_left) * (ix_bottom - ix_top);
            if area > best_overlap {
                best_overlap = area;
                best_monitor = Some((m_left, m_top, m_right, m_bottom));
            }
        }
    }

    if let Some((m_left, m_top, m_right, m_bottom)) = best_monitor {
        let cx = px.max(m_left).min(m_right - win_w) as f64;
        let cy = py.max(m_top).min(m_bottom - win_h) as f64;
        logging::log_message(&format!(
            "[window] Position ({}, {}) straddles monitors — clamping to ({}, {})",
            x, y, cx, cy
        ));
        return (cx, cy);
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
