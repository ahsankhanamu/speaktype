use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[cfg(target_os = "macos")]
static SUPER_CONTROL_GEN: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "macos")]
static SUPER_CONTROL_LOOP: Mutex<Option<core_foundation::runloop::CFRunLoop>> = Mutex::new(None);

/// Which dictation shortcut is live. A modifier chord (`Super+Control`) runs a
/// CGEventTap, not a registry entry, so it is tracked as "no shortcut".
#[derive(Default)]
pub(crate) struct AppHotkey {
    /// The last requested hotkey, normalized; `None` until startup registers.
    pub(crate) hotkey: Option<String>,
    /// The shortcut currently registered in the global registry.
    pub(crate) shortcut: Option<String>,
}

/// One step of a dictation-hotkey change, decided without touching the OS.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HotkeyChange {
    /// Leave the registry alone (same shortcut, or nothing was registered).
    Keep,
    /// Remove this shortcut, then (optionally) register the one in `register`.
    Swap {
        unregister: Option<String>,
        register: Option<String>,
    },
}

/// What moving from `current` to `new` requires. An invalid `new` is an
/// `Err`, so the live shortcut is never replaced.
pub(crate) fn hotkey_change(current: &AppHotkey, new: &str) -> Result<HotkeyChange, String> {
    let new = &crate::settings::Settings::normalize_hotkey(new)[..];
    if is_modifier_chord(new) {
        return Ok(match current.shortcut.as_ref() {
            Some(old) => HotkeyChange::Swap {
                unregister: Some(old.clone()),
                register: None,
            },
            None => HotkeyChange::Keep,
        });
    }
    new.parse::<Shortcut>().map_err(|e| e.to_string())?;
    Ok(if current.shortcut.as_deref() == Some(new) {
        HotkeyChange::Keep
    } else {
        HotkeyChange::Swap {
            unregister: current.shortcut.clone(),
            register: Some(new.to_string()),
        }
    })
}

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

/// Move the dictation hotkey to `hotkey_str`, touching only the previous
/// dictation shortcut: the screenshot and session shortcuts survive.
/// `live` is updated to reflect the result; a failed arm leaves no shortcut
/// recorded, so the next attempt registers it.
/// Super+Control uses a dedicated CGEventTap thread; switching away stops the
/// old tap and switching to it starts the new one.
pub fn reregister_app_hotkey(
    app: &AppHandle,
    live: &mut AppHotkey,
    hotkey_str: &str,
    is_recording: Arc<AtomicBool>,
) -> bool {
    let change = match hotkey_change(live, hotkey_str) {
        Ok(change) => change,
        Err(e) => {
            crate::logging::log_message(&format!(
                "[shortcut] Invalid hotkey {:?}: {}",
                hotkey_str, e
            ));
            return false;
        }
    };
    if let HotkeyChange::Swap {
        unregister: Some(old),
        ..
    } = &change
    {
        match parse_app_hotkey(old) {
            Ok(shortcut) => {
                if let Err(e) = app.global_shortcut().unregister(shortcut) {
                    crate::logging::log_message(&format!(
                        "[shortcut] Failed to unregister {}: {}",
                        old, e
                    ));
                }
            }
            Err(e) => crate::logging::log_message(&format!(
                "[shortcut] Failed to unregister {:?}: {}",
                old, e
            )),
        }
    }
    #[cfg(target_os = "macos")]
    stop_macos_super_control_listener();
    let registered = register_app_hotkey(app, hotkey_str, is_recording);
    let armed = match &change {
        HotkeyChange::Keep => live.shortcut.clone(),
        HotkeyChange::Swap {
            register: Some(new),
            ..
        } => registered.then(|| new.clone()),
        HotkeyChange::Swap { register: None, .. } => None,
    };
    *live = AppHotkey {
        hotkey: Some(crate::settings::Settings::normalize_hotkey(hotkey_str)),
        shortcut: armed,
    };
    registered
}

/// First arm at startup: the dictation hotkey from settings.
pub fn arm_initial_hotkey(app: &AppHandle, live: &mut AppHotkey, is_recording: Arc<AtomicBool>) {
    let hotkey = live.hotkey.clone().unwrap_or_default();
    live.shortcut = None;
    if !register_app_hotkey(app, &hotkey, is_recording) {
        crate::logging::log_message(&format!("[shortcut] Could not arm {:?} at startup", hotkey));
    }
    live.shortcut = (!is_modifier_chord(&hotkey)).then_some(hotkey);
}

