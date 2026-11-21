use crate::focus_probe::{self, Focus};
use crate::logging::log_message;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub name: String,
    #[cfg(target_os = "macos")]
    pub bundle_id: Option<String>,
    pub pid: Option<u32>,
}

const TERMINAL_NAMES: &[&str] = &[
    "terminal",
    "iterm",
    "hyper",
    "kitty",
    "alacritty",
    "wezterm",
];

#[cfg(target_os = "macos")]
const TERMINAL_BUNDLE_IDS: &[&str] = &[
    "com.apple.terminal",
    "com.googlecode.iterm2",
    "co.zeit.hyper",
    "net.kovidgoyal.kitty",
    "org.alacritty",
    "com.github.wez.wezterm",
];

/// Get the currently active (frontmost) window, skipping SpeakType itself.
/// Returns the frontmost non-SpeakType app.
///
/// Uses `NSWorkspace` directly instead of spawning `osascript` so the window
/// tracker in the background thread costs ~nothing and never touches the
/// Accessibility server. `pid` is always filled on macOS and is the most
/// reliable identity for re-activation later — it survives stale
/// LaunchServices registrations (e.g. an .app that was moved) that make
/// `tell application id …` fail with `-43`.
pub fn get_frontmost_window() -> Option<WindowInfo> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWorkspace;
        let ws = NSWorkspace::sharedWorkspace();
        let front = ws.frontmostApplication()?;
        let pid = front.processIdentifier();
        if pid == std::process::id() as i32 {
            return None;
        }
        let name = front
            .localizedName()
            .map(|s| s.to_string())
            .unwrap_or_default();
        #[cfg(target_os = "macos")]
        let bundle_id = front.bundleIdentifier().map(|s| s.to_string());
        Some(WindowInfo {
            name,
            bundle_id,
            pid: Some(pid as u32),
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Run one `osascript` invocation and report whether it exited successfully.
fn run_osascript(script: &str) -> bool {
    match std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Focus a previously captured window and wait until it is actually frontmost.
///
/// Native `NSRunningApplication` activation is preferred: it needs neither the
/// Automation (Apple Events) grant nor a syscall round-trip through `osascript`,
/// and the poll makes the following synthetic keystroke deterministic instead
/// of racing a fixed 50 ms sleep.
pub fn focus_window(window: &WindowInfo) -> bool {
    if let Some(pid) = window.pid {
        if activate_by_pid(pid) {
            log_message(&format!(
                "[window] Focus {:?}: ok (by pid {})",
                window.name, pid
            ));
            return true;
        }
    }

    if focus_by_name(window) {
        return true;
    }

    log_message(&format!(
        "[window] Focus {:?} failed: could not activate",
        window.name
    ));
    false
}

#[cfg(target_os = "macos")]
fn activate_by_pid(pid: u32) -> bool {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};

    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid as i32)
    else {
        return false;
    };
    if !app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows) {
        return false;
    }

    let deadline = Instant::now() + Duration::from_millis(600);
    loop {
        let is_front = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|front| front.processIdentifier() == pid as i32)
            .unwrap_or(false);
        if is_front {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(not(target_os = "macos"))]
fn activate_by_pid(_pid: u32) -> bool {
    false
}

#[cfg(target_os = "macos")]
fn focus_by_name(window: &WindowInfo) -> bool {
    // Bundle-id activation via LaunchServices. Kept as a fallback for processes
    // NSRunningApplication cannot resolve (e.g. a moved .app).
    if let Some(id) = window
        .bundle_id
        .as_deref()
        .filter(|s| !s.is_empty() && s.contains('.'))
    {
        let script = format!("tell application id \"{}\" to activate", id);
        if run_osascript(&script) {
            std::thread::sleep(Duration::from_millis(150));
            return true;
        }
    }

    if !window.name.is_empty() {
        let script = format!(
            "tell application \"System Events\" to set frontmost of \
             application process \"{}\" to true",
            window.name
        );
        if run_osascript(&script) {
            std::thread::sleep(Duration::from_millis(150));
            return true;
        }
    }

    false
}

#[cfg(not(target_os = "macos"))]
fn focus_by_name(_window: &WindowInfo) -> bool {
    false
}

/// Check if the window is a terminal application
pub fn is_terminal(window: &WindowInfo) -> bool {
    let name_lower = window.name.to_lowercase();
    if TERMINAL_NAMES.iter().any(|t| name_lower.contains(t)) {
        return true;
    }

    #[cfg(target_os = "macos")]
    if let Some(ref bid) = window.bundle_id {
        let bid_lower = bid.to_lowercase();
        if TERMINAL_BUNDLE_IDS.iter().any(|t| bid_lower.contains(t)) {
            return true;
        }
    }

    false
}

/// Send ⌘V as a synthetic keyboard event. Every Mac app pastes the clipboard
/// with ⌘V, terminals included; ⌘⇧V in Terminal is Paste Selection, which
/// pastes whatever text is selected in the window instead.
///
/// `CGEventPost` requires only the Accessibility grant. The previous
/// `osascript` approach also required the user to allow SpeakType to control
/// System Events (Automation), so a granted Accessibility toggle alone was not
/// enough for pasting to work.
#[cfg(target_os = "macos")]
fn simulate_paste() -> bool {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEY_V: u16 = 0x09;

    let source = match CGEventSource::new(CGEventSourceStateID::CombinedSessionState) {
        Ok(source) => source,
        Err(_) => {
            log_message("[paste] Failed to create CGEventSource");
            return false;
        }
    };

    let flags = CGEventFlags::CGEventFlagCommand;

    let (Ok(down), Ok(up)) = (
        CGEvent::new_keyboard_event(source.clone(), KEY_V, true),
        CGEvent::new_keyboard_event(source, KEY_V, false),
    ) else {
        log_message("[paste] Failed to create CGEvent for the paste keystroke");
        return false;
    };

    down.set_flags(flags);
    down.post(CGEventTapLocation::HID);
    std::thread::sleep(Duration::from_millis(12));
    up.set_flags(flags);
    up.post(CGEventTapLocation::HID);
    true
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn simulate_keypress(key_name: &str) -> bool {
    use core_graphics::event::{CGEvent, CGEventTapLocation, KeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = match CGEventSource::new(CGEventSourceStateID::CombinedSessionState) {
        Ok(source) => source,
        Err(_) => {
            log_message("[post-paste] Failed to create CGEventSource");
            return false;
        }
    };

    if key_name.chars().count() == 1 {
        let Ok(down) = CGEvent::new_keyboard_event(source.clone(), 0, true) else {
            return false;
        };
        down.set_string(key_name);
        down.post(CGEventTapLocation::HID);
        if let Ok(up) = CGEvent::new_keyboard_event(source, 0, false) {
            up.post(CGEventTapLocation::HID);
        }
        return true;
    }

    let keycode = match key_name {
        "enter" | "return" => KeyCode::RETURN,
        "tab" => KeyCode::TAB,
        "space" => KeyCode::SPACE,
        "escape" | "esc" => KeyCode::ESCAPE,
        "backspace" => KeyCode::DELETE,
        "delete" => KeyCode::FORWARD_DELETE,
        "up" => KeyCode::UP_ARROW,
        "down" => KeyCode::DOWN_ARROW,
        "left" => KeyCode::LEFT_ARROW,
        "right" => KeyCode::RIGHT_ARROW,
        other => {
            log_message(&format!("[post-paste] Unsupported key '{}'", other));
            return false;
        }
    };

    let (Ok(down), Ok(up)) = (
        CGEvent::new_keyboard_event(source.clone(), keycode, true),
        CGEvent::new_keyboard_event(source, keycode, false),
    ) else {
        return false;
    };
    down.post(CGEventTapLocation::HID);
    std::thread::sleep(Duration::from_millis(12));
    up.post(CGEventTapLocation::HID);
    true
}

#[cfg(not(target_os = "macos"))]
fn simulate_keypress(_key_name: &str) -> bool {
    false
}

/// The text the permissions page's Test typing types. One line, ASCII, so it
/// lands the same in every app.
pub const TEST_TYPING_TEXT: &str = "SpeakType test";

/// Type `text` into the focused element as Unicode keystrokes (`CGEvent`
/// string events are Unicode-correct, so no key code table is needed).
#[cfg(target_os = "macos")]
pub fn type_test_text(text: &str) -> Result<(), String> {
    use core_graphics::event::CGEventTapLocation;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| "Couldn't reach the keyboard.".to_string())?;
    for ch in text.chars() {
        let down = core_graphics::event::CGEvent::new_keyboard_event(source.clone(), 0, true)
            .map_err(|_| "Couldn't type.".to_string())?;
        down.set_string(String::from(ch).as_str());
        down.post(CGEventTapLocation::HID);
        let up = core_graphics::event::CGEvent::new_keyboard_event(source.clone(), 0, false)
            .map_err(|_| "Couldn't type.".to_string())?;
        up.post(CGEventTapLocation::HID);
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn type_test_text(_text: &str) -> Result<(), String> {
    Err("Test typing is available on macOS only.".to_string())
}

/// Press configured keys after pasting, if any.
fn press_post_paste_keys(post_paste_keys: Option<&str>) {
    let keys_str = match post_paste_keys {
        Some(k) if !k.is_empty() => k,
        _ => return,
    };

    std::thread::sleep(Duration::from_millis(100));

    for key_name in keys_str.split('+') {
        let key = key_name.trim().to_lowercase();
        if !simulate_keypress(&key) {
            log_message(&format!("[post-paste] Could not send key '{}'", key));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Paste text into the target window and report whether the keystroke was
/// delivered. On failure the transcript is deliberately left on the clipboard
/// so it can still be pasted manually.
/// How a paste ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Delivery {
    /// ⌘V went to an editable field (confirmed when the app reports text length).
    Pasted,
    /// Nothing to paste into, or the text didn't land: it stays on the clipboard.
    Copied,
    /// The paste couldn't be attempted (clipboard, permission, or focus).
    Failed,
}

/// How long to wait for a pasted field to report its new text.
const LAND_TIMEOUT: Duration = Duration::from_millis(500);

pub fn paste_text(
    text: &str,
    target_window: Option<&WindowInfo>,
    post_paste_keys: Option<&str>,
) -> Delivery {
    let old_clipboard = arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get_text().ok());

    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            if let Err(e) = clipboard.set_text(text) {
                log_message(&format!("[paste] Failed to set clipboard: {}", e));
                return Delivery::Failed;
            }
        }
        Err(e) => {
            log_message(&format!("[paste] Failed to open clipboard: {}", e));
            return Delivery::Failed;
        }
    }

    if !crate::permissions::check_accessibility(false) {
        log_message(
            "[paste] Accessibility permission missing — text left on clipboard; enable SpeakType \
             under Privacy & Security → Accessibility",
        );
        return Delivery::Failed;
    }

    let Some(window) = target_window else {
        log_message("[paste] No target window — text kept on clipboard");
        return Delivery::Failed;
    };
    if !focus_window(window) {
        log_message("[paste] Could not focus window — text kept on clipboard");
        return Delivery::Failed;
    }

    // A non-activating panel can hand off focus a beat after activation, so
    // give the target one more moment before posting the keystroke.
    std::thread::sleep(Duration::from_millis(80));

    // Terminals often draw their own text and hide it from Accessibility, so
    // they always get the paste.
    let terminal = is_terminal(window);
    let before = match window.pid {
        Some(pid) if !terminal => focus_probe::probe(pid),
        _ => Focus::Unknown,
    };
    if before == Focus::NotEditable {
        log_message(&format!(
            "[paste] Nothing editable has focus in {} — text kept on clipboard",
            window.name
        ));
        return Delivery::Copied;
    }

    if !simulate_paste() {
        log_message("[paste] Failed to send paste keystroke — text kept on clipboard");
        return Delivery::Failed;
    }

    // Wait for the field to report the pasted text, where it reports length.
    let mut landed = None;
    if let (Focus::Editable { chars: Some(_) }, Some(pid)) = (before, window.pid) {
        let deadline = Instant::now() + LAND_TIMEOUT;
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
            landed = focus_probe::landed(before, focus_probe::probe(pid));
            if landed == Some(true) {
                break;
            }
        }
    }
    if landed == Some(false) {
        log_message(&format!(
            "[paste] The text didn't land in {} — kept on clipboard",
            window.name
        ));
        return Delivery::Copied;
    }

    press_post_paste_keys(post_paste_keys);

    // Only a paste we saw land gives the clipboard back; otherwise the words
    // stay on it, so ⌘V still recovers them.
    if landed == Some(true) {
        if let Some(old) = old_clipboard {
            std::thread::sleep(Duration::from_millis(600));
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(old);
            }
        }
    }

    Delivery::Pasted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typing_text_is_one_plain_line() {
        assert_eq!(TEST_TYPING_TEXT, "SpeakType test");
        assert!(TEST_TYPING_TEXT.is_ascii());
        assert!(!TEST_TYPING_TEXT.contains('\n'));
    }
}
