//! Where the widget window and its pill sit on screen.
//!
//! `home` is where the person put the bubble: its centre, in logical points.
//! Every pill is placed around it and kept inside the display's visible frame,
//! so a bubble parked at the edge still opens into a whole pill.
//!
//! The window is a fixed transparent stage around home, big enough for the
//! pill, its tooltip and its menus, so hovering, opening a menu or morphing
//! never moves or resizes it (a moving window shifts the pill out from under
//! the pointer). Clicks pass through the stage except over the regions the
//! webview reports (the pill, the card, an open menu).

use crate::session_cmds::IpcError;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{PhysicalPosition, WebviewWindow};

/// The stage: room for the widest pill and the right-click menu or a picker.
const STAGE: Extent = Extent {
    width: 480.0,
    height: 600.0,
};
/// The window the widget starts at (`tauri.conf.json`): the saved position is
/// the top-left of this square centred on the bubble.
const SAVED_SQUARE: f64 = 92.0;

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
    fn contains(&self, (x, y): (f64, f64)) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
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

/// The stage around `home`: centred on it and moved inside `area`, on whole
/// points. It always holds home, so it holds every pill placed around it.
pub fn stage_for(home: (f64, f64), area: ScreenRect) -> ScreenRect {
    ScreenRect {
        x: clamp_span(
            home.0 - STAGE.width / 2.0,
            STAGE.width,
            area.x,
            area.right(),
        )
        .round(),
        y: clamp_span(
            home.1 - STAGE.height / 2.0,
            STAGE.height,
            area.y,
            area.bottom(),
        )
        .round(),
        width: STAGE.width,
        height: STAGE.height,
    }
}

#[derive(Default)]
struct Layout {
    /// Home relative to the window's origin. Home moves with the window (a
    /// drag, a corner placement), so it is always the window's real position
    /// plus this; no move events need to be tracked or matched.
    home_offset: Option<(f64, f64)>,
    /// The window's frame, in logical points, for hit testing.
    frame: Option<ScreenRect>,
    /// Where the window takes clicks, in its own coordinates.
    regions: Vec<ScreenRect>,
    /// The primary display's height in points, to flip AppKit's bottom-left y.
    primary_height: f64,
    /// Whether clicks currently pass through the window.
    passing: Option<bool>,
}

static LAYOUT: Mutex<Layout> = Mutex::new(Layout {
    home_offset: None,
    frame: None,
    regions: Vec::new(),
    primary_height: 0.0,
    passing: None,
});

/// Home for a window at `rect`: its offset in the window, or the window's
/// centre before the first layout (the window starts as a square around it).
fn home_in(rect: ScreenRect, offset: Option<(f64, f64)>) -> (f64, f64) {
    match offset {
        Some((dx, dy)) => (rect.x + dx, rect.y + dy),
        None => (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0),
    }
}

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

