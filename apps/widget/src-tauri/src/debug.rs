#[cfg(not(test))]
use crate::logging::log_message;
#[cfg(not(test))]
use crate::paths;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
#[cfg(not(test))]
use std::fs;
#[cfg(not(test))]
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// In-memory capture of everything that happened during one
/// recording → transcribe → paste cycle, so the Debug panel can show how
/// chunks were split and how the model was driven.
///
/// The collector is a process-global (no `AppHandle`) ring buffer, mirroring
/// the `logging` module, because the decode path runs inside spawned tasks
/// spawned by the chunk session monitor and has no access to Tauri state.
const MAX_SESSIONS: usize = 25;

/// Audio retained per session is downsampled to this rate so the ring buffer
/// stays small while still letting the Debug panel slice any chunk's audio
/// back out for playback.
const AUDIO_RETAIN_RATE: u32 = 16_000;

static SESSION_SEQ: AtomicU64 = AtomicU64::new(0);

static CURRENT: Mutex<Option<CurrentSession>> = Mutex::new(None);
static SESSIONS: Mutex<VecDeque<Session>> = Mutex::new(VecDeque::new());

/// A debug-captured transcription session under construction.
struct CurrentSession {
    id: u64,
    started: Instant,
    mode: String,
    model: String,
    duration_secs: f64,
    events: Vec<DebugEvent>,
    audio: Option<StoredAudio>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub duration_secs: f64,
    #[serde(default)]
    pub elapsed_ms: u64,
    #[serde(default)]
    pub final_chars: usize,
    #[serde(default)]
    pub chunks: Vec<ChunkEvent>,
    #[serde(default)]
    pub passes: Vec<PassEvent>,
    #[serde(default)]
    pub final_text: String,
    #[serde(skip)]
    pub audio: Option<StoredAudio>,
}

/// The recorded audio retained for a session, downsampled so a slice can be
/// pulled back out for per-chunk/per-segment playback.
#[derive(Debug, Clone)]
pub struct StoredAudio {
    pub rate: u32,
    pub samples: Vec<f32>,
}

