use crate::provider::{
    check_status, normalize_line_breaks, parse_verbose_json, send_error, RemoteModel, SttProvider,
    TranscribeRequest, Transcript,
};
use crate::SttError;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use url::Url;

/// A whisper.cpp-compatible server: the bundled sidecar (`Local`) or the
/// user's own (`Server`). The request and response handling are ported
/// unchanged from the app's `transcribe_once`.
pub struct WhisperServer {
    endpoint: Url,
    model: Option<String>,
    http: reqwest::Client,
}

impl WhisperServer {
    pub fn new(
        endpoint: &str,
        model: Option<String>,
        http: reqwest::Client,
    ) -> Result<Self, SttError> {
        Ok(Self {
            endpoint: Url::parse(endpoint).map_err(|_| SttError::NotConfigured)?,
            model,
            http,
        })
    }
}

#[async_trait]
impl SttProvider for WhisperServer {
    async fn transcribe(
        &self,
        req: TranscribeRequest,
        _cancel: CancellationToken,
    ) -> Result<Transcript, SttError> {
        let started = std::time::Instant::now();

        let wav_len = req.wav.len() as u64;
        let file_part =
            reqwest::multipart::Part::stream_with_length(reqwest::Body::from(req.wav), wav_len)
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .map_err(|e| SttError::BadResponse(e.to_string()))?;

        // First-pass decode parameters: greedy and deterministic, with
        // whisper's internal temperature fallback disabled.
        let mut form = reqwest::multipart::Form::new().part("file", file_part);
        form = form
            .text(
                "language",
                req.language.unwrap_or_else(|| "auto".to_string()),
            )
            .text("response_format", "verbose_json".to_string())
            .text("temperature", req.decode.temperature.to_string())
            .text(
                "temperature_inc",
                if req.decode.internal_fallback {
                    "0.2"
                } else {
                    "0.0"
                }
                .to_string(),
            )
            .text("no_speech_thold", "0.65".to_string())
            .text("entropy_thold", "2.4".to_string())
            .text("suppress_nst", "true".to_string());
        if let Some(prompt) = req.prompt.as_deref().filter(|p| !p.trim().is_empty()) {
            form = form.text("prompt", prompt.to_string());
            form = form.text("initial_prompt", prompt.to_string());
        }

        let resp = self
            .http
            .post(self.endpoint.clone())
            .multipart(form)
            .send()
            .await
            .map_err(send_error)?;
        let resp = check_status(resp).await?;
        let body = resp.text().await.map_err(|_| SttError::Unreachable)?;

        // Servers that ignore verbose_json still return plain text, which the
        // caller treats as the whole transcript.
        let (text, segments) = parse_verbose_json(&body)
            .unwrap_or_else(|| (normalize_line_breaks(body.trim()), Vec::new()));

        Ok(Transcript {
            text,
            segments,
            latency_ms: started.elapsed().as_millis() as u32,
        })
    }

    async fn list_models(&self, _cancel: CancellationToken) -> Result<Vec<RemoteModel>, SttError> {
        // One model per server process: the local sidecar reports the model
        // it runs; a user's server has no model-listing API.
        Ok(self
            .model
            .as_deref()
            .map(|m| crate::provider::ids(&[m]))
            .unwrap_or_default())
    }
}
