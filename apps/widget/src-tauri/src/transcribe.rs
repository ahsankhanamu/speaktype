use crate::debug;
use crate::logging::log_message;
use crate::quality::{self, Report, Segment, Thresholds};
use crate::settings::Settings;
use serde_json::json;
use speaktype_stt::{
    provider_for, CloudService, Decode, KeychainStore, ProviderConfig, SecretStore, SttError,
    SttProvider, TranscribeRequest, Transcript,
};
use tauri::{AppHandle, Emitter, Runtime};
use tokio_util::sync::CancellationToken;

const LOCAL_NAME: &str = "On this Mac";

/// The bundled clip the Providers page's Test button sends.
pub(crate) const TEST_CLIP: &[u8] = include_bytes!("../../../../assets/test-clip-3s.wav");

/// Display name for provider copy ("Groq", "On this Mac", …).
pub(crate) fn service_name(cfg: &ProviderConfig) -> &'static str {
    match cfg {
        ProviderConfig::Local { .. } => LOCAL_NAME,
        ProviderConfig::Server { .. } => "Your server",
        ProviderConfig::Cloud { service, .. } => match service {
            CloudService::OpenAi => "OpenAI",
            CloudService::Groq => "Groq",
            CloudService::Deepgram => "Deepgram",
            CloudService::Custom => "Your service",
        },
    }
}

/// Build the provider for `cfg`. Local transcription calls the whisper-server
/// the app started: `local_url` is `settings.api_url`, which `server.rs` keeps
/// pointing at the live port.
pub(crate) fn provider(
    cfg: &ProviderConfig,
    local_url: &str,
    secrets: &dyn SecretStore,
) -> Result<Box<dyn SttProvider>, SttError> {
    provider_for(cfg, local_url, secrets, reqwest::Client::new())
}

/// One clip to send: the audio plus the optional decode hints. Providers that
/// cannot express a hint ignore it.
pub(crate) struct Clip<'a> {
    pub wav: &'a [u8],
    pub duration_ms: u32,
    pub language: Option<&'a str>,
    pub prompt: Option<&'a str>,
    pub decode: Decode,
}

impl Clip<'_> {
    fn request(&self) -> TranscribeRequest {
        TranscribeRequest {
            wav: self.wav.to_vec().into(),
            duration_ms: self.duration_ms,
            language: self
                .language
                .filter(|l| !l.trim().is_empty() && *l != "auto")
                .map(str::to_string),
            prompt: self
                .prompt
                .filter(|p| !p.trim().is_empty())
                .map(str::to_string),
            decode: self.decode,
        }
    }
}

/// Send `clip` through the configured provider. A cloud provider that cannot
/// be reached (or times out) and allows it is retried on the local server, and
/// the caller is told through `on_fallback` that it happened. Other failures
/// become one plain-language message.
pub(crate) async fn run_provider(
    cfg: &ProviderConfig,
    local_url: &str,
    secrets: &dyn SecretStore,
    clip: &Clip<'_>,
    on_fallback: impl FnOnce(),
) -> Result<Transcript, String> {
    let attempt = |cfg: &ProviderConfig| {
        let req = clip.request();
        let provider = provider(cfg, local_url, secrets);
        async move { provider?.transcribe(req, CancellationToken::new()).await }
    };

    match attempt(cfg).await {
        Ok(transcript) => Ok(transcript),
        Err(e) if cloud_falls_back(cfg, &e) => {
            log_message(&format!(
                "[transcribe] {} failed ({e}); using {LOCAL_NAME}",
                service_name(cfg)
            ));
            let local = ProviderConfig::Local {
                model: String::new(),
            };
            let transcript = attempt(&local)
                .await
                .map_err(|e| e.user_message(LOCAL_NAME))?;
            on_fallback();
            Ok(transcript)
        }
        Err(e) => Err(e.user_message(service_name(cfg))),
    }
}

/// The one automatic switch (spec §6.7): a cloud service that cannot be
/// reached falls back to On this Mac, only when the user left it on.
pub(crate) fn cloud_falls_back(cfg: &ProviderConfig, err: &SttError) -> bool {
    matches!(
        cfg,
        ProviderConfig::Cloud {
            fallback_to_local: true,
            ..
        }
    ) && matches!(err, SttError::Unreachable | SttError::Timeout)
}

/// Event the widget already listens to, so a fallback shows as a plain error
/// pill without a new channel.
pub(crate) fn emit_fallback<R: Runtime>(app: &AppHandle<R>, cfg: &ProviderConfig) {
    let _ = app.emit(
        "sidecar:error",
        json!({
            "kind": "providerFallback",
            "message": format!("Couldn't reach {}. Used {LOCAL_NAME}.", service_name(cfg)),
        }),
    );
}

/// One decode configuration. The first pass runs greedy and deterministic with
/// whisper's internal temperature fallback disabled, so any invented text comes
/// from a path we control rather than a silent server-side retry.
#[derive(Debug, Clone, Copy)]
struct Attempt {
    temperature: f32,
    use_prompt: bool,
    allow_internal_fallback: bool,
}

