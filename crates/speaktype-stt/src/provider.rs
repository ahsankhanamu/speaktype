use crate::config::{CloudService, ProviderConfig};
use crate::secrets::SecretStore;
use crate::SttError;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use url::Url;

/// Per-pass decode parameters. The first pass is greedy and deterministic with
/// whisper's internal temperature fallback disabled, so invented text comes
/// from a path the app controls rather than a silent server-side retry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decode {
    pub temperature: f32,
    pub internal_fallback: bool,
}

impl Default for Decode {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            internal_fallback: false,
        }
    }
}

/// One clip to transcribe. `wav` is 16 kHz mono 16-bit WAV, converted once at
/// capture; `duration_ms` is the clip's audio length. Providers that cannot
/// express a decode field ignore it.
#[derive(Debug, Clone)]
pub struct TranscribeRequest {
    pub wav: bytes::Bytes,
    pub duration_ms: u32,
    pub language: Option<String>,
    pub prompt: Option<String>,
    pub decode: Decode,
}

#[derive(Debug, Clone)]
pub struct Transcript {
    pub text: String,
    pub segments: Vec<Segment>,
    pub latency_ms: u32,
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub start_ms: u32,
    pub end_ms: u32,
    pub text: String,
    pub no_speech_prob: Option<f32>,
    pub avg_logprob: Option<f32>,
    pub compression_ratio: Option<f32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct RemoteModel {
    pub id: String,
}

#[async_trait]
pub trait SttProvider: Send + Sync {
    async fn transcribe(
        &self,
        req: TranscribeRequest,
        cancel: CancellationToken,
    ) -> Result<Transcript, SttError>;
    async fn list_models(&self, cancel: CancellationToken) -> Result<Vec<RemoteModel>, SttError>;
}

/// Build the provider for `cfg`, wrapped in the retry policy. `local_url` is
/// the runtime address of the whisper-server the app runs (it picks its port
/// at start), used by [`ProviderConfig::Local`]. Fails with
/// [`SttError::NotConfigured`] when a cloud provider has no API key or no
/// usable address.
pub fn provider_for(
    cfg: &ProviderConfig,
    local_url: &str,
    secrets: &dyn SecretStore,
    http: reqwest::Client,
) -> Result<Box<dyn SttProvider>, SttError> {
    let inner: Box<dyn SttProvider> = match cfg {
        ProviderConfig::Local { model } => Box::new(crate::local::WhisperServer::new(
            local_url,
            Some(model.clone()),
            http,
        )?),
        ProviderConfig::Server { base_url } => {
            Box::new(crate::local::WhisperServer::new(base_url, None, http)?)
        }
        ProviderConfig::Cloud {
            service,
            base_url,
            model,
            ..
        } => {
            let key = match secrets.get(service.secret_account()) {
                Ok(Some(key)) => key,
                Ok(None) => return Err(SttError::NotConfigured),
                Err(_) => return Err(SttError::NotConfigured),
            };
            let base = if base_url.trim().is_empty() {
                service.default_base_url().ok_or(SttError::NotConfigured)?
            } else {
                base_url.trim_end_matches('/')
            };
            match service {
                CloudService::Deepgram => {
                    Box::new(crate::deepgram::Deepgram::new(base, model, key, http)?)
                }
                _ => Box::new(crate::openai::OpenAiCompatible::new(
                    *service, base, model, key, http,
                )?),
            }
        }
    };
    Ok(Box::new(crate::retry::WithRetry::new(inner)))
}

pub(crate) fn ids(names: &[&str]) -> Vec<RemoteModel> {
    names
        .iter()
        .map(|id| RemoteModel { id: id.to_string() })
        .collect()
}

pub(crate) fn endpoint(base: &str, path: &str) -> Result<Url, SttError> {
    let base = base.trim_end_matches('/');
    Url::parse(&format!("{base}/{path}")).map_err(|_| SttError::NotConfigured)
}

pub(crate) fn send_error(e: reqwest::Error) -> SttError {
    if e.is_timeout() {
        SttError::Timeout
    } else {
        SttError::Unreachable
    }
}

/// Map a non-success response onto the error the retry policy reasons about.
pub(crate) async fn check_status(resp: reqwest::Response) -> Result<reqwest::Response, SttError> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    Err(match resp.status().as_u16() {
        401 | 403 => SttError::Unauthorized,
        408 => SttError::Timeout,
        429 => SttError::RateLimited {
            retry_after_ms: retry_after_ms(&resp),
        },
        status => SttError::Server { status },
    })
}

/// `Retry-After` in seconds → ms; the HTTP-date form falls back to the
/// default backoff.
fn retry_after_ms(resp: &reqwest::Response) -> Option<u64> {
    let seconds = resp
        .headers()
        .get("Retry-After")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())?;
    Some(seconds * 1000)
}

/// Parse a whisper verbose_json (or OpenAI-compatible) body. Returns `None`
/// when the body is not JSON.
pub(crate) fn parse_verbose_json(body: &str) -> Option<(String, Vec<Segment>)> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;

    let segments: Vec<Segment> = json
        .get("segments")
        .and_then(|v| v.as_array())
        .map(|items| items.iter().filter_map(parse_segment).collect())
        .unwrap_or_default();

    let text = json
        .get("text")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            segments
                .iter()
                .map(|s| s.text.trim())
                .filter(|t| !t.is_empty())
                .collect::<Vec<&str>>()
                .join(" ")
        });

    Some((normalize_line_breaks(&text), segments))
}

fn parse_segment(item: &serde_json::Value) -> Option<Segment> {
    let text = item
        .get("text")
        .and_then(|v| v.as_str())?
        .trim()
        .to_string();
    let field = |key: &str| loose_f64(item.get(key));
    Some(Segment {
        text,
        start_ms: secs_to_ms(field("start").unwrap_or(0.0)),
        end_ms: secs_to_ms(field("end").unwrap_or(0.0)),
        no_speech_prob: field("no_speech_prob").map(|v| v as f32),
        avg_logprob: field("avg_logprob").map(|v| v as f32),
        compression_ratio: field("compression_ratio").map(|v| v as f32),
    })
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

fn secs_to_ms(secs: f64) -> u32 {
    (secs.max(0.0) * 1000.0).round() as u32
}

/// whisper.cpp inserts line breaks at segment boundaries (~30s chunks).
/// Join single newlines into flowing text while preserving paragraph breaks (double newlines).
pub(crate) fn normalize_line_breaks(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let joined: Vec<String> = paragraphs
        .iter()
        .map(|p| {
            p.split('\n')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
                .join(" ")
        })
        .filter(|s| !s.is_empty())
        .collect();
    joined.join("\n\n")
}
