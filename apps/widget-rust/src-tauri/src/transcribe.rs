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

const HALLUCINATION_FRAGMENTS: &[&str] = &[
    "thank you",
    "thanks for",
    "i'm alone",
    "im alone",
    "subscribe",
    "see you next",
    "bye bye",
];

fn looks_like_sound_effect_markup(text: &str) -> bool {
    text.contains('*') && text.chars().filter(|c| *c == '*').count() >= 2
}

fn has_repetitive_clauses(text: &str) -> bool {
    let normalized = text.to_lowercase();
    let clauses: Vec<&str> = normalized
        .split(|c| matches!(c, '.' | '!' | '?'))
        .map(str::trim)
        .filter(|s| s.len() > 5)
        .collect();

    if clauses.len() >= 2 {
        let mut seen = std::collections::HashSet::new();
        for clause in clauses {
            if !seen.insert(clause) {
                return true;
            }
        }
    }

    let words: Vec<&str> = normalized.split_whitespace().collect();
    if words.len() >= 6 && words.len() % 2 == 0 {
        let half = words.len() / 2;
        if words[..half] == words[half..] {
            return true;
        }
    }

    false
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

    if looks_like_sound_effect_markup(t) {
        return true;
    }

    if has_repetitive_clauses(t) {
        return true;
    }

    if t.len() < 40 {
        if HALLUCINATION_PHRASES.iter().any(|phrase| t.contains(phrase)) {
            return true;
        }
    }

    if t.len() < 120 {
        if HALLUCINATION_FRAGMENTS.iter().any(|phrase| t.contains(phrase)) {
            return true;
        }
    }

    false
}

pub async fn transcribe(
    wav_data: Vec<u8>,
    settings: &Settings,
    duration_secs: f64,
) -> Result<String, String> {
    transcribe_with_prompt(wav_data, settings, duration_secs, None).await
}

pub async fn transcribe_with_prompt(
    wav_data: Vec<u8>,
    settings: &Settings,
    duration_secs: f64,
    prompt: Option<&str>,
) -> Result<String, String> {
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
        if let Some(prompt) = prompt.filter(|p| !p.trim().is_empty()) {
            form = form.text("prompt", prompt.to_string());
            form = form.text("initial_prompt", prompt.to_string());
        }
    } else {
        form = form
            .text("language", settings.language.clone())
            .text("temperature", "0.0")
            .text("no_speech_thold", "0.65")
            .text("entropy_thold", "2.4")
            .text("suppress_nst", "true");
        if let Some(prompt) = prompt.filter(|p| !p.trim().is_empty()) {
            form = form.text("prompt", prompt.to_string());
            form = form.text("initial_prompt", prompt.to_string());
        }
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

#[cfg(test)]
mod tests {
    use super::is_hallucination;

    #[test]
    fn rejects_silence_hallucination_with_sound_effects() {
        let text = "*shriek* *shriek* ... ... I'm alone. I'm alone. I'm alone. Thank you. Thank you.";
        assert!(is_hallucination(text));
    }

    #[test]
    fn rejects_common_youtube_outro() {
        assert!(is_hallucination("Thanks for watching!"));
    }

    #[test]
    fn accepts_real_short_phrase() {
        assert!(!is_hallucination("Send the report by Friday."));
    }
}
