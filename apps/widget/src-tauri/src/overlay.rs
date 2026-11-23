//! Always-on drawing (Show & Tell, S1): one transparent window over each
//! display that only shows ink and never takes a click, and an event tap that
//! turns left-button strokes into ink while the draw key (Right ⌥ by default)
//! is held.
//!
//! The overlays never receive mouse events, so drawing can't activate SpeakType
//! (which would move focus off the app being explained and make macOS switch to
//! the desktop holding SpeakType's pill). The tap passes every key through, and
//! swallows only the button events of a stroke drawn with the key held.

use crate::logging;
use crate::shortcuts::ShortcutAction;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Sent to the overlays when the draw key goes down or up.
pub const DRAW_EVENT: &str = "overlay:draw";
/// Sent to one overlay for each point of a stroke, in that window's points.
pub const STROKE_EVENT: &str = "overlay:stroke";

#[derive(Serialize, Clone)]
struct DrawState {
    active: bool,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum StrokePhase {
    Down,
    Move,
    Up,
}

#[derive(Serialize, Clone, Copy)]
struct StrokePoint {
    phase: StrokePhase,
    x: f64,
    y: f64,
}

/// The overlays' display frames (logical points), to rebuild after a display
/// is plugged in, removed or rearranged.
static FRAMES: Mutex<Vec<(f64, f64, f64, f64)>> = Mutex::new(Vec::new());
static DRAWING: AtomicBool = AtomicBool::new(false);
/// The display the current stroke is on while its button is down (-1: none).
/// A stroke stays on the display it began on, and every button event of it is
/// swallowed, even after the key comes up, so no app sees half a click.
static STROKE_DISPLAY: AtomicI64 = AtomicI64::new(-1);

/// A draw key and how to read it from a flags-changed event: its virtual key
/// code, and the device-dependent flag bit that is set while it is down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawKey {
    keycode: i64,
    mask: u64,
}

/// `right_option` | `right_command` | `fn` (the stored `drawKey` values).
pub fn draw_key(value: &str) -> Option<DrawKey> {
    match value {
        // NX_DEVICERALTKEYMASK, NX_DEVICERCMDKEYMASK, NX_SECONDARYFNMASK.
        "right_option" => Some(DrawKey {
            keycode: 61,
            mask: 0x40,
        }),
        "right_command" => Some(DrawKey {
            keycode: 54,
            mask: 0x10,
        }),
        "fn" => Some(DrawKey {
            keycode: 63,
            mask: 0x0080_0000,
        }),
        _ => None,
    }
}

/// Whether a flags-changed event for `keycode` with `flags` means the draw key
/// is now down (`Some(true)`), up (`Some(false)`), or is about another key.
pub fn draw_key_change(key: DrawKey, keycode: i64, flags: u64) -> Option<bool> {
    (keycode == key.keycode).then_some(flags & key.mask != 0)
}

/// The display containing a global point (logical points, top-left origin),
/// and the point in that display's own coordinates.
fn locate(frames: &[(f64, f64, f64, f64)], x: f64, y: f64) -> Option<(usize, f64, f64)> {
    frames
        .iter()
        .position(|&(fx, fy, w, h)| x >= fx && x < fx + w && y >= fy && y < fy + h)
        .map(|i| (i, x - frames[i].0, y - frames[i].1))
}

/// A left-button event while the tap runs.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Button {
    Down,
    Drag,
    Up,
}

/// What to do with a button event: `None` lets it through to the app under
/// the pointer; `Some((display, phase))` swallows it, and the phase is drawn
/// on that display unless the stroke already ended (`phase` is `None`).
fn route(
    button: Button,
    drawing: bool,
    stroke: Option<usize>,
    hit: Option<usize>,
) -> Option<(usize, Option<StrokePhase>)> {
    match (button, stroke) {
        // A press with the key held starts a stroke on the display under it.
        (Button::Down, None) if drawing => hit.map(|d| (d, Some(StrokePhase::Down))),
        (Button::Down, None) => None,
        (Button::Down, Some(d)) => Some((d, None)),
        (Button::Drag, Some(d)) => Some((d, drawing.then_some(StrokePhase::Move))),
        (Button::Up, Some(d)) => Some((d, drawing.then_some(StrokePhase::Up))),
        // Drags and releases of a press the app received stay the app's.
        (Button::Drag | Button::Up, None) => None,
    }
}