/// Where the widget may draw on the display `home` is on, in logical points:
/// the whole display below the menu bar. The Dock doesn't count: it rarely
/// spans the width, and moving a pill up past it would throw it far from
/// the bubble; staying on screen is all that matters.
fn work_area(window: &WebviewWindow, home: (f64, f64)) -> Option<ScreenRect> {
    let logical = |m: &tauri::Monitor| {
        let scale = m.scale_factor();
        let (pos, size) = (m.position(), m.size());
        let top = m.work_area().position.y as f64 / scale;
        let (x, y) = (pos.x as f64 / scale, pos.y as f64 / scale);
        let bottom = y + size.height as f64 / scale;
        ScreenRect {
            x,
            y: top.max(y),
            width: size.width as f64 / scale,
            height: bottom - top.max(y),
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
fn primary_height(window: &WebviewWindow) -> Option<f64> {
    let primary = window.primary_monitor().ok()??;
    Some(primary.size().height as f64 / primary.scale_factor())
}

fn set_frame(window: &WebviewWindow, frame: ScreenRect) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWindow;
        use objc2_foundation::{NSPoint, NSRect, NSSize};
        let primary_height = primary_height(window).ok_or("No display")?;
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

/// Place the pill of `size` around home, inside the stage, and return it in the
/// window's coordinates. The window only moves when home or the display does.
#[tauri::command]
#[specta::specta]
pub fn layout_widget(
    window: WebviewWindow,
    size: Extent,
    inset: f64,
) -> Result<ScreenRect, IpcError> {
    let fail = |e: String| IpcError::new("widgetLayout", e);
    let current = window_rect(&window).ok_or_else(|| fail("The widget has no frame.".into()))?;
    let mut layout = LAYOUT.lock().map_err(|e| fail(e.to_string()))?;
    let home = home_in(current, layout.home_offset);
    let area = work_area(&window, home).unwrap_or(current);
    let frame = stage_for(home, area);
    if layout.primary_height == 0.0 || !same_rect(current, frame) {
        layout.primary_height = primary_height(&window).unwrap_or(layout.primary_height);
    }
    if !same_rect(current, frame) {
        set_frame(&window, frame).map_err(fail)?;
    }
    layout.frame = Some(frame);
    layout.home_offset = Some((home.0 - frame.x, home.1 - frame.y));
    Ok(place_pill(home, size, area, inset).offset(-frame.x, -frame.y))
}

fn same_rect(a: ScreenRect, b: ScreenRect) -> bool {
    (a.x - b.x).abs() < 0.5
        && (a.y - b.y).abs() < 0.5
        && (a.width - b.width).abs() < 0.5
        && (a.height - b.height).abs() < 0.5
}

/// Where the widget takes clicks, in window coordinates; everywhere else they
/// reach the app underneath.
#[tauri::command]
#[specta::specta]
pub fn set_widget_hit_regions(window: WebviewWindow, regions: Vec<ScreenRect>) {
    if let Ok(mut layout) = LAYOUT.lock() {
        layout.regions = regions;
    }
    update_pass_through(&window);
}

/// Let clicks through unless the pointer is over a hit region. Runs on every
/// pointer move (see `watch_pointer`) and whenever the regions change.
fn update_pass_through(window: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        let point = objc2_app_kit::NSEvent::mouseLocation();
        let Ok(mut layout) = LAYOUT.lock() else {
            return;
        };
        let Some(frame) = layout.frame else {
            return;
        };
        let local = (point.x - frame.x, layout.primary_height - point.y - frame.y);
        let pass = !layout.regions.iter().any(|r| r.contains(local));
        // A held button means a press or drag on the widget is under way; the
        // window moves with a drag faster than its frame is updated here, so
        // switching to click-through now would cut the drag off.
        let button_down = objc2_app_kit::NSEvent::pressedMouseButtons() != 0;
        if layout.passing == Some(pass) || (pass && button_down && layout.passing == Some(false)) {
            return;
        }
        layout.passing = Some(pass);
        drop(layout);
        let _ = window.set_ignore_cursor_events(pass);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = window;
}

/// Follow the pointer with AppKit's event monitors: the global one while
/// another app is in front, the local one while SpeakType is. Event-driven, so
/// an idle pointer costs nothing. Installed once, for the app's lifetime.
#[cfg(target_os = "macos")]
pub fn watch_pointer(window: &WebviewWindow) {
    use block2::RcBlock;
    use objc2_app_kit::{NSEvent, NSEventMask};
    use std::ptr::NonNull;

    // Moves only: during a drag the regions are stale, and the button-up is
    // followed by a move that settles the state.
    let mask = NSEventMask::MouseMoved | NSEventMask::LeftMouseUp;
    let on_event = |window: &WebviewWindow, event: NonNull<NSEvent>| {
        // SAFETY: AppKit hands the monitor a live event for the call's duration.
        if unsafe { event.as_ref() }.r#type() == objc2_app_kit::NSEventType::LeftMouseUp {
            settle(window);
        }
        update_pass_through(window);
    };
    let global_window = window.clone();
    let global = RcBlock::new(move |event: NonNull<NSEvent>| on_event(&global_window, event));
    let local_window = window.clone();
    let local = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
        on_event(&local_window, event);
        event.as_ptr()
    });
    let monitors = (
        NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &global),
        // SAFETY: the handler returns the event it was given, unchanged.
        unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &local) },
    );
    std::mem::forget(monitors);
}

