use crate::logging::log_message;
use crate::settings::Settings;

/// Common Whisper hallucinations on silence/noise
const HALLUCINATION_PHRASES: &[&str] = &[
    "thanks for watching",
    "thank you for watching",
    "thanks for listening",
    "thank you for listening",
    "subscribe",
    "like and subscribe",
    "see you next time",
    "the end",
    "silence",
    "no speech",
    "inaudible",
    "[music]",
    "(music)",
];

const HALLUCINATION_WORDS: &[&str] = &[
    "you", "i", "so", "uh", "um", "hmm", "huh", "ah", "oh", "bye", "goodbye",
];

/// Check if URL looks like an OpenAI-compatible API
fn is_openai_api(url: &str) -> bool {
    let lower = url.to_lowercase();
    let patterns = [
        "/v1/audio/transcriptions",
        "/v1/audio/",
        "openai",
        "groq",
        "deepgram",
    ];
    patterns.iter().any(|p| lower.contains(p))
}

/// Check if text is likely a Whisper hallucination
pub fn is_hallucination(text: &str) -> bool {
    let t = text.to_lowercase();
    let t = t.trim();

    if t.len() < 3 {
        return true;
    }

    // Check if entire text is just a hallucination word
    if HALLUCINATION_WORDS.iter().any(|w| *w == t) {
        return true;
    }

    // Check for hallucination phrases in short outputs
    if t.len() < 40 {
        return HALLUCINATION_PHRASES.iter().any(|phrase| t.contains(phrase));
    }

    false
}

/// Transcribe WAV audio data via the configured API
pub async fn transcribe(wav_data: Vec<u8>, settings: &Settings) -> Result<String, String> {
    let api_url = &settings.api_url;
    let is_openai = is_openai_api(api_url);

    log_message(&format!(
        "[transcribe] Sending {}KB to {} (openai_compat={})",
        wav_data.len() / 1024,
        api_url,
        is_openai
    ));

    let file_part = reqwest::multipart::Part::bytes(wav_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Failed to create file part: {}", e))?;

    let mut form = reqwest::multipart::Form::new().part("file", file_part);

    if is_openai {
        form = form
            .text("model", settings.model.clone())
            .text("response_format", "json".to_string());
        if settings.language != "auto" {
            form = form.text("language", settings.language.clone());
        }
    } else {
        if settings.language != "auto" {
            form = form.text("language", settings.language.clone());
        }
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(api_url)
        .multipart(form)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API error: HTTP {} - {}", status, body));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Try JSON {"text": "..."} first, fall back to plain text
    let text = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        json.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        body.trim().to_string()
    };

    log_message(&format!("[transcribe] Result: {:?}", text));
    Ok(text)
}
