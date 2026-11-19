use crate::config::CloudService;
use crate::provider::{
    check_status, endpoint, ids, parse_verbose_json, send_error, RemoteModel, SttProvider,
    TranscribeRequest, Transcript,
};
use crate::SttError;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use url::Url;

/// Any OpenAI-compatible transcriptions API (OpenAI, Groq, or a custom
/// server).
pub struct OpenAiCompatible {
    service: CloudService,
    transcribe_url: Url,
    models_url: Url,
    model: String,
    key: String,
    http: reqwest::Client,
}

impl OpenAiCompatible {
    pub fn new(
        service: CloudService,
        base_url: &str,
        model: &str,
        key: String,
        http: reqwest::Client,
    ) -> Result<Self, SttError> {
        Ok(Self {
            service,
            transcribe_url: endpoint(base_url, "audio/transcriptions")?,
            models_url: endpoint(base_url, "models")?,
            model: model.to_string(),
            key,
            http,
        })
    }
}

#[async_trait]
impl SttProvider for OpenAiCompatible {
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

        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "verbose_json".to_string())
            .text("temperature", req.decode.temperature.to_string());
        if let Some(language) = req.language.as_deref().filter(|l| !l.trim().is_empty()) {
            form = form.text("language", language.to_string());
        }
        if let Some(prompt) = req.prompt.as_deref().filter(|p| !p.trim().is_empty()) {
            form = form.text("prompt", prompt.to_string());
        }

        let resp = self
            .http
            .post(self.transcribe_url.clone())
            .bearer_auth(&self.key)
            .multipart(form)
            .send()
            .await
            .map_err(send_error)?;
        let resp = check_status(resp).await?;
        let body = resp.text().await.map_err(|_| SttError::Unreachable)?;
        let (text, segments) = parse_verbose_json(&body)
            .ok_or_else(|| SttError::BadResponse("response was not JSON".to_string()))?;

        Ok(Transcript {
            text,
            segments,
            latency_ms: started.elapsed().as_millis() as u32,
        })
    }

    async fn list_models(&self, _cancel: CancellationToken) -> Result<Vec<RemoteModel>, SttError> {
        match self.list_models_http().await {
            // Known services ship a curated fallback when the models
            // endpoint is missing.
            Err(SttError::Server { status: 404 }) => match self.service {
                CloudService::OpenAi => Ok(ids(&[
                    "gpt-4o-transcribe",
                    "gpt-4o-mini-transcribe",
                    "whisper-1",
                ])),
                CloudService::Groq => Ok(ids(&["whisper-large-v3-turbo", "whisper-large-v3"])),
                CloudService::Deepgram | CloudService::Custom => {
                    Err(SttError::Server { status: 404 })
                }
            },
            other => other,
        }
    }
}

impl OpenAiCompatible {
    async fn list_models_http(&self) -> Result<Vec<RemoteModel>, SttError> {
        let resp = self
            .http
            .get(self.models_url.clone())
            .bearer_auth(&self.key)
            .send()
            .await
            .map_err(send_error)?;
        let resp = check_status(resp).await?;
        let body = resp.text().await.map_err(|_| SttError::Unreachable)?;
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| SttError::BadResponse(e.to_string()))?;
        let all = json
            .get("data")
            .and_then(|d| d.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.get("id").and_then(|id| id.as_str()))
                    .collect::<Vec<&str>>()
            })
            .unwrap_or_default();
        Ok(all
            .iter()
            .filter(|id| is_speech_model(id))
            .map(|id| RemoteModel { id: id.to_string() })
            .collect())
    }
}

fn is_speech_model(id: &str) -> bool {
    let id = id.to_lowercase();
    id.contains("whisper") || id.contains("transcribe")
}