impl Attempt {
    fn decode(&self) -> Decode {
        Decode {
            temperature: self.temperature,
            internal_fallback: self.allow_internal_fallback,
        }
    }
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

async fn transcribe_pass<R: Runtime>(
    app: &AppHandle<R>,
    settings: &Settings,
    secrets: &dyn SecretStore,
    wav: &[u8],
    duration_ms: u32,
    attempt: Attempt,
    prompt: Option<&str>,
) -> Result<Transcript, String> {
    let cfg = settings.active_provider();
    let clip = Clip {
        wav,
        duration_ms,
        language: Some(&settings.language),
        prompt: prompt.filter(|_| attempt.use_prompt),
        decode: attempt.decode(),
    };
    let transcript = run_provider(&cfg, &settings.api_url, secrets, &clip, || {
        emit_fallback(app, &cfg)
    })
    .await?;
    log_message(&format!(
        "[transcribe] {}: {} chars, {} segments",
        service_name(&cfg),
        transcript.text.len(),
        transcript.segments.len()
    ));
    Ok(transcript)
}

pub async fn transcribe<R: Runtime>(
    app: &AppHandle<R>,
    wav_data: Vec<u8>,
    settings: &Settings,
    duration_secs: f64,
) -> Result<String, String> {
    transcribe_with_prompt(app, wav_data, settings, duration_secs, None).await
}

/// Transcribe and return only text that survived hallucination scoring.
/// Empty means everything was rejected.
pub async fn transcribe_with_prompt<R: Runtime>(
    app: &AppHandle<R>,
    wav_data: Vec<u8>,
    settings: &Settings,
    duration_secs: f64,
    prompt: Option<&str>,
) -> Result<String, String> {
    let report = transcribe_verified(app, wav_data, settings, duration_secs, prompt, None).await?;
    Ok(report.clean_text)
}

/// Transcribe, score the result, and re-decode with different parameters while
/// it still looks hallucinated. Returns the best-scoring candidate rather than
/// discarding suspicious output outright.
pub async fn transcribe_verified<R: Runtime>(
    app: &AppHandle<R>,
    wav_data: Vec<u8>,
    settings: &Settings,
    duration_secs: f64,
    prompt: Option<&str>,
    debug_chunk: Option<u32>,
) -> Result<Report, String> {
    let secrets = KeychainStore;
    let duration_ms = (duration_secs * 1000.0) as u32;
    let first = transcribe_pass(
        app,
        settings,
        &secrets,
        &wav_data,
        duration_ms,
        FIRST_PASS,
        prompt,
    )
    .await?;

    if !settings.hallucination_guard {
        debug::record_pass(debug::PassEvent {
            chunk_idx: debug_chunk,
            pass: 1,
            temperature: FIRST_PASS.temperature,
            prompt_used: prompt.is_some(),
            raw_chars: first.text.len(),
            score: 0.0,
            dropped: 0,
            had_metrics: false,
            reason: "guard disabled".into(),
            segments: Vec::new(),
        });
        return Ok(Report::passthrough(&first.text));
    }

    let thresholds = Thresholds::from_settings(settings);
    let retries = (settings.hallucination_retries as usize).min(RETRY_LADDER.len());
    let passes: Vec<Attempt> = std::iter::once(FIRST_PASS)
        .chain(RETRY_LADDER.iter().take(retries).copied())
        .collect();

    let mut best: Option<Report> = None;

    for (index, pass) in passes.iter().enumerate() {
        let transcript = if index == 0 {
            Ok(first.clone())
        } else {
            transcribe_pass(
                app,
                settings,
                &secrets,
                &wav_data,
                duration_ms,
                *pass,
                prompt,
            )
            .await
        };
        let transcript = match transcript {
            Ok(transcript) => transcript,
            Err(e) if best.is_some() => {
                log_message(&format!("[quality] Retry pass {} failed: {}", index + 1, e));
                break;
            }
            Err(e) => return Err(e),
        };

        let segments: Vec<Segment> = transcript
            .segments
            .iter()
            .map(|s| Segment {
                text: s.text.clone(),
                start: s.start_ms as f64 / 1000.0,
                end: s.end_ms as f64 / 1000.0,
                no_speech_prob: s.no_speech_prob,
                avg_logprob: s.avg_logprob,
            })
            .collect();
        let silent = transcript.text.trim().is_empty();
        let report = quality::evaluate(&transcript.text, &segments, &thresholds);
        log_message(&format!(
            "[quality] Pass {} (prompt={}, metrics={}): score={:.1}, {}/{} segments dropped [{}]",
            index + 1,
            pass.use_prompt && prompt.is_some(),
            report.had_metrics,
            report.score,
            report.dropped,
            report.segments.len(),
            report.reason()
        ));
        debug::record_pass(debug::PassEvent {
            chunk_idx: debug_chunk,
            pass: index as u32 + 1,
            temperature: pass.temperature,
            prompt_used: pass.use_prompt && prompt.is_some(),
            raw_chars: transcript.text.len(),
            score: report.score,
            dropped: report.dropped,
            had_metrics: report.had_metrics,
            reason: report.reason(),
            segments: report
                .segments
                .iter()
                .enumerate()
                .map(|(i, seg)| {
                    let src = segments.get(i);
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
        if best.as_ref().is_none_or(|b| report.score > b.score) {
            best = Some(report);
        }
        if clean {
            break;
        }
        // Nothing was decoded, so there is nothing to salvage. Re-decoding
        // silence invites a hallucination instead of fixing one.
        if silent {
            log_message("[quality] No text decoded — not re-decoding");
            break;
        }
        if index + 1 < passes.len() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use speaktype_stt::MemoryStore;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn verbose_json() -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_string(
            r#"{"task":"transcribe","language":"english","duration":2.4,
                "text":" Ship the release notes before the stand-up tomorrow.\n",
                "segments":[{"id":0,"text":" Ship the release notes before the stand-up tomorrow.",
                "start":0.16,"end":2.3,"avg_logprob":-0.048,"no_speech_prob":0.018}]}"#,
        )
    }

    fn unreachable_cloud(fallback_to_local: bool) -> ProviderConfig {
        ProviderConfig::Cloud {
            service: CloudService::Groq,
            base_url: "http://127.0.0.1:9/openai/v1".to_string(),
            model: "whisper-large-v3-turbo".to_string(),
            fallback_to_local,
        }
    }

    fn clip() -> Clip<'static> {
        Clip {
            wav: b"RIFF fake wav",
            duration_ms: 2400,
            language: Some("en"),
            prompt: None,
            decode: Decode::default(),
        }
    }

    fn cloud_secrets() -> MemoryStore {
        let secrets = MemoryStore::new();
        secrets.set("stt.groq", "sk-test").unwrap();
        secrets
    }

    #[test]
    fn service_names_are_plain_language() {
        assert_eq!(
            service_name(&ProviderConfig::Local {
                model: "base".to_string()
            }),
            "On this Mac"
        );
        assert_eq!(
            service_name(&ProviderConfig::Server {
                base_url: "http://127.0.0.1:9137/inference".to_string()
            }),
            "Your server"
        );
        assert_eq!(service_name(&unreachable_cloud(true)), "Groq");
    }

    #[tokio::test]
    async fn cloud_unreachable_falls_back_to_local_when_enabled() {
        let cloud = unreachable_cloud(true);
        let local = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/inference"))
            .respond_with(verbose_json())
            .expect(1)
            .mount(&local)
            .await;
        let fell_back = std::cell::Cell::new(false);
        let secrets = cloud_secrets();

        let outcome = run_provider(
            &cloud,
            &format!("{}/inference", local.uri()),
            &secrets,
            &clip(),
            || fell_back.set(true),
        )
        .await;

        assert_eq!(
            outcome.unwrap().text,
            "Ship the release notes before the stand-up tomorrow."
        );
        assert!(
            fell_back.get(),
            "the caller must be told about the fallback"
        );
    }

    #[tokio::test]
    async fn cloud_unreachable_without_fallback_is_one_plain_message() {
        let fell_back = std::cell::Cell::new(false);
        let err = run_provider(
            &unreachable_cloud(false),
            "http://127.0.0.1:9/inference",
            &cloud_secrets(),
            &clip(),
            || fell_back.set(true),
        )
        .await
        .unwrap_err();
        // Retry policy turns a refused connection into the plain copy once it
        // gives up; either way it is one user-facing sentence, never a debug string.
        assert!(err.starts_with("Couldn't reach Groq"), "{err}");
        assert!(!fell_back.get());
    }

    #[test]
    fn retry_ladder_is_the_three_passes_from_main() {
        let ladder: Vec<(f32, bool, bool)> = std::iter::once(FIRST_PASS)
            .chain(RETRY_LADDER.iter().copied())
            .map(|a| (a.temperature, a.use_prompt, a.allow_internal_fallback))
            .collect();
        assert_eq!(
            ladder,
            vec![(0.0, true, false), (0.0, false, false), (0.4, false, true)]
        );
        assert_eq!(
            RETRY_LADDER[0].decode(),
            Decode {
                temperature: 0.0,
                internal_fallback: false
            }
        );
        assert_eq!(
            RETRY_LADDER[1].decode(),
            Decode {
                temperature: 0.4,
                internal_fallback: true
            }
        );
    }

    #[test]
    fn only_unreachable_cloud_failures_fall_back() {
        let timeout = SttError::Timeout;
        let unreachable = SttError::Unreachable;
        assert!(cloud_falls_back(&unreachable_cloud(true), &unreachable));
        assert!(cloud_falls_back(&unreachable_cloud(true), &timeout));
        assert!(!cloud_falls_back(&unreachable_cloud(false), &unreachable));
        assert!(!cloud_falls_back(
            &unreachable_cloud(true),
            &SttError::Unauthorized
        ));
        assert!(!cloud_falls_back(
            &ProviderConfig::Local {
                model: "base".to_string()
            },
            &unreachable
        ));
    }
}
