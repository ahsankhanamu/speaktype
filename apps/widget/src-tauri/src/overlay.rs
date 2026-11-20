//! Always-on drawing (Show & Tell, S1): one transparent, click-through panel
//! over each display, and a listen-only key tap that hands the overlays the
//! mouse only while the draw key (Right ⌥ by default) is held. SpeakType never
//! takes the keyboard: the tap only listens, and only to modifier changes.

use crate::logging;
use crate::shortcuts::ShortcutAction;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Sent to the overlays when the draw key goes down or up.
pub const DRAW_EVENT: &str = "overlay:draw";

#[derive(Serialize, Clone)]
struct DrawState {
    active: bool,
}

/// The overlays' display frames (logical points), to rebuild after a display
/// is plugged in, removed or rearranged.
static FRAMES: Mutex<Vec<(f64, f64, f64, f64)>> = Mutex::new(Vec::new());
static DRAWING: AtomicBool = AtomicBool::new(false);

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
                make_overlay_panel(&window);
            }
            Err(e) => logging::log_message(&format!("[overlay] Could not create overlay {i}: {e}")),
        }
    }
    logging::log_message(&format!("[overlay] {} overlay(s) ready", frames.len()));
    *current = frames;
}

/// A non-activating panel above other apps (full-screen ones included) on
/// every Space, so drawing never steals focus from the app being explained.
#[cfg(target_os = "macos")]
fn make_overlay_panel(window: &tauri::WebviewWindow) {
    let _ = window.with_webview(|webview| unsafe {
        use objc2::runtime::AnyClass;
        use objc2_app_kit::{
            NSPanel, NSStatusWindowLevel, NSWindowCollectionBehavior, NSWindowStyleMask,
        };
        use objc2_foundation::NSObject;

        unsafe extern "C" {
            fn object_setClass(obj: *mut NSObject, cls: *const AnyClass) -> *const AnyClass;
        }
        let ns_window = webview.ns_window();
        let Some(panel_class) = AnyClass::get(c"NSPanel") else {
            return;
        };
        object_setClass(ns_window as *mut NSObject, panel_class as *const AnyClass);
        let panel = &*(ns_window as *const NSPanel);
        panel.setStyleMask(NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel);
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(true);
        panel.setHasShadow(false);
        panel.setLevel(NSStatusWindowLevel);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        panel.setIgnoresMouseEvents(true);
    });
}

#[cfg(not(target_os = "macos"))]
fn make_overlay_panel(_window: &tauri::WebviewWindow) {}

/// Hand the overlays the mouse (draw key down) or give it back (key up).
fn set_drawing(app: &AppHandle, active: bool) {
    if DRAWING.swap(active, Ordering::SeqCst) == active {
        return;
    }
    if active {
        ensure_overlays(app);
    }
    for (label, window) in app.webview_windows() {
        if label.starts_with("overlay-") {
            let _ = window.set_ignore_cursor_events(!active);
        }
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

/// A listen-only tap on modifier changes: it can't block or change a key, and
/// only modifier presses reach it.
#[cfg(target_os = "macos")]
fn run_draw_key_tap(app: AppHandle) -> Result<(), String> {
    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
        EventField,
    };

    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::TailAppendEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::FlagsChanged],
        move |_proxy, _type, event| {
            let key = current_draw_key(&app)?;
            let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
            if let Some(down) = draw_key_change(key, keycode, event.get_flags().bits()) {
                set_drawing(&app, down);
            }
            None
        },
    )
    .map_err(|_| {
        "CGEventTapCreate failed (Accessibility or Input Monitoring not granted)".to_string()
    })?;

    let source = tap
        .mach_port
        .create_runloop_source(0)
        .map_err(|e| format!("create_runloop_source failed: {e:?}"))?;
    let run_loop = CFRunLoop::get_current();
    unsafe { run_loop.add_source(&source, kCFRunLoopCommonModes) };
    tap.enable();
    logging::log_message("[overlay] Draw key tap listening");
    CFRunLoop::run_current();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
