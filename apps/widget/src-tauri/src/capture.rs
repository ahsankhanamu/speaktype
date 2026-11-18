use crate::logging::log_message;
use std::time::{SystemTime, UNIX_EPOCH};

/// Read a PNG's width/height straight from its IHDR chunk. Used everywhere we
/// need pixel dimensions up front (capture metadata).
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
/// chosen path. Saving into the session's capture directory means no "find the
/// new file" step — the intent and the retrieval are the same action.
///
/// Returns `(absolute_image_path, "WxH" dimensions)`. Returns an `Err` when the
/// user cancels (Esc) so the caller can treat it as a deliberate no-op.
#[cfg(target_os = "macos")]
pub fn capture_interactive_screenshot(session_id: i64) -> Result<(String, String), String> {
    let dir = crate::session_cmds::session_capture_dir(session_id);
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
    log_message(&format!(
        "[capture] Screenshot saved: {} ({})",
        abs_str, dims
    ));
    Ok((abs_str, dims))
}

#[cfg(not(target_os = "macos"))]
pub fn capture_interactive_screenshot(_session_id: i64) -> Result<(String, String), String> {
    Err("Screenshot capture is only supported on macOS".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid 1x1 transparent PNG (IHDR width/height = 1).
    const ONE_PX_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x04, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0x78,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn png_dimensions_reads_ihdr() {
        assert_eq!(png_dimensions(ONE_PX_PNG), Some((1, 1)));
        assert_eq!(png_dimensions(b"not a png"), None);
    }
}
