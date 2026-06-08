use crate::logging::log_message;
use crate::settings::Settings;

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

pub fn is_hallucination(text: &str) -> bool {
    let t = text.to_lowercase();
    let t = t.trim();

    if t.len() < 3 {
        return true;
    }

    if HALLUCINATION_WORDS.iter().any(|w| *w == t) {
        return true;
    }

    if t.len() < 40 {
        return HALLUCINATION_PHRASES.iter().any(|phrase| t.contains(phrase));
    }

    false
}

pub async fn transcribe(wav_data: Vec<u8>, settings: &Settings, duration_secs: f64) -> Result<String, String> {
    let api_url = &settings.api_url;
    let is_openai = is_openai_api(api_url);

    let timeout_secs = (duration_secs * 2.0).max(30.0).min(300.0) as u64;

    log_message(&format!(
        "[transcribe] Sending {}KB to {} (duration={:.1}s, timeout={}s, openai_compat={})",
        wav_data.len() / 1024,
        api_url,
        duration_secs,
        timeout_secs,
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
        form = form.text("language", settings.language.clone());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let resp = client
        .post(api_url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("API request failed after {}s timeout: {}", timeout_secs, e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_else(|e| format!("<failed to read response body: {}>", e));
        return Err(format!("API error: HTTP {} - {}", status, body));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let text = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        json.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        body.trim().to_string()
    };

    let text = normalize_line_breaks(&text);
    log_message(&format!("[transcribe] Result: {} chars", text.len()));
    Ok(text)
}

/// whisper.cpp inserts line breaks at segment boundaries (~30s chunks).
/// Join single newlines into flowing text while preserving paragraph breaks (double newlines).
fn normalize_line_breaks(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let joined: Vec<String> = paragraphs
        .iter()
        .map(|p| {
            p.split('\n')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
                .join(" ")
        })
        .filter(|s| !s.is_empty())
        .collect();
    joined.join("\n\n")
}
