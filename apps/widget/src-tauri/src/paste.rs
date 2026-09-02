use crate::logging::log_message;

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
        let name = front.localizedName().map(|s| s.to_string()).unwrap_or_default();
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

/// Focus a previously captured window. Tries, in order:
///   1. `tell application id "<bundle_id>" to activate` (no Accessibility needed)
///   2. System Events `set frontmost …` for the captured PID (needs Accessibility,
///      but works even when LaunchServices can't resolve the app)
///   3. System Events by process name (last resort)
pub fn focus_window(window: &WindowInfo) -> bool {
    // 1) Bundle id activation (works without Accessibility permission).
    if let Some(id) = window
        .bundle_id
        .as_deref()
        .filter(|s| !s.is_empty() && s.contains('.'))
    {
        let script = format!("tell application id \"{}\" to activate", id);
        if run_osascript(&script) {
            log_message(&format!("[window] Focus {:?}: ok (by bundle id)", window.name));
            std::thread::sleep(std::time::Duration::from_millis(50));
            return true;
        }
    }

    // 2) PID-based activation via the Accessibility API.
    if let Some(pid) = window.pid {
        let script = format!(
            "tell application \"System Events\" to set frontmost of \
             (first application process whose unix id is {}) to true",
            pid
        );
        if run_osascript(&script) {
            log_message(&format!("[window] Focus {:?}: ok (by pid {})", window.name, pid));
            std::thread::sleep(std::time::Duration::from_millis(50));
            return true;
        }
    } else if !window.name.is_empty() {
        // 3) No pid captured — fall back to process name.
        let script = format!(
            "tell application \"System Events\" to set frontmost of \
             application process \"{}\" to true",
            window.name
        );
        if run_osascript(&script) {
            log_message(&format!("[window] Focus {:?}: ok (by process name)", window.name));
            std::thread::sleep(std::time::Duration::from_millis(50));
            return true;
        }
    }

    log_message(&format!("[window] Focus {:?} failed: could not activate", window.name));
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

/// Simulate Cmd+V (or Cmd+Shift+V for terminals) via osascript.
/// This avoids enigo's requirement to run on the main thread on macOS.
fn simulate_paste(use_shift: bool) {
    let script = if use_shift {
        r#"tell application "System Events" to keystroke "v" using {command down, shift down}"#
    } else {
        r#"tell application "System Events" to keystroke "v" using command down"#
    };

    match std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log_message(&format!("[paste] osascript failed: {}", stderr));
            }
        }
        Err(e) => {
            log_message(&format!("[paste] Failed to run osascript: {}", e));
        }
    }
}

/// Simulate a single key press via osascript
fn simulate_keypress(key_name: &str) {
    let script = match key_name {
        "enter" | "return" => {
            r#"tell application "System Events" to key code 36"#.to_string()
        }
        "tab" => {
            r#"tell application "System Events" to key code 48"#.to_string()
        }
        "space" => {
            r#"tell application "System Events" to key code 49"#.to_string()
        }
        "escape" | "esc" => {
            r#"tell application "System Events" to key code 53"#.to_string()
        }
        "backspace" => {
            r#"tell application "System Events" to key code 51"#.to_string()
        }
        "delete" => {
            r#"tell application "System Events" to key code 117"#.to_string()
        }
        "up" => {
            r#"tell application "System Events" to key code 126"#.to_string()
        }
        "down" => {
            r#"tell application "System Events" to key code 125"#.to_string()
        }
        "left" => {
            r#"tell application "System Events" to key code 123"#.to_string()
        }
        "right" => {
            r#"tell application "System Events" to key code 124"#.to_string()
        }
        other => {
            // For single characters, use keystroke
            format!(
                r#"tell application "System Events" to keystroke "{}""#,
                other
            )
        }
    };

    match std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log_message(&format!("[post-paste] keypress '{}' failed: {}", key_name, stderr.trim()));
            }
        }
        Err(e) => {
            log_message(&format!("[post-paste] osascript failed for '{}': {}", key_name, e));
        }
    }
}

/// Press configured keys after pasting, if any
pub fn press_post_paste_keys(post_paste_keys: Option<&str>) {
    let keys_str = match post_paste_keys {
        Some(k) if !k.is_empty() => k,
        _ => return,
    };

    std::thread::sleep(std::time::Duration::from_millis(100));

    for key_name in keys_str.split('+') {
        let key = key_name.trim().to_lowercase();
        simulate_keypress(&key);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Paste text into the target window
pub fn paste_text(text: &str, target_window: Option<&WindowInfo>) {
    // Save old clipboard
    let old_clipboard = arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get_text().ok());

    // Set new clipboard
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Err(e) = clipboard.set_text(text) {
            log_message(&format!("[paste] Failed to set clipboard: {}", e));
            return;
        }
    } else {
        log_message("[paste] Failed to open clipboard");
        return;
    }

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Focus target window
    if let Some(window) = target_window {
        if !focus_window(window) {
            log_message("[paste] Could not focus window — text kept on clipboard");
            return;
        }
    } else {
        log_message("[paste] No target window — text kept on clipboard");
        return;
    }

    // Simulate paste keystroke via osascript (must use main thread-safe API on macOS)
    let use_shift = target_window.map_or(false, |w| is_terminal(w));
    simulate_paste(use_shift);

    // Restore old clipboard after 500ms
    if let Some(old) = old_clipboard {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(old);
            }
        });
    }
}
