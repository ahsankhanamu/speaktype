//! Where the widget window and its pill sit on screen.
//!
//! `home` is where the person put the bubble: its centre, in logical points.
//! Every pill is placed around it and kept inside the display's visible frame,
//! so a bubble parked at the edge still opens into a whole pill. The window is
//! the pill plus room for its halo, tooltip or menu, and it moves and resizes
//! in one native call so the pill never jumps while it morphs.

use crate::session_cmds::IpcError;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use tauri::{PhysicalPosition, WebviewWindow};

/// Room around the pill for its halo.
pub const MARGIN: f64 = 24.0;
/// The window the widget starts at (`tauri.conf.json`): the saved position is
/// the top-left of this square centred on the bubble.
const SAVED_SQUARE: f64 = 92.0;
/// Origins of our own reframes whose `Moved` events have not arrived yet.
const PENDING_MOVES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct ScreenRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ScreenRect {
    fn right(&self) -> f64 {
        self.x + self.width
    }
    fn bottom(&self) -> f64 {
        self.y + self.height
    }
    fn union(&self, other: &ScreenRect) -> ScreenRect {
        let (x, y) = (self.x.min(other.x), self.y.min(other.y));
        ScreenRect {
            x,
            y,
            width: self.right().max(other.right()) - x,
            height: self.bottom().max(other.bottom()) - y,
        }
    }
    fn offset(&self, dx: f64, dy: f64) -> ScreenRect {
        ScreenRect {
            x: self.x + dx,
            y: self.y + dy,
            ..*self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct Extent {
    pub width: f64,
    pub height: f64,
}

/// The pill before and after a layout, in the window's own coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct PillLayout {
    pub from: ScreenRect,
    pub to: ScreenRect,
}

/// Keep `start..start + len` inside `lo..hi`; a span larger than the range
/// starts at `lo`.
fn clamp_span(start: f64, len: f64, lo: f64, hi: f64) -> f64 {
    start.min(hi - len).max(lo)
}

/// The pill's rect on screen: centred on `home`, `inset` inside `area`.
pub fn place_pill(home: (f64, f64), size: Extent, area: ScreenRect, inset: f64) -> ScreenRect {
    ScreenRect {
        x: clamp_span(
            home.0 - size.width / 2.0,
            size.width,
            area.x + inset,
            area.right() - inset,
        ),
        y: clamp_span(
            home.1 - size.height / 2.0,
            size.height,
            area.y + inset,
            area.bottom() - inset,
        ),
        width: size.width,
        height: size.height,
    }
}

/// The window for `pill`: the pill and `MARGIN` around it, at least `room` in
/// size, on whole points. Extra room for a tooltip or menu is moved inside
/// `area` so what opens in it stays on screen; the halo margin may run off the
/// edge, where nothing is drawn.
pub fn frame_for(pill: ScreenRect, room: Extent, area: ScreenRect) -> ScreenRect {
    let width = (pill.width + 2.0 * MARGIN).max(room.width).ceil();
    let height = (pill.height + 2.0 * MARGIN).max(room.height).ceil();
    let centred = |start: f64, len: f64, size: f64| start + len / 2.0 - size / 2.0;
    let mut x = centred(pill.x, pill.width, width);
    let mut y = centred(pill.y, pill.height, height);
    if room.width > pill.width + 2.0 * MARGIN {
        x = clamp_span(x, width, area.x, area.right()).clamp(pill.right() - width, pill.x);
    }
    if room.height > pill.height + 2.0 * MARGIN {
        y = clamp_span(y, height, area.y, area.bottom()).clamp(pill.bottom() - height, pill.y);
    }
    ScreenRect {
        x: x.round(),
        y: y.round(),
        width,
        height,
    }
}

#[derive(Default)]
struct Layout {
    home: Option<(f64, f64)>,
    /// The window as we last set it, in logical points.
    frame: Option<ScreenRect>,
    own_moves: VecDeque<(i32, i32)>,
}

static LAYOUT: Mutex<Layout> = Mutex::new(Layout {
    home: None,
    frame: None,
    own_moves: VecDeque::new(),
});

fn window_rect(window: &WebviewWindow) -> Option<ScreenRect> {
    let scale = window.scale_factor().ok()?;
    let pos = window.outer_position().ok()?.to_logical::<f64>(scale);
    let size = window.outer_size().ok()?.to_logical::<f64>(scale);
    Some(ScreenRect {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
    })
}

/// The visible frame (below the menu bar, above the Dock) of the display
/// `home` is on, in logical points.
fn work_area(window: &WebviewWindow, home: (f64, f64)) -> Option<ScreenRect> {
    let logical = |m: &tauri::Monitor| {
        let area = m.work_area();
        let scale = m.scale_factor();
        ScreenRect {
            x: area.position.x as f64 / scale,
            y: area.position.y as f64 / scale,
            width: area.size.width as f64 / scale,
            height: area.size.height as f64 / scale,
        }
    };
    let monitors = window.available_monitors().ok()?;
    let areas: Vec<ScreenRect> = monitors.iter().map(logical).collect();
    let distance = |a: &ScreenRect| {
        let dx = (a.x - home.0).max(home.0 - a.right()).max(0.0);
        let dy = (a.y - home.1).max(home.1 - a.bottom()).max(0.0);
        dx * dx + dy * dy
    };
    areas
        .into_iter()
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
}

/// Move and resize the window in one step. On macOS that is a single
/// `setFrame:display:`, so the window never shows the new origin at the old
/// size.
fn set_frame(window: &WebviewWindow, frame: ScreenRect) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWindow;
        use objc2_foundation::{NSPoint, NSRect, NSSize};
        let primary = window
            .primary_monitor()
            .map_err(|e| e.to_string())?
            .ok_or("No display")?;
        let primary_height = primary.size().height as f64 / primary.scale_factor();
        let ns_window = window.ns_window().map_err(|e| e.to_string())?;
        // SAFETY: a sync command runs on the main thread, and the pointer is
        // the live NSWindow (an NSPanel subclass) that Tauri owns.
        unsafe {
            let ns_window = &*(ns_window as *const NSWindow);
            ns_window.setFrame_display(
                NSRect::new(
                    NSPoint::new(frame.x, primary_height - frame.y - frame.height),
                    NSSize::new(frame.width, frame.height),
                ),
                true,
            );
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        window
            .set_position(tauri::LogicalPosition::new(frame.x, frame.y))
            .and_then(|_| window.set_size(tauri::LogicalSize::new(frame.width, frame.height)))
            .map_err(|e| e.to_string())
    }
}

/// Place the pill of `size` around home and fit the window to it. `current` is
/// where the pill is drawn now, in the window's coordinates: the window then
/// also holds it, so the pill can morph from there. Without it the window hugs
/// the new pill alone.
#[tauri::command]
#[specta::specta]
pub fn layout_widget(
    window: WebviewWindow,
    size: Extent,
    room: Extent,
    inset: f64,
    current: Option<ScreenRect>,
) -> Result<PillLayout, IpcError> {
    let fail = |e: String| IpcError::new("widgetLayout", e);
    let scale = window.scale_factor().map_err(|e| fail(e.to_string()))?;
    let mut layout = LAYOUT.lock().map_err(|e| fail(e.to_string()))?;
    let before = match layout.frame {
        Some(frame) => frame,
        None => window_rect(&window).ok_or_else(|| fail("The widget has no frame.".into()))?,
    };
    let home = *layout.home.get_or_insert((
        before.x + before.width / 2.0,
        before.y + before.height / 2.0,
    ));
    let area = work_area(&window, home).unwrap_or(before);
    let to = place_pill(home, size, area, inset);
    let from = current.map_or(to, |c| c.offset(before.x, before.y));
    let frame = frame_for(from.union(&to), room, area);
    if frame != before || layout.frame.is_none() {
        set_frame(&window, frame).map_err(fail)?;
        if layout.own_moves.len() == PENDING_MOVES {
            layout.own_moves.pop_front();
        }
        layout.own_moves.push_back((
            (frame.x * scale).round() as i32,
            (frame.y * scale).round() as i32,
        ));
        layout.frame = Some(frame);
    }
    Ok(PillLayout {
        from: from.offset(-frame.x, -frame.y),
        to: to.offset(-frame.x, -frame.y),
    })
}

/// A `Moved` event for the widget. Our own reframes are ignored; any other move
/// (a drag, a corner placement) carries the bubble with it. Returns the saved
/// position for the new home, or `None` when nothing moved it but us.
pub fn on_moved(
    window: &WebviewWindow,
    position: PhysicalPosition<i32>,
) -> Option<PhysicalPosition<i32>> {
    let scale = window.scale_factor().ok()?;
    let mut layout = LAYOUT.lock().ok()?;
    let near = |&(x, y): &(i32, i32)| (x - position.x).abs() <= 1 && (y - position.y).abs() <= 1;
    if let Some(i) = layout.own_moves.iter().position(near) {
        layout.own_moves.drain(..=i);
        return None;
    }
    let moved_to = position.to_logical::<f64>(scale);
    match layout.frame {
        Some(frame) => {
            let (dx, dy) = (moved_to.x - frame.x, moved_to.y - frame.y);
            layout.frame = Some(frame.offset(dx, dy));
            layout.home = layout.home.map(|(x, y)| (x + dx, y + dy));
        }
        // Not laid out yet: the window is still the square it starts as.
        None => layout.home = None,
    }
    let home = layout.home.unwrap_or((
        moved_to.x + SAVED_SQUARE / 2.0,
        moved_to.y + SAVED_SQUARE / 2.0,
    ));
    Some(saved_position(home, scale))
}

/// Where the widget is saved: the top-left of the starting square around the
/// bubble, whatever size the window is right now.
pub fn current_saved_position(window: &WebviewWindow) -> Option<PhysicalPosition<i32>> {
    let scale = window.scale_factor().ok()?;
    let home = LAYOUT.lock().ok()?.home;
    match home {
        Some(home) => Some(saved_position(home, scale)),
        None => window.outer_position().ok(),
    }
}

fn saved_position(home: (f64, f64), scale: f64) -> PhysicalPosition<i32> {
    PhysicalPosition::new(
        ((home.0 - SAVED_SQUARE / 2.0) * scale).round() as i32,
        ((home.1 - SAVED_SQUARE / 2.0) * scale).round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: ScreenRect = ScreenRect {
        x: 0.0,
        y: 25.0,
        width: 1512.0,
        height: 920.0,
    };
    const BUBBLE: Extent = Extent {
        width: 44.0,
        height: 44.0,
    };
    const RECORDING: Extent = Extent {
        width: 172.0,
        height: 40.0,
    };
    const NO_ROOM: Extent = Extent {
        width: 0.0,
        height: 0.0,
    };

    #[test]
    fn a_pill_opens_centred_on_the_bubble_away_from_the_edges() {
        let pill = place_pill((700.0, 500.0), RECORDING, SCREEN, 8.0);
        assert_eq!((pill.x, pill.y), (614.0, 480.0));
    }

    #[test]
    fn a_bubble_at_the_right_edge_opens_into_a_whole_pill_on_screen() {
        let home = (1512.0 - 24.0, 500.0);
        let pill = place_pill(home, RECORDING, SCREEN, 8.0);
        assert_eq!(pill.right(), 1512.0 - 8.0);
        assert!(pill.x >= SCREEN.x);
    }

    #[test]
    fn a_bubble_at_the_left_and_top_edges_opens_inside_them() {
        let pill = place_pill((10.0, 30.0), RECORDING, SCREEN, 8.0);
        assert_eq!((pill.x, pill.y), (8.0, 33.0));
    }

    #[test]
    fn the_idle_bubble_may_sit_right_at_the_edge() {
        let home = (1512.0 - 22.0, 500.0);
        let bubble = place_pill(home, BUBBLE, SCREEN, 0.0);
        assert_eq!(bubble.right(), 1512.0);
        assert_eq!(bubble.x, home.0 - 22.0);
    }

    #[test]
    fn a_pill_wider_than_the_screen_starts_at_its_left_edge() {
        let wide = Extent {
            width: 2000.0,
            height: 40.0,
        };
        assert_eq!(place_pill((700.0, 500.0), wide, SCREEN, 8.0).x, 8.0);
    }

    #[test]
    fn the_window_hugs_the_pill_with_its_halo_margin() {
        let pill = place_pill((700.0, 500.0), BUBBLE, SCREEN, 0.0);
        let frame = frame_for(pill, NO_ROOM, SCREEN);
        assert_eq!(
            frame,
            ScreenRect {
                x: 654.0,
                y: 454.0,
                width: 92.0,
                height: 92.0
            }
        );
    }

    #[test]
    fn a_menu_near_the_edge_gets_its_room_on_screen_without_losing_the_pill() {
        let pill = place_pill((1512.0 - 30.0, 900.0), BUBBLE, SCREEN, 0.0);
        let menu = Extent {
            width: 460.0,
            height: 560.0,
        };
        let frame = frame_for(pill, menu, SCREEN);
        assert_eq!(frame.right(), 1512.0);
        assert_eq!(frame.bottom(), SCREEN.bottom());
        assert!(frame.x <= pill.x && frame.right() >= pill.right());
        assert!(frame.y <= pill.y && frame.bottom() >= pill.bottom());
    }

    #[test]
    fn a_morph_window_holds_both_the_bubble_and_the_pill() {
        let home = (1512.0 - 24.0, 500.0);
        let bubble = place_pill(home, BUBBLE, SCREEN, 0.0);
        let pill = place_pill(home, RECORDING, SCREEN, 8.0);
        let frame = frame_for(bubble.union(&pill), NO_ROOM, SCREEN);
        assert!(frame.x <= pill.x && frame.right() >= bubble.right());
    }

    #[test]
    fn the_saved_position_is_the_starting_square_around_the_bubble() {
        assert_eq!(
            saved_position((700.0, 500.0), 2.0),
            PhysicalPosition::new(1308, 908)
        );
    }
}
