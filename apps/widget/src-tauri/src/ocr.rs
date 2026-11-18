use crate::logging::log_message;

/// Run Apple Vision OCR on an image file. Returns the recognized text joined by
/// newlines (Vision's natural top-down, left-to-right order).
///
/// Returns `Ok(None)` when the file doesn't exist — the caller treats it as a
/// non-fatal "no OCR" case.
#[cfg(target_os = "macos")]
pub fn ocr_image(path: &str) -> Result<Option<String>, String> {
    use apple_vision::recognize_text::{RecognitionLevel, TextRecognizer};

    if !std::path::Path::new(path).exists() {
        return Ok(None);
    }

    let recognizer = TextRecognizer::new()
        .with_recognition_level(RecognitionLevel::Accurate)
        .with_language_correction(true);

    let observations = recognizer
        .recognize_in_path(path)
        .map_err(|e| format!("OCR failed: {}", e))?;

    if observations.is_empty() {
        return Ok(None);
    }

    let text = observations
        .iter()
        .map(|obs| obs.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    log_message(&format!(
        "[scribe] OCR extracted {} chars from {:?}",
        text.len(),
        path
    ));
    Ok(Some(text))
}

#[cfg(not(target_os = "macos"))]
pub fn ocr_image(path: &str) -> Result<Option<String>, String> {
    let _ = path;
    Ok(None)
}
