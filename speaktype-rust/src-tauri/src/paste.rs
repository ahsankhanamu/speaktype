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

/// Get the currently active window
pub fn get_active_window() -> Option<WindowInfo> {
    #[cfg(target_os = "macos")]
    {
        macos_get_active_window()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn macos_get_active_window() -> Option<WindowInfo> {
    // Use osascript to get the frontmost app that isn't us.
    // NSWorkspace.frontmostApplication returns our own app when the widget is clicked,
    // so we need to find the app that was active before we took focus.
    let script = r#"
        tell application "System Events"
            set appList to every process whose frontmost is true
            if (count of appList) > 0 then
                set frontApp to item 1 of appList
                set appName to name of frontApp
                set appID to bundle identifier of frontApp
                if appID is "com.speaktype.widget" then
                    -- We are frontmost, find the next visible app
                    set allApps to every process whose visible is true and bundle identifier is not "com.speaktype.widget"
                    if (count of allApps) > 0 then
                        set targetApp to item 1 of allApps
                        return (name of targetApp) & "|" & (bundle identifier of targetApp)
                    end if
                end if
                return appName & "|" & appID
            end if
        end tell
        return ""
    "#;

    match std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
    {
        Ok(output) => {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if result.is_empty() {
                log_message("[window] No active window found");
                return None;
            }
            let parts: Vec<&str> = result.splitn(2, '|').collect();
            let name = parts.first().unwrap_or(&"").to_string();
            let bundle_id = parts.get(1).map(|s| s.to_string());

            log_message(&format!(
                "[window] Active: name={:?}, bundle_id={:?}",
                name, bundle_id
            ));

            Some(WindowInfo { name, bundle_id })
        }
        Err(e) => {
            log_message(&format!("[window] osascript failed: {}", e));
            None
        }
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