/// Send a stroke point at global (`x`, `y`) to its display's overlay.
fn emit_stroke(app: &AppHandle, display: usize, phase: StrokePhase, x: f64, y: f64) {
    let Some((fx, fy, _, _)) = FRAMES.lock().ok().and_then(|f| f.get(display).copied()) else {
        return;
    };
    let point = StrokePoint {
        phase,
        x: x - fx,
        y: y - fy,
    };
    let _ = app.emit_to(format!("overlay-{display}"), STROKE_EVENT, point);
}

fn emit_stroke_end(app: &AppHandle, display: usize) {
    let end = StrokePoint {
        phase: StrokePhase::Up,
        x: 0.0,
        y: 0.0,
    };
    let _ = app.emit_to(format!("overlay-{display}"), STROKE_EVENT, end);
}

/// Handle one left-button event at global point (`x`, `y`); true swallows it.
fn on_button(app: &AppHandle, button: Button, x: f64, y: f64) -> bool {
    let stroke = usize::try_from(STROKE_DISPLAY.load(Ordering::SeqCst)).ok();
    let hit = FRAMES
        .lock()
        .ok()
        .and_then(|frames| locate(&frames, x, y))
        .map(|(display, _, _)| display);
    let Some((display, phase)) = route(button, DRAWING.load(Ordering::SeqCst), stroke, hit) else {
        return false;
    };
    match button {
        Button::Down => STROKE_DISPLAY.store(display as i64, Ordering::SeqCst),
        Button::Up => STROKE_DISPLAY.store(-1, Ordering::SeqCst),
        Button::Drag => {}
    }
    if let Some(phase) = phase {
        emit_stroke(app, display, phase, x, y);
    }
    true
}

fn monitor_frames(app: &AppHandle) -> Vec<(f64, f64, f64, f64)> {
    let Some(window) = app.get_webview_window("main") else {
        return Vec::new();
    };
    window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let scale = m.scale_factor();
            let (pos, size) = (m.position(), m.size());
            (
                pos.x as f64 / scale,
                pos.y as f64 / scale,
                size.width as f64 / scale,
                size.height as f64 / scale,
            )
        })
        .collect()
}

/// Make sure there is one overlay per display, rebuilding them if the
/// displays changed. Cheap when nothing changed.
fn ensure_overlays(app: &AppHandle) {
    let frames = monitor_frames(app);
    let Ok(mut current) = FRAMES.lock() else {
        return;
    };
    if *current == frames && !frames.is_empty() {
        return;
    }
    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-") {
            let _ = window.close();
        }
    }
    for (i, &(x, y, width, height)) in frames.iter().enumerate() {
        match WebviewWindowBuilder::new(
            app,
            format!("overlay-{i}"),
            WebviewUrl::App("overlay.html".into()),
        )
        .title("SpeakType Ink")
        .position(x, y)
        .inner_size(width, height)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .focused(false)
        .always_on_top(true)
        .visible(true)
        .build()
        {
            Ok(window) => {
                let _ = window.set_ignore_cursor_events(true);
                float_over_everything(&window);
            }
            Err(e) => logging::log_message(&format!("[overlay] Could not create overlay {i}: {e}")),
        }
    }
    logging::log_message(&format!("[overlay] {} overlay(s) ready", frames.len()));
    *current = frames;
}

/// Above other apps (full-screen ones included), on every Space, and never a
/// target for the mouse: the window only shows ink.
#[cfg(target_os = "macos")]
fn float_over_everything(window: &tauri::WebviewWindow) {
    let _ = window.with_webview(|webview| unsafe {
        use objc2_app_kit::{NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior};
        let ns_window = &*(webview.ns_window() as *const NSWindow);
        ns_window.setHasShadow(false);
        ns_window.setLevel(NSStatusWindowLevel);
        ns_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        ns_window.setIgnoresMouseEvents(true);
    });
}