/// Re-arm after macOS grants Accessibility while running. The shortcut is
/// unchanged, so its registry entry is replaced in place; the tap that failed
/// without the grant is started now.
pub fn rearm_after_accessibility_grant(
    app: &AppHandle,
    live: &mut AppHotkey,
    is_recording: Arc<AtomicBool>,
) {
    let hotkey = live.hotkey.clone().unwrap_or_default();
    live.shortcut = None;
    if register_app_hotkey(app, &hotkey, is_recording) {
        live.shortcut = (!is_modifier_chord(&hotkey)).then_some(hotkey);
    } else {
        crate::logging::log_message("[shortcut] Re-arm after Accessibility grant failed");
    }
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
    match app
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
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
            crate::logging::log_message(&format!("[shortcut] Failed to register {}: {}", label, e));
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
fn start_macos_super_control_listener(app: AppHandle, is_recording: Arc<AtomicBool>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const SCREENSHOT: &str = "Super+Option+S";

    /// The app's global-shortcut table, which also holds the screenshot
    /// shortcut. `unregister_all_calls` proves the bulk wipe is never used.
    #[derive(Default)]
    struct FakeRegistry {
        registered: BTreeSet<String>,
        unregistered: Vec<String>,
        unregister_all_calls: usize,
    }

    impl FakeRegistry {
        fn with_screenshot() -> Self {
            FakeRegistry {
                registered: BTreeSet::from([SCREENSHOT.to_string()]),
                ..Default::default()
            }
        }

        fn registered(&self) -> Vec<String> {
            self.registered.iter().cloned().collect()
        }
    }

    fn apply(registry: &mut FakeRegistry, change: HotkeyChange) {
        if let HotkeyChange::Swap {
            unregister,
            register,
        } = change
        {
            if let Some(old) = unregister {
                registry.registered.remove(&old);
                registry.unregistered.push(old);
            }
            if let Some(new) = register {
                registry.registered.insert(new);
            }
        }
    }

    fn state(shortcut: &str) -> AppHotkey {
        AppHotkey {
            hotkey: Some(shortcut.to_string()),
            shortcut: Some(shortcut.to_string()),
        }
    }

    fn chord() -> AppHotkey {
        AppHotkey {
            hotkey: Some("Super+Control".to_string()),
            shortcut: None,
        }
    }

    #[test]
    fn reregistering_the_dictation_hotkey_keeps_other_shortcuts() {
        let mut registry = FakeRegistry::with_screenshot();

        apply(
            &mut registry,
            hotkey_change(&AppHotkey::default(), "F9").expect("F9 is valid"),
        );
        let change = hotkey_change(&state("F9"), "F8").expect("F8 is valid");
        assert_eq!(
            change,
            HotkeyChange::Swap {
                unregister: Some("F9".to_string()),
                register: Some("F8".to_string()),
            }
        );
        apply(&mut registry, change);

        assert_eq!(
            registry.registered(),
            vec!["F8".to_string(), SCREENSHOT.to_string()]
        );
        assert_eq!(registry.unregistered, vec!["F9".to_string()]);
        assert_eq!(registry.unregister_all_calls, 0, "never wipe the registry");
    }

    #[test]
    fn same_hotkey_again_touches_nothing() {
        assert_eq!(hotkey_change(&state("F9"), "F9"), Ok(HotkeyChange::Keep));
    }

    #[test]
    fn switching_to_the_modifier_chord_unregisters_only_the_old_shortcut() {
        let mut registry = FakeRegistry::with_screenshot();

        let change = hotkey_change(&state("F8"), "Super+Control").expect("the chord is valid");
        assert_eq!(
            change,
            HotkeyChange::Swap {
                unregister: Some("F8".to_string()),
                register: None,
            }
        );
        apply(&mut registry, change);

        assert_eq!(registry.registered(), vec![SCREENSHOT.to_string()]);
        assert_eq!(registry.unregistered, vec!["F8".to_string()]);
    }

    #[test]
    fn leaping_from_the_modifier_chord_to_a_shortcut_registers_only_the_new_one() {
        assert_eq!(
            hotkey_change(&chord(), "F9"),
            Ok(HotkeyChange::Swap {
                unregister: None,
                register: Some("F9".to_string()),
            })
        );
    }

    #[test]
    fn the_chord_to_itself_keeps_the_tap() {
        assert_eq!(
            hotkey_change(&chord(), "Super+Control"),
            Ok(HotkeyChange::Keep)
        );
    }

    #[test]
    fn an_invalid_hotkey_errors_and_registers_nothing() {
        let mut registry = FakeRegistry::with_screenshot();

        apply(
            &mut registry,
            hotkey_change(&AppHotkey::default(), "F9").expect("F9 is valid"),
        );

        assert!(hotkey_change(&state("F9"), "not a key").is_err());

        assert_eq!(
            registry.registered(),
            vec!["F9".to_string(), SCREENSHOT.to_string()]
        );
        assert!(registry.unregistered.is_empty());
    }
}
