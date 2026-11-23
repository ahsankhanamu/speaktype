use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::shortcuts::ShortcutAction;

#[cfg(target_os = "macos")]
static SUPER_CONTROL_GEN: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "macos")]
static SUPER_CONTROL_LOOP: Mutex<Option<core_foundation::runloop::CFRunLoop>> = Mutex::new(None);

/// The app's registration: the dictation chord rides a CGEventTap, everything
/// else the global-shortcut plugin.
pub(crate) struct AppShortcuts {
    pub(crate) app: AppHandle,
    pub(crate) is_recording: Arc<AtomicBool>,
}

impl crate::shortcuts::ShortcutsPlatform for AppShortcuts {
    fn arm(&self, action: ShortcutAction, keys: &str) -> bool {
        if crate::shortcuts::uses_modifier_tap(action, keys) {
            #[cfg(target_os = "macos")]
            {
                start_macos_super_control_listener(self.app.clone(), self.is_recording.clone());
                crate::logging::log_message(
                    "[shortcut] Registered: Super+Control (modifier chord)",
                );
                return true;
            }
            #[cfg(not(target_os = "macos"))]
            {
                crate::logging::log_message(
                    "[shortcut] Super+Control modifier chord is macOS-only; falling back to F9",
                );
                return plugin_on(&self.app, "F9", crate::commands::press_dictation);
            }
        }
        let press = match action {
            ShortcutAction::Screenshot => crate::commands::press_screenshot,
            ShortcutAction::Session => crate::commands::press_session,
            _ => crate::commands::press_dictation,
        };
        plugin_on(&self.app, keys, press)
    }

    fn disarm(&self, keys: &str) {
        #[cfg(target_os = "macos")]
        if crate::shortcuts::uses_modifier_tap(ShortcutAction::Dictation, keys) {
            stop_macos_super_control_listener();
            return;
        }
        let Some(spec) = crate::shortcuts::spec_keys(keys) else {
            return;
        };
        if let Ok(shortcut) = spec.parse::<Shortcut>() {
            if let Err(e) = self.app.global_shortcut().unregister(shortcut) {
                crate::logging::log_message(&format!(
                    "[shortcut] Failed to unregister {}: {}",
                    keys, e
                ));
            }
        }
    }
}

/// Register one plugin shortcut for one action's press.
fn plugin_on(app: &AppHandle, keys: &str, press: fn(&AppHandle)) -> bool {
    let Some(spec) = crate::shortcuts::spec_keys(keys) else {
        crate::logging::log_message(&format!("[shortcut] {} is not registerable", keys));
        return false;
    };
    let Ok(shortcut) = spec.parse::<Shortcut>() else {
        crate::logging::log_message(&format!("[shortcut] {} is not a shortcut", keys));
        return false;
    };
    let app_handle = app.clone();
    match app
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            press(&app_handle);
        }) {
        Ok(()) => {
            crate::logging::log_message(&format!("[shortcut] Registered: {}", keys));
            true
        }
        Err(e) => {
            crate::logging::log_message(&format!("[shortcut] Failed to register {}: {}", keys, e));
            false
        }
    }
}

/// The shortcut registrations the Esc cancel lifecycle needs. A trait so the
/// arm/release rule is testable without touching a real keyboard.
pub trait HotkeyRegistration {
    fn register(&self, hotkey: &str, is_recording: Arc<AtomicBool>) -> bool;
    fn unregister_escape(&self) -> Result<(), String>;
}

/// The app's own registration: `"Escape"` is the cancel shortcut, anything
/// else is a global shortcut.
pub struct EscRegistration {
    pub app: AppHandle,
}

impl HotkeyRegistration for EscRegistration {
    fn register(&self, hotkey: &str, _is_recording: Arc<AtomicBool>) -> bool {
        if hotkey != "Escape" {
            return plugin_on(&self.app, hotkey, crate::commands::press_dictation);
        }
        let app_handle = self.app.clone();
        match self.app.global_shortcut().on_shortcut(
            escape_shortcut(),
            move |_app, _shortcut, event| {
                if event.state != ShortcutState::Pressed {
                    return;
                }
                let app = app_handle.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    crate::commands::cancel_dictation(&app)
                });
            },
        ) {
            Ok(()) => {
                crate::logging::log_message("[shortcut] Registered: Escape (cancel dictation)");
                true
            }
            Err(e) => {
                crate::logging::log_message(&format!("[shortcut] Failed to register Esc: {}", e));
                false
            }
        }
    }

    fn unregister_escape(&self) -> Result<(), String> {
        self.app
            .global_shortcut()
            .unregister(escape_shortcut())
            .map_err(|e| e.to_string())
    }
}

fn escape_shortcut() -> Shortcut {
    "Escape"
        .parse::<Shortcut>()
        .expect("Escape is a valid global shortcut")
}

/// Held registration: while it exists, Esc cancels dictation. It is armed on
/// `recording` and released on every other phase, so Esc keeps its normal
/// meaning everywhere else.
pub struct EscShortcut<R: HotkeyRegistration> {
    registration: R,
    is_recording: Arc<AtomicBool>,
    held: bool,
}

