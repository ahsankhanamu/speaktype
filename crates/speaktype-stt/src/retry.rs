use crate::error::SttError;
use crate::provider::{RemoteModel, SttProvider, TranscribeRequest, Transcript};
use async_trait::async_trait;
use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;

/// Exponential backoff with jitter. The default retries transient failures
/// at most 3 times, waiting `base_ms × 2^(attempt-1) × (1 ± jitter)`.
#[derive(Debug)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_ms: u64,
    pub jitter: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_ms: 500,
            jitter: 0.2,
        }
    }
}

/// `max(10 s, 3 × duration)`, capped at 120 s.
pub fn timeout_for(duration_ms: u32) -> Duration {
    Duration::from_millis((duration_ms as u64 * 3).clamp(10_000, 120_000))
}

/// `Retry-After` is honoured only up to the 10 s timeout floor; a longer
/// hint would block dictation past its budget (program plan, Review Focus 7).
const RETRY_AFTER_MAX_WAIT_MS: u64 = 10_000;

pub(crate) struct WithRetry {
    inner: Box<dyn SttProvider>,
    policy: RetryPolicy,
}

impl WithRetry {
    pub(crate) fn new(inner: Box<dyn SttProvider>) -> Self {
        Self {
            inner,
            policy: RetryPolicy::default(),
        }
    }

    /// Run `call` under the policy: each attempt bounded by
    /// [`timeout_for`], only transient failures retried, and `cancel`
    /// honoured at once.
    async fn run<T, F, Fut>(
        &self,
        duration_ms: u32,
        cancel: CancellationToken,
        mut call: F,
    ) -> Result<T, SttError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, SttError>>,
    {
        let mut attempt = 1u32;
        loop {
            let err = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(SttError::Cancelled),
                result = tokio::time::timeout(timeout_for(duration_ms), call()) => match result {
                    Ok(Ok(value)) => return Ok(value),
                    Ok(Err(err)) => err,
                    Err(_elapsed) => SttError::Timeout,
                },
            };
            if attempt >= self.policy.max_attempts || !err.is_transient() {
                return Err(err);
            }
            let backoff = match err.retry_after_ms() {
                Some(ms) if ms <= RETRY_AFTER_MAX_WAIT_MS => Duration::from_millis(ms),
                // A Retry-After beyond the cap would block dictation longer
                // than the timeout: fail now instead of waiting it out.
                Some(_) => return Err(err),
                None => self.backoff_after(attempt),
            };
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(SttError::Cancelled),
                _ = tokio::time::sleep(backoff) => {}
            }
            attempt += 1;
        }
    }

    fn backoff_after(&self, attempt: u32) -> Duration {
        let scaled = self.policy.base_ms as f64 * 2f64.powi(attempt as i32 - 1);
        let factor = 1.0 + self.policy.jitter * (2.0 * jitter_fraction() - 1.0);
        Duration::from_millis((scaled * factor).round() as u64)
    }
}

/// Uniform-ish [0, 1) from the wall clock; jitter quality is not critical.
fn jitter_fraction() -> f64 {
    let subsec = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(since) => since.subsec_nanos(),
        // Clock before the epoch: the offset itself still varies.
        Err(before) => before.duration().subsec_nanos(),
    };
    subsec as f64 / 1_000_000_000.0
}

#[async_trait]
impl SttProvider for WithRetry {
    async fn transcribe(
        &self,
        req: TranscribeRequest,
        cancel: CancellationToken,
    ) -> Result<Transcript, SttError> {
        let attempt_cancel = cancel.clone();
        let call = || self.inner.transcribe(req.clone(), attempt_cancel.clone());
        self.run(req.duration_ms, cancel, call).await
    }

    async fn list_models(&self, cancel: CancellationToken) -> Result<Vec<RemoteModel>, SttError> {
        let attempt_cancel = cancel.clone();
        let call = || self.inner.list_models(attempt_cancel.clone());
        // Model listing has no audio to scale from, so it gets the 10 s floor.
        self.run(0, cancel, call).await
    }
}
