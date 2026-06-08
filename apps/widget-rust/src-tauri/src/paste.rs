use crate::logging::log_message;

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub name: String,
    #[cfg(target_os = "macos")]
    pub bundle_id: Option<String>,
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
pub fn get_frontmost_window() -> Option<WindowInfo> {
    #[cfg(target_os = "macos")]
    {
        macos_get_frontmost_window()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn macos_get_frontmost_window() -> Option<WindowInfo> {
    // Get frontmost app with its PID. If it's our own process, return None (caller keeps previous value).
    let script = r#"
        tell application "System Events"
            set frontApp to first application process whose frontmost is true
            set appName to name of frontApp
            set appID to bundle identifier of frontApp
            set appPID to unix id of frontApp
            return appName & "|" & appID & "|" & appPID
        end tell
    "#;

    match std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
    {
        Ok(output) => {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if result.is_empty() {
                return None;
            }
            let parts: Vec<&str> = result.splitn(3, '|').collect();
            let name = parts.first().unwrap_or(&"").to_string();
            let bundle_id = parts.get(1).map(|s| s.to_string());
            let pid = parts.get(2).and_then(|s| s.trim().parse::<u32>().ok());

            // Skip if it's our own process (works for any bundle ID)
            if pid == Some(std::process::id()) {
                return None;
            }

            Some(WindowInfo { name, bundle_id })
        }
        Err(_) => None,
    }
}

/// Focus a previously captured window using osascript (reliable, no accessibility needed for activation)
pub fn focus_window(window: &WindowInfo) -> bool {
    let identifier = window.bundle_id.as_deref().unwrap_or(&window.name);
    let script = format!(
        r#"tell application id "{}" to activate"#,
        identifier
    );

    match std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                log_message(&format!("[window] Focus {:?}: ok", window.name));
                std::thread::sleep(std::time::Duration::from_millis(50));
                true
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log_message(&format!("[window] Focus {:?} failed: {}", window.name, stderr.trim()));
                false
            }
        }
        Err(e) => {
            log_message(&format!("[window] osascript failed: {}", e));
            false
        }
    }
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