#[cfg(not(target_os = "macos"))]
fn float_over_everything(_window: &tauri::WebviewWindow) {}

/// The draw key went down or up. A stroke in progress ends with the key.
fn set_drawing(app: &AppHandle, active: bool) {
    if DRAWING.swap(active, Ordering::SeqCst) == active {
        return;
    }
    logging::log_message(&format!(
        "[overlay] Draw key {}",
        if active { "down" } else { "up" }
    ));
    if active {
        ensure_overlays(app);
    } else if let Ok(display) = usize::try_from(STROKE_DISPLAY.load(Ordering::SeqCst)) {
        // The button is still down: end the ink here; its release is swallowed.
        emit_stroke_end(app, display);
    }
    let _ = app.emit(DRAW_EVENT, DrawState { active });
}

/// The draw key and whether drawing is on, read from the live settings.
fn current_draw_key(app: &AppHandle) -> Option<DrawKey> {
    let state = app.try_state::<crate::commands::AppState>()?;
    let settings = state.settings.lock().ok()?;
    // With "Draw anytime" off, drawing still works inside a session.
    if !settings.draw_anytime {
        let store = app.try_state::<std::sync::Arc<speaktype_session::Store>>()?;
        if !matches!(store.active_session(), Ok(Some(_))) {
            return None;
        }
    }
    let value = settings
        .shortcuts
        .get(&ShortcutAction::DrawKey)
        .cloned()
        .unwrap_or_else(|| ShortcutAction::DrawKey.default_keys().to_string());
    draw_key(&value)
}

/// Build the overlays (when always-on drawing is on) and start the draw-key
/// tap. Called once at startup.
pub fn start(app: &AppHandle) {
    if current_draw_key(app).is_some() {
        ensure_overlays(app);
    }
    #[cfg(target_os = "macos")]
    {
        let app = app.clone();
        std::thread::spawn(move || {
            if let Err(e) = run_draw_key_tap(app) {
                logging::log_message(&format!("[overlay] Draw key tap failed: {e}"));
            }
        });
    }
}

