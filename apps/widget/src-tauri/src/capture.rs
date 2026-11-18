use crate::logging::log_message;
use crate::scribe::ScribeStore;
use screencapturekit::shareable_content::SCShareableContent;
use std::time::{SystemTime, UNIX_EPOCH};

/// Check whether the app has macOS Screen Recording permission.
#[cfg(target_os = "macos")]
pub fn has_screen_recording_permission() -> bool {
    unsafe extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
    }
    unsafe { CGPreflightScreenCaptureAccess() }
}

#[cfg(not(target_os = "macos"))]
pub fn has_screen_recording_permission() -> bool {
    false
}

/// Try to obtain Screen Recording permission. Returns `(granted, denied)`.
///
/// macOS has no programmatic grant — the system only shows its permission
/// dialog on the *first* attempt to touch the screen. So this deliberately runs
/// a real ScreenCaptureKit query: if the state is undetermined, macOS pops the
/// prompt; if it is already denied, the query errors out and `denied` is true so
/// the UI can route the user to System Settings.
#[cfg(target_os = "macos")]
pub fn request_screen_recording_permission() -> (bool, bool) {
    if has_screen_recording_permission() {
        return (true, false);
    }
    match SCShareableContent::get() {
        Ok(_) => (true, false),
        Err(_) => (false, true),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_screen_recording_permission() -> (bool, bool) {
    (false, true)
}

/// Read a PNG's width/height straight from its IHDR chunk. Used everywhere we
/// need pixel dimensions up front (docx embedding, event metadata).
pub fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if bytes.len() >= 24 && bytes[0..8] == PNG_SIGNATURE {
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        Some((w, h))
    } else {
        None
    }
}

/// Capture a screenshot with macOS's own region/window picker.
///
/// We launch `/usr/sbin/screencapture -i`, the same native interaction that
/// the system screenshot shortcuts use: a crosshair appears, the user drags a
/// region or hits Space + clicks a window, and the image is written to the
/// chosen path. Saving into the session directory means no "find the new file"
/// step — the intent and the retrieval are the same action.
///
/// Returns `(relative_image_path, "WxH" dimensions)`. Returns an `Err` when the
/// user cancels (Esc) so the caller can treat it as a deliberate no-op.
#[cfg(target_os = "macos")]
pub fn capture_interactive_screenshot(session_id: i64) -> Result<(String, String), String> {
    let dir = ScribeStore::session_dir(session_id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let file_name = format!("screenshot-{}.png", ts);
    let abs_path = dir.join(&file_name);
    let abs_str = abs_path.to_string_lossy().to_string();

    // -x: no shutter sound; -t png: force PNG regardless of the user's
    // `com.apple.screencapture type` preference; -i: interactive selection.
    let status = std::process::Command::new("/usr/sbin/screencapture")
        .args(["-x", "-t", "png", "-i"])
        .arg(&abs_str)
        .status()
        .map_err(|e| format!("Failed to launch screencapture: {}", e))?;

    if !status.success() || !abs_path.exists() {
        let _ = std::fs::remove_file(&abs_path);
        return Err("Screenshot capture cancelled".to_string());
    }

    let dims = match std::fs::read(&abs_path)
        .ok()
        .and_then(|b| png_dimensions(&b))
    {
        Some((w, h)) => format!("{}x{}", w, h),
        None => "unknown".to_string(),
    };
    let relative = format!("scribe/{}/{}", session_id, file_name);
    log_message(&format!(
        "[scribe] Screenshot saved: {} ({})",
        abs_str, dims
    ));
    Ok((relative, dims))
}

#[cfg(not(target_os = "macos"))]
pub fn capture_interactive_screenshot(_session_id: i64) -> Result<(String, String), String> {
    Err("Screenshot capture is only supported on macOS".to_string())
}