/// Downsample an arbitrary mono `f32` buffer to `AUDIO_RETAIN_RATE` using a
/// simple windowed average. Non-monofonic rates fall back to a nearest-sample
/// pick; if the source is already at or below the target rate it is returned
/// unchanged.
fn downsample_audio(input: &[f32], rate: u32) -> StoredAudio {
    if rate <= AUDIO_RETAIN_RATE {
        return StoredAudio {
            rate,
            samples: input.to_vec(),
        };
    }
    let ratio = rate as f64 / AUDIO_RETAIN_RATE as f64;
    let out_len = (input.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let start = (i as f64 * ratio) as usize;
        let end = (((i + 1) as f64) * ratio).ceil() as usize;
        let end = end.min(input.len());
        if end <= start {
            out.push(input[start]);
            continue;
        }
        let sum: f64 = input[start..end].iter().map(|&s| s as f64).sum();
        out.push((sum / (end - start) as f64) as f32);
    }
    StoredAudio {
        rate: AUDIO_RETAIN_RATE,
        samples: out,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkEvent {
    #[serde(default)]
    pub idx: u32,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub duration_secs: f64,
    /// Start of this chunk in the recording, in seconds.
    #[serde(default)]
    pub start_secs: f64,
    /// End of this chunk in the recording, in seconds.
    #[serde(default)]
    pub end_secs: f64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub chars: usize,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub seeded_prompt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassEvent {
    /// Chunk this pass belongs to; None for a whole-recording decode.
    #[serde(default)]
    pub chunk_idx: Option<u32>,
    #[serde(default)]
    pub pass: u32,
    #[serde(default)]
    pub temperature: f32,
    #[serde(default)]
    pub prompt_used: bool,
    #[serde(default)]
    pub raw_chars: usize,
    #[serde(default)]
    pub score: f32,
    #[serde(default)]
    pub dropped: usize,
    #[serde(default)]
    pub had_metrics: bool,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub segments: Vec<SegmentDebug>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentDebug {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub start: f64,
    #[serde(default)]
    pub end: f64,
    #[serde(default)]
    pub no_speech_prob: Option<f32>,
    #[serde(default)]
    pub avg_logprob: Option<f32>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub bad: bool,
}

enum DebugEvent {
    /// A pipelined chunk whose speech was accepted for decode. A silence
    /// stretch that the monitor committed but skipped is recorded as
    /// `Silence`, so the panel can show that no audio was lost.
    Pipelined { idx: u32, start_secs: f64, end_secs: f64 },
    FinalTail { idx: u32, start_secs: f64, end_secs: f64 },
    PostHoc { idx: u32, start_secs: f64, end_secs: f64 },
    Silence { start_secs: f64, end_secs: f64 },
    ChunkAccepted { idx: u32, chars: usize, clean: bool, seeded: bool },
    ChunkRejected { idx: u32, reason: String },
    ChunkFailed { idx: u32, error: String },
    Pass(PassEvent),
}

/// Start capturing a new transcription session. Destroys any not-yet-finalized
/// session (e.g. a cancelled recording).
pub fn start_session(model: &str) {
    let mut guard = match CURRENT.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    *guard = Some(CurrentSession {
        id: SESSION_SEQ.fetch_add(1, Ordering::SeqCst),
        started: Instant::now(),
        mode: "live".to_string(),
        model: model.to_string(),
        duration_secs: 0.0,
        events: Vec::new(),
        audio: None,
    });
}

/// Fill in metadata that is only known once the recording has finished (used
/// before `finish_session`).
pub fn update_meta(duration_secs: f64, mode: &str) {
    if let Ok(mut guard) = CURRENT.lock() {
        if let Some(cur) = guard.as_mut() {
            cur.duration_secs = duration_secs;
            cur.mode = mode.to_string();
        }
    }
}

/// Abandon the in-progress session without publishing it (recording cancelled,
/// nothing transcribed).
pub fn discard_session() {
    if let Ok(mut guard) = CURRENT.lock() {
        guard.take();
    }
}

/// Finish the current session, move it into the retained ring buffer, and
/// return its text for callers that want to show a preview.
pub fn finish_session(final_text: &str) -> u64 {
    let taken = {
        let mut guard = match CURRENT.lock() {
            Ok(g) => g,
            Err(_) => return 0,
        };
        guard.take()
    };
    let Some(cur) = taken else { return 0 };

    let elapsed_ms = cur.started.elapsed().as_millis() as u64;
    let mut chunks = Vec::new();
    let mut passes = Vec::new();
    for event in cur.events {
        match event {
            DebugEvent::Pipelined { idx, start_secs, end_secs } => chunks.push(ChunkEvent {
                idx,
                source: "pipelined".into(),
                duration_secs: end_secs - start_secs,
                start_secs,
                end_secs,
                status: "pipelined".into(),
                chars: 0,
                reason: String::new(),
                seeded_prompt: false,
            }),
            DebugEvent::FinalTail { idx, start_secs, end_secs } => chunks.push(ChunkEvent {
                idx,
                source: "final_tail".into(),
                duration_secs: end_secs - start_secs,
                start_secs,
                end_secs,
                status: "pipelined".into(),
                chars: 0,
                reason: String::new(),
                seeded_prompt: false,
            }),
            DebugEvent::PostHoc { idx, start_secs, end_secs } => chunks.push(ChunkEvent {
                idx,
                source: "post_hoc".into(),
                duration_secs: end_secs - start_secs,
                start_secs,
                end_secs,
                status: "pipelined".into(),
                chars: 0,
                reason: String::new(),
                seeded_prompt: false,
            }),
            DebugEvent::Silence { start_secs, end_secs } => chunks.push(ChunkEvent {
                idx: u32::MAX,
                source: "silence".into(),
                duration_secs: end_secs - start_secs,
                start_secs,
                end_secs,
                status: "silence".into(),
                chars: 0,
                reason: "complete silence (skipped by speech gate)".into(),
                seeded_prompt: false,
            }),
            DebugEvent::ChunkAccepted { idx, chars, clean, seeded } => {
                if let Some(c) = chunks.iter_mut().find(|c| c.idx == idx) {
                    c.status = if clean { "accepted" } else { "accepted_cleaned" }.into();
                    c.chars = chars;
                    c.reason = if clean {
                        String::new()
                    } else {
                        "hallucination guard removed segments".into()
                    };
                    c.seeded_prompt = seeded;
                }
            }
            DebugEvent::ChunkRejected { idx, reason } => {
                if let Some(c) = chunks.iter_mut().find(|c| c.idx == idx) {
                    c.status = "rejected".into();
                    c.reason = reason;
                }
            }
            DebugEvent::ChunkFailed { idx, error } => {
                if let Some(c) = chunks.iter_mut().find(|c| c.idx == idx) {
                    c.status = "failed".into();
                    c.reason = error;
                }
            }
            DebugEvent::Pass(pass) => passes.push(pass),
        }
    }

    chunks.sort_by(|a, b| a.start_secs.partial_cmp(&b.start_secs).unwrap_or(std::cmp::Ordering::Equal));
    passes.sort_by_key(|p| p.chunk_idx.unwrap_or(0));

    let session = Session {
        id: cur.id,
        started_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        mode: cur.mode,
        model: cur.model,
        duration_secs: cur.duration_secs,
        elapsed_ms,
        final_chars: final_text.len(),
        chunks,
        passes,
        final_text: final_text.to_string(),
        audio: cur.audio,
    };

    if let Ok(mut guard) = SESSIONS.lock() {
        guard.push_front(session.clone());
        while guard.len() > MAX_SESSIONS {
            if let Some(evicted) = guard.pop_back() {
                let _ = evicted;
                #[cfg(not(test))]
                delete_audio(evicted.id);
            }
        }
    }

    #[cfg(not(test))]
    persist_session(&session);

    cur.id
}

pub fn record_pipelined(idx: u32, start_secs: f64, end_secs: f64) {
    push(DebugEvent::Pipelined { idx, start_secs, end_secs });
}

pub fn record_final_tail(idx: u32, start_secs: f64, end_secs: f64) {
    push(DebugEvent::FinalTail { idx, start_secs, end_secs });
}

pub fn record_post_hoc(idx: u32, start_secs: f64, end_secs: f64) {
    push(DebugEvent::PostHoc { idx, start_secs, end_secs });
}

pub fn record_silence(start_secs: f64, end_secs: f64) {
    push(DebugEvent::Silence { start_secs, end_secs });
}

pub fn record_chunk_accepted(idx: u32, chars: usize, clean: bool, seeded: bool) {
    push(DebugEvent::ChunkAccepted { idx, chars, clean, seeded });
}

pub fn record_chunk_rejected(idx: u32, reason: String) {
    push(DebugEvent::ChunkRejected { idx, reason });
}

pub fn record_chunk_failed(idx: u32, error: String) {
    push(DebugEvent::ChunkFailed { idx, error });
}

pub fn record_pass(pass: PassEvent) {
    push(DebugEvent::Pass(pass));
}

/// Attach the recording to the current session so chunk audio can be replayed
/// later. Downsampled in place to keep the ring buffer cheap.
pub fn capture_audio(samples: &[f32], sample_rate: u32) {
    if let Ok(mut guard) = CURRENT.lock() {
        if let Some(cur) = guard.as_mut() {
            if cur.audio.is_none() {
                cur.audio = Some(downsample_audio(samples, sample_rate));
            }
        }
    }
}

fn push(event: DebugEvent) {
    if let Ok(mut guard) = CURRENT.lock() {
        if let Some(cur) = guard.as_mut() {
            cur.events.push(event);
        }
    }
}

/// Sessions newest-first.
pub fn sessions() -> Vec<Session> {
    match SESSIONS.lock() {
        Ok(guard) => guard.iter().cloned().collect(),
        Err(_) => Vec::new(),
    }
}

pub fn clear() {
    if let Ok(mut guard) = SESSIONS.lock() {
        guard.clear();
    }
    #[cfg(not(test))]
    clear_persisted();
}

/// Remove a single collected debug session by id, along with its persisted
/// audio. Returns whether a session with that id was present.
pub fn delete_session(id: u64) -> bool {
    let removed = match SESSIONS.lock() {
        Ok(mut guard) => {
            let before = guard.len();
            guard.retain(|s| s.id != id);
            guard.len() != before
        }
        Err(_) => false,
    };

    #[cfg(not(test))]
    if removed {
        delete_audio(id);
        persist_sessions_only();
    }
    removed
}

/* ---- Persistence ---- */

/// The collector is kept on disk as `debug_sessions.json` plus one downsampled
/// WAV per session under `debug_audio/`, so the panel survives app restarts.
/// In-memory audio is not reloaded eagerly; slices read the WAV on demand.

#[cfg(not(test))]
fn persist_path() -> PathBuf {
    paths::config_dir().join("debug_sessions.json")
}

#[cfg(not(test))]
fn audio_dir() -> PathBuf {
    paths::config_dir().join("debug_audio")
}

#[cfg(not(test))]
fn audio_path(id: u64) -> PathBuf {
    audio_dir().join(format!("{}.wav", id))
}

/// Reload persisted sessions into the ring buffer and continue the ID
/// sequence past the last one, so restarted sessions never collide.
#[cfg(not(test))]
pub fn load_persisted() {
    let path = persist_path();
    let sessions: Vec<Session> = if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Vec<Session>>(&content) {
                Ok(v) => v,
                Err(e) => {
                    let backup = path.with_extension("json.corrupt");
                    let _ = fs::rename(&path, &backup);
                    log_message(&format!(
                        "[debug] Corrupt debug_sessions.json, backed up to debug_sessions.json.corrupt: {}",
                        e
                    ));
                    Vec::new()
                }
            },
            Err(e) => {
                log_message(&format!("[debug] Failed to read debug_sessions.json: {}", e));
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let count = sessions.len();
    if let Ok(mut guard) = SESSIONS.lock() {
        for s in sessions {
            guard.push_back(s);
        }
        while guard.len() > MAX_SESSIONS {
            guard.pop_back();
        }
        let max_id = guard.iter().map(|s| s.id).max().unwrap_or(0);
        SESSION_SEQ.store(max_id.saturating_add(1), Ordering::SeqCst);
    }
    log_message(&format!("[debug] Loaded {} persisted debug session(s)", count));
}

#[cfg(not(test))]
fn persist_session(session: &Session) {
    if let Some(audio) = session.audio.as_ref() {
        if fs::create_dir_all(audio_dir()).is_ok() {
            match crate::audio::to_wav(&audio.samples, audio.rate) {
                Ok(wav) => {
                    if let Err(e) = fs::write(audio_path(session.id), &wav) {
                        log_message(&format!(
                            "[debug] Failed to persist audio for session {}: {}",
                            session.id, e
                        ));
                    }
                }
                Err(e) => log_message(&format!(
                    "[debug] Failed to encode audio for session {}: {}",
                    session.id, e
                )),
            }
        }
    }

    persist_sessions_only();
}

/// Rewrite `debug_sessions.json` from the in-memory ring buffer.
#[cfg(not(test))]
fn persist_sessions_only() {
    match SESSIONS.lock() {
        Ok(guard) => {
            let all: Vec<Session> = guard.iter().cloned().collect();
            match serde_json::to_string_pretty(&all) {
                Ok(json) => {
                    if let Err(e) = fs::write(persist_path(), json) {
                        log_message(&format!("[debug] Failed to persist sessions: {}", e));
                    }
                }
                Err(e) => log_message(&format!("[debug] Failed to serialize sessions: {}", e)),
            }
        }
        Err(_) => {}
    }
}

#[cfg(not(test))]
fn delete_audio(id: u64) {
    let path = audio_path(id);
    if path.exists() {
        if let Err(e) = fs::remove_file(&path) {
            log_message(&format!("[debug] Failed to delete audio {:?}: {}", path, e));
        }
    }
}

#[cfg(not(test))]
fn clear_persisted() {
    let _ = fs::create_dir_all(paths::config_dir());
    let _ = fs::write(persist_path(), "[]".to_string());
    if let Ok(entries) = fs::read_dir(audio_dir()) {
        for entry in entries.flatten() {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Load a session's recording from its persisted WAV, if present.
#[cfg(not(test))]
fn persisted_audio(id: u64) -> Option<(Vec<f32>, u32)> {
    let path = audio_path(id);
    if !path.exists() {
        return None;
    }
    crate::audio::from_wav(&fs::read(&path).ok()?).ok()
}

#[cfg(test)]
fn persisted_audio(_id: u64) -> Option<(Vec<f32>, u32)> {
    None
}

/// Render the audio between `[start_secs, end_secs]` of the named session as a
/// mono WAV. Uses the retained (downsampled) recording, so playback matches
/// what the chunk actually carried. Returns `None` if the session holds no
/// audio or the requested range is outside it.
pub fn slice_audio(session_id: u64, start_secs: f64, end_secs: f64) -> Option<Vec<u8>> {
    if start_secs < 0.0 || end_secs <= start_secs {
        return None;
    }
    let guard = SESSIONS.lock().ok()?;
    let session = guard.iter().find(|s| s.id == session_id)?;

    use std::borrow::Cow;
    let disk_owned;
    let rate;
    let samples: Cow<'_, [f32]> = match session.audio.as_ref() {
        Some(audio) => {
            rate = audio.rate;
            Cow::Borrowed(&audio.samples)
        }
        // Sessions reloaded from disk hold no audio in memory; pull the WAV
        // on demand so waveforms and playback still work after a restart.
        None => {
            let (disk, disk_rate) = persisted_audio(session_id)?;
            rate = disk_rate;
            disk_owned = disk;
            Cow::Owned(disk_owned)
        }
    };

    let start = (start_secs * rate as f64).floor() as usize;
    let end = (end_secs * rate as f64).ceil() as usize;
    if start >= samples.len() {
        return None;
    }
    let end = end.min(samples.len());
    if end <= start {
        return None;
    }
    crate::audio::to_wav(&samples[start..end], rate).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The collector is global, so the debug tests must not run their
    /// start/record/finish sequences interleaved.
    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn finishes_and_retains_a_session() {
        let _guard = test_lock();
        clear();
        SESSION_SEQ.store(1, Ordering::SeqCst);
        start_session("small");
        update_meta(4.2, "live");
        record_pipelined(0, 0.0, 2.0);
        record_pipelined(1, 2.0, 4.2);
        record_silence(2.0, 2.9);
        record_chunk_accepted(0, 12, true, true);
        record_pass(PassEvent {
            chunk_idx: Some(0),
            pass: 1,
            temperature: 0.0,
            prompt_used: true,
            raw_chars: 12,
            score: 95.0,
            dropped: 0,
            had_metrics: true,
            reason: "clean".into(),
            segments: Vec::new(),
        });
        let id = finish_session("hello world");
        assert_ne!(id, 0);
        let s = sessions();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].chunks.len(), 3);
        assert_eq!(s[0].chunks[0].status, "accepted");
        assert_eq!(s[0].chunks[2].status, "silence");
        assert_eq!(s[0].passes.len(), 1);
        assert_eq!(s[0].final_text, "hello world");
    }

    #[test]
    fn discard_does_not_retain() {
        let _guard = test_lock();
        clear();
        start_session("small");
        record_pipelined(0, 0.0, 1.0);
        discard_session();
        assert!(sessions().is_empty());
    }

    #[test]
    fn ring_buffer_caps_at_max() {
        let _guard = test_lock();
        clear();
        for i in 0..(MAX_SESSIONS + 10) {
            start_session("small");
            finish_session(&format!("text {}", i));
        }
        assert_eq!(sessions().len(), MAX_SESSIONS);
    }
}