/// One tap for the draw key and the strokes. Key events always pass through
/// unchanged; button events are swallowed only as `route` decides.
#[cfg(target_os = "macos")]
fn run_draw_key_tap(app: AppHandle) -> Result<(), String> {
    use core_foundation::base::TCFType;
    use core_foundation::mach_port::CFMachPortRef;
    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
        CallbackResult, EventField,
    };
    use std::sync::atomic::AtomicPtr;

    unsafe extern "C" {
        fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    }
    // macOS turns a tap off if a callback is ever slow; it's turned back on so
    // drawing (and the key) keep working.
    static TAP: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![
            CGEventType::FlagsChanged,
            CGEventType::LeftMouseDown,
            CGEventType::LeftMouseDragged,
            CGEventType::LeftMouseUp,
        ],
        move |_proxy, event_type, event| {
            let button = match event_type {
                CGEventType::LeftMouseDown => Button::Down,
                CGEventType::LeftMouseDragged => Button::Drag,
                CGEventType::LeftMouseUp => Button::Up,
                CGEventType::FlagsChanged => {
                    if let Some(key) = current_draw_key(&app) {
                        let keycode =
                            event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                        if let Some(down) = draw_key_change(key, keycode, event.get_flags().bits())
                        {
                            set_drawing(&app, down);
                        }
                    }
                    return CallbackResult::Keep;
                }
                CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
                    let port = TAP.load(Ordering::SeqCst);
                    if !port.is_null() {
                        unsafe { CGEventTapEnable(port as CFMachPortRef, true) };
                    }
                    logging::log_message("[overlay] Draw tap was turned off by macOS; back on");
                    return CallbackResult::Keep;
                }
                _ => return CallbackResult::Keep,
            };
            let at = event.location();
            if on_button(&app, button, at.x, at.y) {
                CallbackResult::Drop
            } else {
                CallbackResult::Keep
            }
        },
    )
    .map_err(|_| "CGEventTapCreate failed (Accessibility not granted)".to_string())?;
    TAP.store(
        tap.mach_port().as_concrete_TypeRef() as *mut std::ffi::c_void,
        Ordering::SeqCst,
    );

    let source = tap
        .mach_port()
        .create_runloop_source(0)
        .map_err(|e| format!("create_runloop_source failed: {e:?}"))?;
    let run_loop = CFRunLoop::get_current();
    unsafe { run_loop.add_source(&source, kCFRunLoopCommonModes) };
    tap.enable();
    logging::log_message("[overlay] Draw tap listening");
    CFRunLoop::run_current();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_DISPLAYS: [(f64, f64, f64, f64); 2] =
        [(0.0, 0.0, 1512.0, 982.0), (1512.0, -200.0, 2560.0, 1440.0)];

    #[test]
    fn a_point_lands_on_its_display_in_that_display_s_coordinates() {
        assert_eq!(locate(&TWO_DISPLAYS, 10.0, 20.0), Some((0, 10.0, 20.0)));
        assert_eq!(
            locate(&TWO_DISPLAYS, 1600.0, -100.0),
            Some((1, 88.0, 100.0))
        );
        assert_eq!(locate(&TWO_DISPLAYS, -5.0, 0.0), None);
    }

    #[test]
    fn without_the_draw_key_every_click_reaches_the_app() {
        assert_eq!(route(Button::Down, false, None, Some(0)), None);
        assert_eq!(route(Button::Drag, false, None, Some(0)), None);
        assert_eq!(route(Button::Up, false, None, Some(0)), None);
    }

    #[test]
    fn a_stroke_with_the_key_held_is_drawn_and_never_reaches_the_app() {
        assert_eq!(
            route(Button::Down, true, None, Some(1)),
            Some((1, Some(StrokePhase::Down)))
        );
        assert_eq!(
            route(Button::Drag, true, Some(1), Some(0)),
            Some((1, Some(StrokePhase::Move)))
        );
        assert_eq!(
            route(Button::Up, true, Some(1), Some(1)),
            Some((1, Some(StrokePhase::Up)))
        );
    }

    #[test]
    fn releasing_the_key_mid_stroke_still_swallows_the_rest_of_that_click() {
        assert_eq!(
            route(Button::Drag, false, Some(0), Some(0)),
            Some((0, None))
        );
        assert_eq!(route(Button::Up, false, Some(0), Some(0)), Some((0, None)));
    }

    #[test]
    fn a_press_the_app_received_keeps_its_drag_and_release() {
        assert_eq!(route(Button::Drag, true, None, Some(0)), None);
        assert_eq!(route(Button::Up, true, None, Some(0)), None);
    }

    #[test]
    fn every_stored_draw_key_reads_its_own_modifier() {
        for value in ["right_option", "right_command", "fn"] {
            assert!(draw_key(value).is_some(), "{value}");
        }
        assert_eq!(draw_key("left_option"), None);
    }

    #[test]
    fn right_option_down_and_up_toggle_drawing() {
        let key = draw_key("right_option").unwrap();
        // Right ⌥ down: Alternate (0x80000) plus the right-hand device bit.
        assert_eq!(draw_key_change(key, 61, 0x0008_0140), Some(true));
        assert_eq!(draw_key_change(key, 61, 0x0000_0100), Some(false));
    }

    #[test]
    fn left_option_never_draws() {
        let key = draw_key("right_option").unwrap();
        // Left ⌥ is keycode 58 with the left-hand device bit (0x20).
        assert_eq!(draw_key_change(key, 58, 0x0008_0120), None);
    }

    #[test]
    fn right_command_reads_its_own_device_bit() {
        let key = draw_key("right_command").unwrap();
        assert_eq!(draw_key_change(key, 54, 0x0010_0110), Some(true));
        assert_eq!(draw_key_change(key, 54, 0x0000_0100), Some(false));
    }
}