impl<R: HotkeyRegistration> EscShortcut<R> {
    pub fn new(registration: R, is_recording: Arc<AtomicBool>) -> Self {
        Self {
            registration,
            is_recording,
            held: false,
        }
    }

    pub fn sync(&mut self, recording: bool) {
        if recording == self.held {
            return;
        }
        self.held = recording;
        if recording {
            if self
                .registration
                .register("Escape", self.is_recording.clone())
            {
                return;
            }
            self.held = false;
            crate::logging::log_message("[shortcut] Could not register Esc to cancel dictation");
            return;
        }
        if let Err(e) = self.registration.unregister_escape() {
            crate::logging::log_message(&format!("[shortcut] Failed to release Esc: {}", e));
        }
    }
}

impl<R: HotkeyRegistration> Drop for EscShortcut<R> {
    fn drop(&mut self) {
        if self.held {
            let _ = self.registration.unregister_escape();
        }
    }
}

/// Re-arm the dictation shortcut after macOS grants Accessibility while
/// running: the tap that failed without the grant is started now, and a plugin
/// key is registered again in place. Other actions are untouched.
pub fn rearm_dictation(shortcuts: &AppShortcuts, keys: &str) {
    if keys.is_empty()
        || !crate::shortcuts::ShortcutsPlatform::arm(shortcuts, ShortcutAction::Dictation, keys)
    {
        crate::logging::log_message("[shortcut] Re-arm after Accessibility grant failed");
    }
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
fn start_macos_super_control_listener(app: AppHandle, _is_recording: Arc<AtomicBool>) {
    stop_macos_super_control_listener();
    let my_gen = SUPER_CONTROL_GEN.load(Ordering::SeqCst);
    std::thread::spawn(move || {
        if let Err(e) = run_macos_super_control_tap(app, _is_recording, my_gen) {
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
    _is_recording: Arc<AtomicBool>,
    my_gen: u64,
) -> Result<(), String> {
    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use core_graphics::event::{
        CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        CGEventType, CallbackResult,
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
                return CallbackResult::Keep;
            }
            let flags = event.get_flags();
            let command = flags.contains(CGEventFlags::CGEventFlagCommand);
            let control = flags.contains(CGEventFlags::CGEventFlagControl);
            let both_now = command && control;
            let was_down = both_flag.load(Ordering::SeqCst);
            if both_now && !was_down {
                crate::commands::press_dictation(&app_for_callback);
            }
            both_flag.store(both_now, Ordering::SeqCst);
            CallbackResult::Keep
        },
    )
    .map_err(|_| "CGEventTapCreate failed (grant Accessibility permission)".to_string())?;

    let loop_source = tap
        .mach_port()
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

    #[test]
    fn shortcuts_are_registered_in_one_place() {
        let module = include_str!("shortcuts.rs");
        let production = module.split("#[cfg(test)]").next().expect("a module body");
        assert!(
            !production.contains("unregister_all"),
            "changing a shortcut must never wipe every registration"
        );
    }

    #[derive(Default)]
    struct Recorder {
        calls: Arc<Mutex<Vec<&'static str>>>,
        accept: Arc<AtomicBool>,
    }

    impl Clone for Recorder {
        fn clone(&self) -> Self {
            Self {
                calls: self.calls.clone(),
                accept: self.accept.clone(),
            }
        }
    }

    impl Recorder {
        fn accepting() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                accept: Arc::new(AtomicBool::new(true)),
            }
        }
    }

    impl HotkeyRegistration for Recorder {
        fn register(&self, _hotkey: &str, _is_recording: Arc<AtomicBool>) -> bool {
            self.calls.lock().unwrap().push("register:esc");
            self.accept.load(Ordering::SeqCst)
        }

        fn unregister_escape(&self) -> Result<(), String> {
            self.calls.lock().unwrap().push("unregister:esc");
            Ok(())
        }
    }

    #[test]
    fn esc_shortcut_registered_only_while_recording() {
        let recorder = Recorder::accepting();
        let mut esc = EscShortcut::new(recorder.clone(), Arc::new(AtomicBool::new(false)));

        // idle -> recording -> transcribing
        esc.sync(true);
        esc.sync(true);
        esc.sync(false);

        assert_eq!(
            *recorder.calls.lock().unwrap(),
            vec!["register:esc", "unregister:esc"],
            "Esc is armed once on recording and released on any other phase"
        );
    }

    #[test]
    fn dropping_while_recording_releases_esc() {
        let recorder = Recorder::accepting();
        {
            let mut esc = EscShortcut::new(recorder.clone(), Arc::new(AtomicBool::new(true)));
            esc.sync(true);
        }
        assert_eq!(
            recorder.calls.lock().unwrap().last().copied(),
            Some("unregister:esc")
        );
    }

    #[test]
    fn failed_esc_registration_is_not_held() {
        let refusing = Recorder::default();
        let mut esc = EscShortcut::new(refusing.clone(), Arc::new(AtomicBool::new(false)));
        esc.sync(true);
        esc.sync(false);
        assert_eq!(
            *refusing.calls.lock().unwrap(),
            vec!["register:esc"],
            "a refused registration is never released"
        );
    }
}
