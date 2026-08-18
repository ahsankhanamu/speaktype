use crate::debug;
use crate::logging::log_message;
use crate::quality::{self, Report, Segment, Thresholds};
use crate::settings::Settings;

/// One decode configuration. The first pass runs greedy and deterministic with
/// whisper's internal temperature fallback disabled, so any invented text comes
/// from a path we control rather than a silent server-side retry.
#[derive(Debug, Clone, Copy)]
struct Attempt {
    temperature: f32,
    use_prompt: bool,
    allow_internal_fallback: bool,
}

const FIRST_PASS: Attempt = Attempt {
    temperature: 0.0,
    use_prompt: true,
    allow_internal_fallback: false,
};

/// Tried in order when the previous pass looks hallucinated. Dropping the
/// prompt comes first because carried-over context is what seeds most
/// repetition loops; raising the temperature then breaks a degenerate greedy
/// path that would otherwise repeat verbatim.
const RETRY_LADDER: &[Attempt] = &[
    Attempt {
        temperature: 0.0,
        use_prompt: false,
        allow_internal_fallback: false,
    },
    Attempt {
        temperature: 0.4,
        use_prompt: false,
        allow_internal_fallback: true,
    },
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

pub async fn transcribe(
    wav_data: Vec<u8>,
    settings: &Settings,
    duration_secs: f64,
) -> Result<String, String> {
    transcribe_with_prompt(wav_data, settings, duration_secs, None).await
}

/// Transcribe and return only text that survived hallucination scoring.
/// Empty means everything was rejected.
pub async fn transcribe_with_prompt(
    wav_data: Vec<u8>,
    settings: &Settings,
    duration_secs: f64,
    prompt: Option<&str>,
) -> Result<String, String> {
    let report = transcribe_verified(wav_data, settings, duration_secs, prompt, None).await?;
    Ok(report.clean_text)
}

/// Transcribe, score the result, and re-decode with different parameters while
/// it still looks hallucinated. Returns the best-scoring candidate rather than
/// discarding suspicious output outright.
pub async fn transcribe_verified(
    wav_data: Vec<u8>,
    settings: &Settings,
    duration_secs: f64,
    prompt: Option<&str>,
    debug_chunk: Option<u32>,
) -> Result<Report, String> {
    if !settings.hallucination_guard {
        let raw = transcribe_once(wav_data, settings, duration_secs, prompt, FIRST_PASS).await?;
        debug::record_pass(crate::debug::PassEvent {
            chunk_idx: debug_chunk,
            pass: 1,
            temperature: FIRST_PASS.temperature,
            prompt_used: prompt.is_some(),
            raw_chars: raw.text.len(),
            score: 0.0,
            dropped: 0,
            had_metrics: false,
            reason: "guard disabled".into(),
            segments: Vec::new(),
        });
        return Ok(Report::passthrough(&raw.text));
    }

    let thresholds = Thresholds::from_settings(settings);
    let retries = (settings.hallucination_retries as usize).min(RETRY_LADDER.len());
    let attempts: Vec<Attempt> = std::iter::once(FIRST_PASS)
        .chain(RETRY_LADDER.iter().take(retries).copied())
        .collect();

    let mut best: Option<Report> = None;

    for (pass, attempt) in attempts.iter().enumerate() {
        let attempt_prompt = if attempt.use_prompt { prompt } else { None };
        let raw = match transcribe_once(
            wav_data.clone(),
            settings,
            duration_secs,
            attempt_prompt,
            *attempt,
        )
        .await
        {
            Ok(raw) => raw,
            Err(e) if best.is_some() => {
                log_message(&format!("[quality] Retry pass {} failed: {}", pass + 1, e));
                break;
            }
            Err(e) => return Err(e),
        };

        let silent = raw.text.trim().is_empty();
        let report = quality::evaluate(&raw.text, &raw.segments, &thresholds);
        log_message(&format!(
            "[quality] Pass {} (temp={:.1}, prompt={}, metrics={}): score={:.1}, {}/{} segments dropped [{}]",
            pass + 1,
            attempt.temperature,
            attempt.use_prompt && prompt.is_some(),
            report.had_metrics,
            report.score,
            report.dropped,
            report.segments.len(),
            report.reason()
        ));
        debug::record_pass(crate::debug::PassEvent {
            chunk_idx: debug_chunk,
            pass: pass as u32 + 1,
            temperature: attempt.temperature,
            prompt_used: attempt.use_prompt && prompt.is_some(),
            raw_chars: raw.text.len(),
            score: report.score,
            dropped: report.dropped,
            had_metrics: report.had_metrics,
            reason: report.reason(),
            segments: report
                .segments
                .iter()
                .enumerate()
                .map(|(i, seg)| {
                    let src = raw.segments.get(i);
                    crate::debug::SegmentDebug {
                        text: seg.text.clone(),
                        start: src.map(|s| s.start).unwrap_or(0.0),
                        end: src.map(|s| s.end).unwrap_or(0.0),
                        no_speech_prob: src.and_then(|s| s.no_speech_prob),
                        avg_logprob: seg.avg_logprob.or_else(|| src.and_then(|s| s.avg_logprob)),
                        flags: seg.flags.iter().map(|f| f.label().to_string()).collect(),
                        bad: seg.bad,
                    }
                })
                .collect(),
        });

        let clean = report.is_clean();
        if best.as_ref().map_or(true, |b| report.score > b.score) {
            best = Some(report);
        }
        if clean {
            break;
        }
        // Nothing was decoded, so there is nothing to salvage. Re-decoding
        // silence at a higher temperature invites a hallucination instead of
        // fixing one.
        if silent {
            log_message("[quality] No text decoded — not re-decoding");
            break;
        }
        if pass + 1 < attempts.len() {
            log_message("[quality] Result looks hallucinated — re-decoding");
        }
    }

    let report = best.ok_or_else(|| "Transcription produced no result".to_string())?;
    if report.dropped > 0 {
        log_message(&format!(
            "[quality] Kept {} of {} chars, dropped {} segment(s) [{}]: {:?}",
            report.clean_text.len(),
            report.raw_text.len(),
            report.dropped,
            report.reason(),
            report.dropped_text()
        ));
    }
    Ok(report)
}

struct RawResult {
    text: String,
    segments: Vec<Segment>,
}

async fn transcribe_once(
    wav_data: Vec<u8>,
    settings: &Settings,
    duration_secs: f64,
    prompt: Option<&str>,
    attempt: Attempt,
) -> Result<RawResult, String> {
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

    // verbose_json carries the per-segment metrics hallucination scoring needs.
    // Servers that ignore it still return `text`, which scoring falls back to.
    if is_openai {
        form = form
            .text("model", settings.model.clone())
            .text("response_format", "verbose_json".to_string())
            .text("temperature", attempt.temperature.to_string());
        if settings.language != "auto" {
            form = form.text("language", settings.language.clone());
        }
    } else {
        form = form
            .text("language", settings.language.clone())
            .text("response_format", "verbose_json".to_string())
            .text("temperature", attempt.temperature.to_string())
            .text(
                "temperature_inc",
                if attempt.allow_internal_fallback {
                    "0.2"
                } else {
                    "0.0"
                }
                .to_string(),
            )
            .text("no_speech_thold", "0.65")
            .text("entropy_thold", "2.4")
            .text("suppress_nst", "true");
    }

    if let Some(prompt) = prompt.filter(|p| !p.trim().is_empty()) {
        form = form.text("prompt", prompt.to_string());
        form = form.text("initial_prompt", prompt.to_string());
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

    let mut parsed = parse_response(&body);
    parsed.text = normalize_line_breaks(&parsed.text);
    log_message(&format!(
        "[transcribe] Result: {} chars, {} segments",
        parsed.text.len(),
        parsed.segments.len()
    ));
    Ok(parsed)
}

fn parse_response(body: &str) -> RawResult {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(body) else {
        return RawResult {
            text: body.trim().to_string(),
            segments: Vec::new(),
        };
    };

    let segments: Vec<Segment> = json
        .get("segments")
        .and_then(|v| v.as_array())
        .map(|items| items.iter().filter_map(parse_segment).collect())
        .unwrap_or_default();

    let text = json
        .get("text")
        .and_then(|v| v.as_str())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| {
            segments
                .iter()
                .map(|s| s.text.trim())
                .filter(|t| !t.is_empty())
                .collect::<Vec<&str>>()
                .join(" ")
        });

    RawResult { text, segments }
}

/// Whisper servers disagree on whether numbers arrive as JSON numbers or
/// strings, so accept either.
fn loose_f64(value: Option<&serde_json::Value>) -> Option<f64> {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn parse_segment(item: &serde_json::Value) -> Option<Segment> {
    let text = item.get("text").and_then(|v| v.as_str())?.trim().to_string();
    Some(Segment {
        text,
        start: loose_f64(item.get("start")).unwrap_or(0.0),
        end: loose_f64(item.get("end")).unwrap_or(0.0),
        no_speech_prob: loose_f64(item.get("no_speech_prob")).map(|v| v as f32),
        avg_logprob: loose_f64(item.get("avg_logprob")).map(|v| v as f32),
    })
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
    use super::*;

    #[test]
    fn parses_verbose_json_segment_metrics() {
        let body = r#"{
            "text": "Ship the release notes.",
            "segments": [
                {"start": 0.0, "end": 2.4, "text": " Ship the release notes.",
                 "no_speech_prob": 0.012, "avg_logprob": -0.31}
            ]
        }"#;
        let parsed = parse_response(body);
        assert_eq!(parsed.text, "Ship the release notes.");
        assert_eq!(parsed.segments.len(), 1);
        assert_eq!(parsed.segments[0].no_speech_prob, Some(0.012));
        assert_eq!(parsed.segments[0].avg_logprob, Some(-0.31));
        assert_eq!(parsed.segments[0].end, 2.4);
    }

    #[test]
    fn parses_metrics_sent_as_strings() {
        let body = r#"{"text": "hi", "segments": [
            {"text": "hi", "start": "0.0", "end": "1.0", "no_speech_prob": "0.9"}
        ]}"#;
        let parsed = parse_response(body);
        assert_eq!(parsed.segments[0].no_speech_prob, Some(0.9));
        assert_eq!(parsed.segments[0].avg_logprob, None);
    }

    #[test]
    fn falls_back_to_plain_text_without_segments() {
        let parsed = parse_response(r#"{"text": "Just text."}"#);
        assert_eq!(parsed.text, "Just text.");
        assert!(parsed.segments.is_empty());

        let parsed = parse_response("not json at all");
        assert_eq!(parsed.text, "not json at all");
        assert!(parsed.segments.is_empty());
    }

    /// Trimmed from an actual whisper-server verbose_json response (large-v3,
    /// VAD on) so the parser and thresholds stay pinned to the real payload.
    #[test]
    fn real_server_payload_scores_clean() {
        let body = r#"{"task":"transcribe","language":"english","duration":2.4,
            "text":" Ship the release notes before the stand-up tomorrow.\n",
            "segments":[{"id":0,"text":" Ship the release notes before the stand-up tomorrow.",
            "start":0.16,"end":2.3000000000000003,"temperature":0.0,
            "avg_logprob":-0.04834417253732681,"no_speech_prob":0.01814487762749195}]}"#;
        let parsed = parse_response(body);
        let report = quality::evaluate(&parsed.text, &parsed.segments, &Thresholds::default());
        assert!(report.is_clean(), "clean speech must not be flagged");
        assert_eq!(
            report.clean_text,
            "Ship the release notes before the stand-up tomorrow."
        );
    }

    /// VAD trims silence to nothing, which must read as "no speech" rather than
    /// as a hallucination worth re-decoding.
    #[test]
    fn vad_trimmed_silence_yields_empty_result() {
        let body = r#"{"task":"transcribe","language":"english","duration":4.0,
            "text":"","segments":[]}"#;
        let parsed = parse_response(body);
        assert!(parsed.text.is_empty());
        let report = quality::evaluate(&parsed.text, &parsed.segments, &Thresholds::default());
        assert!(report.is_empty());
        assert!(!report.is_clean());
    }

    #[test]
    fn rebuilds_text_from_segments_when_top_level_text_missing() {
        let body = r#"{"segments": [
            {"text": " Hello"}, {"text": " world."}
        ]}"#;
        assert_eq!(parse_response(body).text, "Hello world.");
    }
}
