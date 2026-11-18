use crate::provider::{
    check_status, endpoint, ids, send_error, RemoteModel, SttProvider, TranscribeRequest,
    Transcript,
};
use crate::SttError;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use url::Url;

/// The Deepgram transcription API.
pub struct Deepgram {
    listen_url: Url,
    model: String,
    key: String,
    http: reqwest::Client,
}

impl Deepgram {
    pub fn new(
        base_url: &str,
        model: &str,
        key: String,
        http: reqwest::Client,
    ) -> Result<Self, SttError> {
        Ok(Self {
            listen_url: endpoint(base_url, "listen")?,
            model: model.to_string(),
            key,
            http,
        })
    }
}

#[async_trait]
impl SttProvider for Deepgram {
    async fn transcribe(
        &self,
        req: TranscribeRequest,
        _cancel: CancellationToken,
    ) -> Result<Transcript, SttError> {
        let started = std::time::Instant::now();

        let mut url = self.listen_url.clone();
        {
            let mut pairs = url.query_pairs_mut();
            pairs
                .append_pair("model", &self.model)
                .append_pair("smart_format", "true");
            if let Some(language) = req.language.as_deref().filter(|l| !l.trim().is_empty()) {
                pairs.append_pair("language", language);
            }
        }

        let resp = self
            .http
            .post(url)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Token {}", self.key),
            )
            .header(reqwest::header::CONTENT_TYPE, "audio/wav")
            .body(req.wav)
            .send()
            .await
            .map_err(send_error)?;
        let resp = check_status(resp).await?;
        let body = resp.text().await.map_err(|_| SttError::Unreachable)?;
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| SttError::BadResponse(e.to_string()))?;
        let text = json
            .pointer("/results/channels/0/alternatives/0/transcript")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SttError::BadResponse("missing transcript".to_string()))?
            .trim()
            .to_string();

        Ok(Transcript {
            text,
            segments: Vec::new(),
            latency_ms: started.elapsed().as_millis() as u32,
        })
    }

    async fn list_models(&self, _cancel: CancellationToken) -> Result<Vec<RemoteModel>, SttError> {
        Ok(ids(&["nova-3", "nova-2"]))
    }
}
