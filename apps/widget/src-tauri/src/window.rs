use crate::logging;
use serde::{Deserialize, Serialize};
use tauri::{PhysicalPosition, WebviewWindow};

const CORNER_MARGIN_X: i32 = 20;
#[cfg(target_os = "macos")]
const CORNER_MARGIN_Y: i32 = 36;
#[cfg(not(target_os = "macos"))]
const CORNER_MARGIN_Y: i32 = 20;

/// A display's visible frame in physical pixels: below the menu bar, above the
/// Dock, so a corner position never sits under either.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A window size in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// Gap between the widget and the edges of the visible frame (spec: 16 pt).
pub const CORNER_INSET: i32 = 16;

/// The Settings → Appearance "Widget position" choices, plus `custom` once the
/// user has dragged the widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WidgetCorner {
    BottomRight,
    BottomCenter,
    TopRight,
    BottomLeft,
    Custom,
}

impl WidgetCorner {
    /// `None` for `custom` and anything unrecognised: those keep the saved
    /// position instead of snapping to a corner.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "bottom_right" => Some(Self::BottomRight),
            "bottom_center" => Some(Self::BottomCenter),
            "top_right" => Some(Self::TopRight),
            "bottom_left" => Some(Self::BottomLeft),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::BottomRight => "bottom_right",
            Self::BottomCenter => "bottom_center",
            Self::TopRight => "top_right",
            Self::BottomLeft => "bottom_left",
            Self::Custom => "custom",
        }
    }
}

/// The widget's top-left origin for `corner`: `CORNER_INSET` from the visible
/// frame's edges. A window wider than the frame still starts at the left edge.
pub fn corner_origin(visible: Region, size: Size, corner: WidgetCorner) -> Point {
    let left = visible.x + CORNER_INSET;
    let right = visible.x + visible.width as i32 - size.width as i32 - CORNER_INSET;
    let top = visible.y + CORNER_INSET;
    let bottom = visible.y + visible.height as i32 - size.height as i32 - CORNER_INSET;
    let (x, y) = match corner {
        WidgetCorner::Custom | WidgetCorner::BottomLeft => (left, bottom),
        WidgetCorner::BottomCenter => (
            (visible.x + visible.width as i32 - size.width as i32) / 2,
            bottom,
        ),
        WidgetCorner::TopRight => (right, top),
        WidgetCorner::BottomRight => (right, bottom),
    };
    Point { x, y }
}

/// The visible frame of the display the widget is on (the active display).
pub fn visible_frame(window: &WebviewWindow) -> Option<Region> {
    let monitor = window.current_monitor().ok().flatten()?;
    let area = monitor.work_area();
    Some(Region {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
    })
}

/// Place the widget at `corner` and return the position, for the caller to
/// store.
pub fn place_at_corner(window: &WebviewWindow, corner: WidgetCorner) -> Option<Point> {
    let visible = visible_frame(window)?;
    let outer = window.outer_size().ok()?;
    let point = corner_origin(
        visible,
        Size {
            width: outer.width,
            height: outer.height,
        },
        corner,
    );
    window
        .set_position(PhysicalPosition::new(point.x, point.y))
        .map_err(|e| logging::log_message(&format!("[window] Could not place at corner: {e}")))
        .ok()?;
    Some(point)
}

fn monitor_for_window(window: &WebviewWindow) -> Option<(i32, i32, i32, i32)> {
    let monitor = window
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten())?;
    let pos = monitor.position();
    let size = monitor.size();
    Some((pos.x, pos.y, size.width as i32, size.height as i32))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1512 × 950 Retina frame below a 38 px menu bar.
    fn visible() -> Region {
        Region {
            x: 0,
            y: 38,
            width: 1512,
            height: 912,
        }
    }

    const SIZE: Size = Size {
        width: 96,
        height: 96,
    };

    #[test]
    fn widget_corner_frames() {
        let frame = visible();
        assert_eq!(
            corner_origin(frame, SIZE, WidgetCorner::BottomRight),
            Point { x: 1400, y: 838 }
        );
        assert_eq!(
            corner_origin(frame, SIZE, WidgetCorner::BottomCenter),
            Point { x: 708, y: 838 }
        );
        assert_eq!(
            corner_origin(frame, SIZE, WidgetCorner::TopRight),
            Point { x: 1400, y: 54 }
        );
        assert_eq!(
            corner_origin(frame, SIZE, WidgetCorner::BottomLeft),
            Point { x: 16, y: 838 }
        );
    }

    #[test]
    fn corner_origin_uses_the_offset_display_and_the_inset() {
        let second = Region {
            x: 1512,
            y: 0,
            width: 1280,
            height: 800,
        };
        assert_eq!(
            corner_origin(second, SIZE, WidgetCorner::TopRight),
            Point { x: 2680, y: 16 }
        );
        assert_eq!(
            corner_origin(second, SIZE, WidgetCorner::BottomLeft),
            Point { x: 1528, y: 688 }
        );
    }

    #[test]
    fn widget_corner_wire_names() {
        for (corner, wire) in [
            (WidgetCorner::BottomRight, "bottom_right"),
            (WidgetCorner::BottomCenter, "bottom_center"),
            (WidgetCorner::TopRight, "top_right"),
            (WidgetCorner::BottomLeft, "bottom_left"),
            (WidgetCorner::Custom, "custom"),
        ] {
            assert_eq!(corner.as_str(), wire);
            assert_eq!(serde_json::to_value(corner).unwrap(), wire);
        }
        assert_eq!(
            WidgetCorner::parse("bottom_center"),
            Some(WidgetCorner::BottomCenter)
        );
        assert_eq!(WidgetCorner::parse("custom"), None);
        assert_eq!(WidgetCorner::parse("middle"), None);
    }
}
