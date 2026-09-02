use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[cfg(target_os = "macos")]
static SUPER_CONTROL_GEN: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "macos")]
static SUPER_CONTROL_LOOP: Mutex<Option<core_foundation::runloop::CFRunLoop>> = Mutex::new(None);

pub fn is_modifier_chord(hotkey: &str) -> bool {
    crate::settings::Settings::normalize_hotkey(hotkey) == "Super+Control"
}

/// Parse a stored hotkey string into a registerable global shortcut.
/// Modifier-only chords (`Super+Control`) are handled separately.
pub fn parse_app_hotkey(hotkey: &str) -> Result<Shortcut, String> {
    let normalized = crate::settings::Settings::normalize_hotkey(hotkey);
    if is_modifier_chord(&normalized) {
        return Err("modifier chord uses dedicated listener".into());
    }
    normalized.parse::<Shortcut>().map_err(|e| e.to_string())
}

/// Unregister all global shortcuts and re-register for `hotkey_str`.
/// Super+Control uses a dedicated CGEventTap thread; switching away stops the old tap.
pub fn reregister_app_hotkey(
    app: &AppHandle,
    hotkey_str: &str,
    is_recording: Arc<AtomicBool>,
) -> bool {
    if let Err(e) = app.global_shortcut().unregister_all() {
        crate::logging::log_message(&format!(
            "[shortcut] Failed to unregister existing shortcuts: {}",
            e
        ));
    }
    #[cfg(target_os = "macos")]
    stop_macos_super_control_listener();
    register_app_hotkey(app, hotkey_str, is_recording)
}

pub fn register_app_hotkey(
    app: &AppHandle,
    hotkey_str: &str,
    is_recording: Arc<AtomicBool>,
) -> bool {
    if is_modifier_chord(hotkey_str) {
        #[cfg(target_os = "macos")]
        {
            start_macos_super_control_listener(app.clone(), is_recording);
            crate::logging::log_message("[shortcut] Registered: Super+Control (modifier chord)");
            return true;
        }
        #[cfg(not(target_os = "macos"))]
        {
            crate::logging::log_message(
                "[shortcut] Super+Control modifier chord is macOS-only; falling back to F9",
            );
            if let Ok(shortcut) = "F9".parse::<Shortcut>() {
                return register_global_shortcut(app, shortcut, is_recording, "F9");
            }
            return false;
        }
    }

    match parse_app_hotkey(hotkey_str) {
        Ok(shortcut) => register_global_shortcut(app, shortcut, is_recording, hotkey_str),
        Err(e) => {
            crate::logging::log_message(&format!(
                "[shortcut] Invalid hotkey {:?}: {}",
                hotkey_str, e
            ));
            false
        }
    }
}

fn register_global_shortcut(
    app: &AppHandle,
    shortcut: Shortcut,
    is_recording: Arc<AtomicBool>,
    label: &str,
) -> bool {
    let app_handle = app.clone();
    match app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
        if event.state != ShortcutState::Pressed {
            return;
        }
        spawn_toggle(app_handle.clone(), is_recording.clone());
    }) {
        Ok(()) => {
            crate::logging::log_message(&format!("[shortcut] Registered: {}", label));
            true
        }
        Err(e) => {
            crate::logging::log_message(&format!(
                "[shortcut] Failed to register {}: {}",
                label, e
            ));
            false
        }
    }
}

fn spawn_toggle(app: AppHandle, _is_recording: Arc<AtomicBool>) {
    let app_main = app.clone();
    let _ = app.run_on_main_thread(move || {
        let state = app_main.state::<crate::commands::AppState>();
        if let Err(e) = crate::commands::toggle_recording_impl(&app_main, &state) {
            crate::logging::log_message(&format!("[shortcut] toggle_recording error: {}", e));
        }
    });
}

#[cfg(target_os = "macos")]
fn stop_macos_super_control_listener() {
    SUPER_CONTROL_GEN.fetch_add(1, Ordering::SeqCst);
    if let Ok(guard) = SUPER_CONTROL_LOOP.lock() {
        if let Some(ref run_loop) = *guard {
            run_loop.stop();
        }
    }
}

#[cfg(target_os = "macos")]
fn start_macos_super_control_listener(
    app: AppHandle,
    is_recording: Arc<AtomicBool>,
) {
    stop_macos_super_control_listener();
    let my_gen = SUPER_CONTROL_GEN.load(Ordering::SeqCst);
    std::thread::spawn(move || {
        if let Err(e) = run_macos_super_control_tap(app, is_recording, my_gen) {
            crate::logging::log_message(&format!(
                "[shortcut] Super+Control listener failed: {}",
                e
            ));
        }
    });
}

#[cfg(target_os = "macos")]
fn run_macos_super_control_tap(
    app: AppHandle,
    is_recording: Arc<AtomicBool>,
    my_gen: u64,
) -> Result<(), String> {
    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use core_graphics::event::{
        CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        CGEventType,
    };

    let both_were_down = Arc::new(AtomicBool::new(false));
    let both_flag = both_were_down.clone();
    let app_for_callback = app.clone();
    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![CGEventType::FlagsChanged],
        move |_proxy, _event_type, event| {
            if SUPER_CONTROL_GEN.load(Ordering::SeqCst) != my_gen {
                return None;
            }
            let flags = event.get_flags();
            let command = flags.contains(CGEventFlags::CGEventFlagCommand);
            let control = flags.contains(CGEventFlags::CGEventFlagControl);
            let both_now = command && control;
            let was_down = both_flag.load(Ordering::SeqCst);
            if both_now && !was_down {
                spawn_toggle(app_for_callback.clone(), is_recording.clone());
            }
            both_flag.store(both_now, Ordering::SeqCst);
            None
        },
    )
    .map_err(|_| "CGEventTapCreate failed (grant Accessibility permission)".to_string())?;

    let loop_source = tap
        .mach_port
        .create_runloop_source(0)
        .map_err(|e| format!("create_runloop_source failed: {:?}", e))?;

    let run_loop = CFRunLoop::get_current();
    if let Ok(mut guard) = SUPER_CONTROL_LOOP.lock() {
        *guard = Some(run_loop.clone());
    }
    unsafe {
        run_loop.add_source(&loop_source, kCFRunLoopCommonModes);
    }
    tap.enable();
    CFRunLoop::run_current();
    if let Ok(mut guard) = SUPER_CONTROL_LOOP.lock() {
        *guard = None;
    }
    Ok(())
}
