use crate::audio;
use crate::logging::log_message;
use crate::quality::Report;
use crate::settings::Settings;
use crate::transcribe;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tokio::sync::Mutex as AsyncMutex;

const MONITOR_INTERVAL: Duration = Duration::from_millis(150);
const MAX_PROMPT_CHARS: usize = 800;

pub struct ChunkSession {
    settings: Settings,
    _sample_rate: u32,
    _buffer: Arc<Mutex<Vec<f32>>>,
    committed_samples: Arc<AtomicUsize>,
    stop_monitor: Arc<AtomicBool>,
    chunk_results: Arc<Mutex<BTreeMap<u32, String>>>,
    prompt_tail: Arc<Mutex<String>>,
    next_chunk_idx: Arc<AtomicU32>,
    pending_tasks: Arc<AsyncMutex<Vec<JoinHandle<()>>>>,
    pending_count: Arc<AtomicU32>,
    pipelined: Arc<AtomicBool>,
    monitor_handle: Mutex<Option<JoinHandle<()>>>,
}

impl ChunkSession {
    pub fn start(
        settings: Settings,
        sample_rate: u32,
        buffer: Arc<Mutex<Vec<f32>>>,
    ) -> Self {
        let committed_samples = Arc::new(AtomicUsize::new(0));
        let stop_monitor = Arc::new(AtomicBool::new(false));
        let chunk_results = Arc::new(Mutex::new(BTreeMap::new()));
        let prompt_tail = Arc::new(Mutex::new(String::new()));
        let next_chunk_idx = Arc::new(AtomicU32::new(0));
        let pending_tasks = Arc::new(AsyncMutex::new(Vec::new()));
        let pending_count = Arc::new(AtomicU32::new(0));
        let pipelined = Arc::new(AtomicBool::new(false));

        let session = Self {
            settings: settings.clone(),
            _sample_rate: sample_rate,
            _buffer: buffer.clone(),
            committed_samples: committed_samples.clone(),
            stop_monitor: stop_monitor.clone(),
            chunk_results: chunk_results.clone(),
            prompt_tail: prompt_tail.clone(),
            next_chunk_idx: next_chunk_idx.clone(),
            pending_tasks: pending_tasks.clone(),
            pending_count: pending_count.clone(),
            pipelined: pipelined.clone(),
            monitor_handle: Mutex::new(None),
        };

        let monitor = tauri::async_runtime::spawn(async move {
            while !stop_monitor.load(Ordering::SeqCst) {
                tokio::time::sleep(MONITOR_INTERVAL).await;
                if stop_monitor.load(Ordering::SeqCst) {
                    break;
                }

                let total_samples = buffer.lock().map(|b| b.len()).unwrap_or(0);
                let committed = committed_samples.load(Ordering::SeqCst);
                if committed >= total_samples {
                    continue;
                }

                // Single-threaded whisper sidecar: one in-flight chunk at a time.
                if pending_count.load(Ordering::SeqCst) > 0 {
                    continue;
                }

                let pending: Vec<f32> = {
                    let buf = match buffer.lock() {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    if committed >= buf.len() {
                        continue;
                    }
                    buf[committed..].to_vec()
                };

                let Some(split) = audio::find_phrase_flush_point(&pending, sample_rate) else {
                    continue;
                };
                if split == 0 {
                    continue;
                }

                if committed_samples
                    .compare_exchange(
                        committed,
                        committed + split,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    )
                    .is_err()
                {
                    continue;
                }

                let chunk = pending[..split].to_vec();
                if !audio::has_speech(&chunk, sample_rate) {
                    continue;
                }

                pipelined.store(true, Ordering::SeqCst);

                let idx = next_chunk_idx.fetch_add(1, Ordering::SeqCst);
                log_message(&format!(
                    "[chunk] Pipelining chunk {} ({:.1}s, committed={}/{})",
                    idx,
                    chunk.len() as f64 / sample_rate as f64,
                    committed + split,
                    total_samples
                ));

                if stop_monitor.load(Ordering::SeqCst) {
                    break;
                }

                spawn_chunk_task(
                    settings.clone(),
                    sample_rate,
                    chunk,
                    idx,
                    chunk_results.clone(),
                    prompt_tail.clone(),
                    pending_tasks.clone(),
                    pending_count.clone(),
                );
            }
        });

        if let Ok(mut handle) = session.monitor_handle.lock() {
            *handle = Some(monitor);
        }

        session
    }

    /// Stop the live monitor and optionally wait briefly for it to exit.
    pub fn abort(&self) {
        self.stop_monitor.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.monitor_handle.lock() {
            if let Some(handle) = guard.take() {
                tauri::async_runtime::block_on(async {
                    let _ = tokio::time::timeout(Duration::from_millis(500), handle).await;
                });
            }
        }
    }

    pub async fn finalize(self, samples: Vec<f32>, sample_rate: u32) -> Option<String> {
        self.stop_monitor.store(true, Ordering::SeqCst);

        let monitor = {
            let mut guard = self.monitor_handle.lock().ok()?;
            guard.take()
        };
        if let Some(h) = monitor {
            let _ = h.await;
        }

        let committed = self.committed_samples.load(Ordering::SeqCst);
        let tail = if committed < samples.len() {
            samples[committed..].to_vec()
        } else {
            Vec::new()
        };

        let pipelined = self.pipelined.load(Ordering::SeqCst);

        if !pipelined {
            if samples.is_empty() || !audio::has_speech(&samples, sample_rate) {
                return None;
            }
            log_message("[chunk] Short recording — single-shot transcription");
            return transcribe_samples(&samples, sample_rate, &self.settings, None)
                .await
                .ok()
                .map(|report| report.clean_text);
        }

        if !tail.is_empty() && audio::has_speech(&tail, sample_rate) {
            let idx = self.next_chunk_idx.fetch_add(1, Ordering::SeqCst);
            log_message(&format!(
                "[chunk] Final tail chunk {} ({:.1}s)",
                idx,
                tail.len() as f64 / sample_rate as f64
            ));
            spawn_chunk_task(
                self.settings.clone(),
                sample_rate,
                tail,
                idx,
                self.chunk_results.clone(),
                self.prompt_tail.clone(),
                self.pending_tasks.clone(),
                self.pending_count.clone(),
            );
        }

        while self.pending_count.load(Ordering::SeqCst) > 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let mut tasks = self.pending_tasks.lock().await;
        for task in tasks.drain(..) {
            let _ = task.await;
        }
        drop(tasks);

        let total_chunks = self.next_chunk_idx.load(Ordering::SeqCst);
        let results = self.chunk_results.lock().ok()?;
        let mut parts: Vec<String> = Vec::new();
        for i in 0..total_chunks {
            if let Some(text) = results.get(&i) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
        }

        if parts.is_empty() {
            return None;
        }

        let combined = parts.join(" ");
        log_message(&format!(
            "[chunk] Assembled {} pipelined chunks → {} chars",
            parts.len(),
            combined.len()
        ));
        Some(combined)
    }
}

impl Drop for ChunkSession {
    fn drop(&mut self) {
        self.stop_monitor.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.monitor_handle.lock() {
            guard.take();
        }
    }
}

fn spawn_chunk_task(
    settings: Settings,
    sample_rate: u32,
    chunk: Vec<f32>,
    idx: u32,
    chunk_results: Arc<Mutex<BTreeMap<u32, String>>>,
    prompt_tail: Arc<Mutex<String>>,
    pending_tasks: Arc<AsyncMutex<Vec<JoinHandle<()>>>>,
    pending_count: Arc<AtomicU32>,
) {
    pending_count.fetch_add(1, Ordering::SeqCst);
    let pending_count_done = pending_count.clone();
    let handle = tauri::async_runtime::spawn(async move {
        let _done = scopeguard::guard((), |_| {
            pending_count_done.fetch_sub(1, Ordering::SeqCst);
        });

        if let Ok(results) = chunk_results.lock() {
            if results.contains_key(&idx) {
                log_message(&format!(
                    "[chunk] Chunk {} already in results — skipping duplicate",
                    idx
                ));
                return;
            }
        }

        let prompt = prompt_tail.lock().ok().map(|p| p.clone()).unwrap_or_default();
        let prompt_ref = if prompt.is_empty() {
            None
        } else {
            Some(prompt.as_str())
        };

        match transcribe_samples(&chunk, sample_rate, &settings, prompt_ref).await {
            Ok(report) if !report.is_empty() => {
                let text = report.clean_text.clone();
                if let Ok(mut results) = chunk_results.lock() {
                    if results.contains_key(&idx) {
                        log_message(&format!(
                            "[chunk] Chunk {} result already stored — ignoring duplicate",
                            idx
                        ));
                    } else {
                        results.insert(idx, text.clone());
                    }
                }
                // Only fully clean chunks seed the next prompt: feeding salvaged
                // text back is how a single bad chunk turns into a repetition loop.
                if report.is_clean() {
                    if let Ok(mut tail) = prompt_tail.lock() {
                        let combined = if tail.is_empty() {
                            text.clone()
                        } else {
                            format!("{} {}", tail.trim(), text.trim())
                        };
                        *tail = if combined.len() > MAX_PROMPT_CHARS {
                            combined[combined.len() - MAX_PROMPT_CHARS..].to_string()
                        } else {
                            combined
                        };
                    }
                } else {
                    log_message(&format!(
                        "[chunk] Chunk {} salvaged — not seeding prompt [{}]",
                        idx,
                        report.reason()
                    ));
                }
                log_message(&format!("[chunk] Chunk {} done: {} chars", idx, text.len()));
            }
            Ok(report) => {
                log_message(&format!(
                    "[chunk] Chunk {} rejected [{}] — skipped",
                    idx,
                    report.reason()
                ));
            }
            Err(e) => {
                log_message(&format!("[chunk] Chunk {} failed: {}", idx, e));
            }
        }
    });

    let pending = pending_tasks.clone();
    tauri::async_runtime::spawn(async move {
        pending.lock().await.push(handle);
    });
}

async fn transcribe_samples(
    samples: &[f32],
    sample_rate: u32,
    settings: &Settings,
    prompt: Option<&str>,
) -> Result<Report, String> {
    let normalized = audio::normalize_audio(samples);
    let duration_secs = normalized.len() as f64 / sample_rate as f64;
    let wav = audio::to_wav(&normalized, sample_rate)?;
    transcribe::transcribe_verified(wav, settings, duration_secs, prompt).await
}

/// Transcribe long audio by splitting on phrase boundaries (fallback when no live session).
pub async fn transcribe_chunked(
    samples: Vec<f32>,
    sample_rate: u32,
    settings: &Settings,
) -> Result<String, String> {
    let duration_secs = samples.len() as f64 / sample_rate as f64;
    let chunks = audio::split_on_phrase_boundaries(&samples, sample_rate);
    if chunks.len() <= 1 {
        let wav = audio::to_wav(&audio::normalize_audio(&samples), sample_rate)?;
        return transcribe::transcribe(wav, settings, duration_secs).await;
    }

    log_message(&format!(
        "[chunk] Post-hoc split into {} phrase chunks ({:.1}s total)",
        chunks.len(),
        duration_secs
    ));

    let mut prompt_tail = String::new();
    let mut parts = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let prompt_ref = if prompt_tail.is_empty() {
            None
        } else {
            Some(prompt_tail.as_str())
        };
        match transcribe_samples(chunk, sample_rate, settings, prompt_ref).await {
            Ok(report) if !report.is_empty() => {
                let text = report.clean_text.clone();
                parts.push(text.trim().to_string());
                if report.is_clean() {
                    prompt_tail = if prompt_tail.is_empty() {
                        text.clone()
                    } else {
                        format!("{} {}", prompt_tail.trim(), text.trim())
                    };
                    if prompt_tail.len() > MAX_PROMPT_CHARS {
                        prompt_tail =
                            prompt_tail[prompt_tail.len() - MAX_PROMPT_CHARS..].to_string();
                    }
                }
                log_message(&format!("[chunk] Post-hoc chunk {} done", i));
            }
            Ok(report) => log_message(&format!(
                "[chunk] Post-hoc chunk {} rejected [{}]",
                i,
                report.reason()
            )),
            Err(e) => log_message(&format!("[chunk] Post-hoc chunk {} failed: {}", i, e)),
        }
    }

    if parts.is_empty() {
        return Err("No speech in chunked transcription".to_string());
    }

    Ok(parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn test_session() -> ChunkSession {
        let settings = Settings::default();
        let buffer = Arc::new(Mutex::new(Vec::new()));
        ChunkSession::start(settings, 16_000, buffer)
    }

    #[test]
    fn abort_sets_stop_monitor() {
        let session = test_session();
        assert!(!session.stop_monitor.load(Ordering::SeqCst));
        session.abort();
        assert!(session.stop_monitor.load(Ordering::SeqCst));
    }

    #[test]
    fn drop_sets_stop_monitor() {
        let stop = {
            let session = test_session();
            let stop = session.stop_monitor.clone();
            drop(session);
            stop
        };
        std::thread::sleep(Duration::from_millis(200));
        assert!(stop.load(Ordering::SeqCst));
    }
}