/// Asks the widget to lay out again.
pub const RELAYOUT_EVENT: &str = "widget:relayout";

/// After a button-up, a widget dropped near an edge hangs its stage past the
/// screen (a drag ends after `startDragging` returns, so the layout that
/// followed it ran too early): ask for a layout, which pulls the stage back on
/// screen so its pill and menus aren't cut off.
fn settle(window: &WebviewWindow) {
    let Some(current) = window_rect(window) else {
        return;
    };
    let Some(home) = LAYOUT
        .lock()
        .ok()
        .and_then(|l| l.home_offset)
        .map(|offset| home_in(current, Some(offset)))
    else {
        return;
    };
    let Some(area) = work_area(window, home) else {
        return;
    };
    if !same_rect(current, stage_for(home, area)) {
        use tauri::Emitter;
        let _ = window.emit_to(window.label(), RELAYOUT_EVENT, ());
    }
}

/// The widget moved (a drag, a corner placement, or our own layout). Home
/// moves with it; returns where to save it.
pub fn on_moved(window: &WebviewWindow) -> Option<PhysicalPosition<i32>> {
    let scale = window.scale_factor().ok()?;
    let live = window_rect(window)?;
    let mut layout = LAYOUT.lock().ok()?;
    if layout.frame.is_some() {
        layout.frame = Some(live);
    }
    Some(saved_position(home_in(live, layout.home_offset), scale))
}

/// Where the widget is saved: the top-left of the starting square around the
/// bubble, whatever size the window is right now.
pub fn current_saved_position(window: &WebviewWindow) -> Option<PhysicalPosition<i32>> {
    let scale = window.scale_factor().ok()?;
    let live = window_rect(window)?;
    let offset = LAYOUT.lock().ok()?.home_offset;
    Some(saved_position(home_in(live, offset), scale))
}

/// The square around the bubble that positions are saved and placed as, in
/// physical pixels.
pub fn square_size(window: &WebviewWindow) -> (i32, i32) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let side = (SAVED_SQUARE * scale).round() as i32;
    (side, side)
}

/// Move the widget so the square around the bubble has its top-left at
/// `square` (physical): the window keeps the bubble where it draws it.
pub fn move_square_to(window: &WebviewWindow, square: PhysicalPosition<i32>) -> tauri::Result<()> {
    let scale = window.scale_factor()?;
    let offset = LAYOUT.lock().ok().and_then(|l| l.home_offset);
    let corner = square.to_logical::<f64>(scale);
    let (x, y) = match offset {
        Some((dx, dy)) => (
            corner.x + SAVED_SQUARE / 2.0 - dx,
            corner.y + SAVED_SQUARE / 2.0 - dy,
        ),
        None => (corner.x, corner.y),
    };
    window.set_position(tauri::LogicalPosition::new(x, y))
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
    fn the_stage_holds_home_and_stays_on_screen_at_the_edges() {
        for home in [(700.0, 500.0), (1512.0 - 4.0, 30.0), (4.0, 940.0)] {
            let stage = stage_for(home, SCREEN);
            assert!(stage.contains(home), "{home:?} outside {stage:?}");
            assert!(stage.x >= SCREEN.x && stage.right() <= SCREEN.right());
            assert!(stage.y >= SCREEN.y && stage.bottom() <= SCREEN.bottom());
        }
    }

    #[test]
    fn every_pill_fits_in_the_stage_around_its_home() {
        let home = (1512.0 - 24.0, 40.0);
        let stage = stage_for(home, SCREEN);
        for (size, inset) in [(BUBBLE, 0.0), (RECORDING, 8.0)] {
            let pill = place_pill(home, size, SCREEN, inset);
            assert!(pill.x >= stage.x && pill.right() <= stage.right());
            assert!(pill.y >= stage.y && pill.bottom() <= stage.bottom());
        }
    }

    #[test]
    fn the_saved_position_is_the_starting_square_around_the_bubble() {
        assert_eq!(
            saved_position((700.0, 500.0), 2.0),
            PhysicalPosition::new(1308, 908)
        );
    }
}
