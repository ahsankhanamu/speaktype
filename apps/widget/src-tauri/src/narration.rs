//! Continuous narration for a Show & Tell session: the mic stays live while a
//! session runs, a small voice-activity splitter cuts the audio into phrases at
//! pauses, and each phrase is transcribed in the background into a speech item.
//! Audio is drained as it is cut, so a long session never piles up samples.

use crate::audio::{to_wav, AudioRecorder};
use crate::commands::AppState;
use crate::logging::log_message;
use serde::Serialize;
use speaktype_session::{NewItem, Store};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// How often the splitter looks at new audio.
const TICK: Duration = Duration::from_millis(200);
/// Analysis frame for speech detection.
const FRAME_MS: u32 = 20;
/// A frame louder than this (RMS) counts as speech.
const SPEECH_RMS: f32 = 0.012;
/// A pause this long after speech ends a phrase…
const PAUSE_MS: u32 = 700;
/// …as long as the phrase is at least this long.
const MIN_PHRASE_MS: u32 = 1_200;
/// A phrase is cut here even without a pause, so captions keep up.
const MAX_PHRASE_MS: u32 = 25_000;
/// Audio with no speech is dropped after this long.
const DROP_SILENCE_MS: u32 = 3_000;
/// A session with no ink, speech or copied text for this long ends itself.
pub const IDLE_END_MS: i64 = 120_000;

/// The latest transcribed phrase, for live captions under the session bar.
pub const CAPTION_EVENT: &str = "session:caption";
/// The live input level (0..1) while a session listens.
pub const LEVEL_EVENT: &str = "session:level";

#[derive(Serialize, Clone)]
struct Caption {
    text: String,
}

#[derive(Serialize, Clone)]
struct Level {
    level: f32,
}

/// What to do with the audio gathered since the last cut.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Action {
    Wait,
    /// Transcribe it as a phrase.
    Cut,
    /// No speech in it: discard.
    Drop,
}

/// The splitter's decision from what it has heard since the last cut.
pub fn decide(heard_speech: bool, trailing_silence_ms: u32, length_ms: u32) -> Action {
    if !heard_speech {
        return if length_ms >= DROP_SILENCE_MS {
            Action::Drop
        } else {
            Action::Wait
        };
    }
    if length_ms >= MAX_PHRASE_MS || (trailing_silence_ms >= PAUSE_MS && length_ms >= MIN_PHRASE_MS)
    {
        Action::Cut
    } else {
        Action::Wait
    }
}

fn rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt()
}

/// The splitter's running view of the audio since the last cut.
#[derive(Default)]
struct Phrase {
    /// Samples already looked at, from the start of the buffer.
    scanned: usize,
    heard_speech: bool,
    trailing_silence_ms: u32,
    /// Wall-clock start of the phrase (epoch ms).
    started_ms: i64,
}

struct Running {
    stop: Arc<AtomicBool>,
    done: tokio::sync::oneshot::Receiver<()>,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);
/// Last time the session saw ink, speech or a copied clip (epoch ms).
static LAST_ACTIVITY: AtomicI64 = AtomicI64::new(0);

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// When audio already in a buffer began: `samples` at `sample_rate`, as of `now`.
fn heard_since(now: i64, samples: usize, sample_rate: u32) -> i64 {
    now - (samples as u64 * 1000 / sample_rate.max(1) as u64) as i64
}

/// Something happened in the session (a mark, a clip): it isn't idle.
pub fn touch() {
    LAST_ACTIVITY.store(now_ms(), Ordering::SeqCst);
}

/// Start listening for `session_id`. A no-op if narration is already running.
/// `live` is a recorder already listening (a dictation the session took over):
/// it keeps running, and what it has heard becomes the first phrase. Without
/// one, the session opens the microphone itself.
pub fn start(app: &AppHandle, session_id: i64, live: Option<AudioRecorder>) {
    let Ok(mut running) = RUNNING.lock() else {
        return;
    };
    if running.is_some() {
        return;
    }
    let state = app.state::<AppState>();
    let (settings, device) = match state.settings.lock() {
        Ok(s) => (s.clone(), s.input_device.clone()),
        Err(_) => return,
    };
    let recorder = match live {
        Some(recorder) => {
            log_message(&format!(
                "[narration] Session {session_id} continues the dictation's recording"
            ));
            recorder
        }
        None => {
            let mut recorder = match AudioRecorder::new() {
                Ok(r) => r,
                Err(e) => return log_message(&format!("[narration] No recorder: {e}")),
            };
            if let Err(e) = recorder.start_with_input(app.clone(), device.as_deref(), || {}) {
                return log_message(&format!("[narration] Mic did not start: {e}"));
            }
            recorder
        }
    };
    log_message(&format!("[narration] Listening for session {session_id}"));
    touch();

    let stop = Arc::new(AtomicBool::new(false));
    let (done_tx, done) = tokio::sync::oneshot::channel();
    *running = Some(Running {
        stop: stop.clone(),
        done,
    });
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        listen(app, session_id, recorder, settings, stop).await;
        let _ = done_tx.send(());
    });
}

/// Stop listening: the last phrase is cut and transcribed, and this resolves
/// once every phrase is in the session.
pub async fn finish() {
    let running = RUNNING.lock().ok().and_then(|mut r| r.take());
    if let Some(Running { stop, done }) = running {
        stop.store(true, Ordering::SeqCst);
        let _ = done.await;
    }
}

async fn listen(
    app: AppHandle,
    session_id: i64,
    mut recorder: AudioRecorder,
    settings: crate::settings::Settings,
    stop: Arc<AtomicBool>,
) {
    let buffer = recorder.buffer();
    let sample_rate = recorder.sample_rate().max(1);
    let frame = (sample_rate * FRAME_MS / 1000) as usize;
    let muted = app.state::<Arc<AtomicBool>>().inner().clone();
    let already_heard = buffer.lock().map(|b| b.len()).unwrap_or(0);
    let mut phrase = Phrase {
        started_ms: heard_since(now_ms(), already_heard, sample_rate),
        ..Default::default()
    };
    let mut pending = Vec::new();

    loop {
        tokio::time::sleep(TICK).await;
        let stopping = stop.load(Ordering::SeqCst);

        // Look at the new frames, and take the audio to cut while holding the lock.
        let (action, audio, level) = {
            let Ok(mut buf) = buffer.lock() else { break };
            let mut level = 0.0f32;
            while phrase.scanned + frame <= buf.len() {
                let r = rms(&buf[phrase.scanned..phrase.scanned + frame]);
                level = level.max(r);
                if r >= SPEECH_RMS {
                    phrase.heard_speech = true;
                    phrase.trailing_silence_ms = 0;
                } else {
                    phrase.trailing_silence_ms += FRAME_MS;
                }
                phrase.scanned += frame;
            }
            let length_ms = (phrase.scanned as u64 * 1000 / sample_rate as u64) as u32;
            let action = if muted.load(Ordering::SeqCst) {
                Action::Drop
            } else if stopping {
                if phrase.heard_speech {
                    Action::Cut
                } else {
                    Action::Drop
                }
            } else {
                decide(phrase.heard_speech, phrase.trailing_silence_ms, length_ms)
            };
            let audio = match action {
                Action::Wait => Vec::new(),
                Action::Cut | Action::Drop => {
                    let end = phrase.scanned.min(buf.len());
                    buf.drain(..end).collect()
                }
            };
            (action, audio, level)
        };
        let _ = app.emit(
            LEVEL_EVENT,
            Level {
                level: (level * 15.0).min(1.0),
            },
        );

        if phrase.heard_speech && !muted.load(Ordering::SeqCst) {
            touch();
        }
        if action != Action::Wait {
            let started_ms = phrase.started_ms;
            phrase = Phrase {
                started_ms: now_ms(),
                ..Default::default()
            };
            if action == Action::Cut {
                let app = app.clone();
                let settings = settings.clone();
                pending.push(tauri::async_runtime::spawn(async move {
                    transcribe_phrase(&app, session_id, &settings, audio, sample_rate, started_ms)
                        .await;
                }));
            }
        }

        if stopping {
            break;
        }
        if now_ms() - LAST_ACTIVITY.load(Ordering::SeqCst) > IDLE_END_MS {
            log_message("[narration] Two minutes with nothing new — ending the session");
            let app = app.clone();
            tauri::async_runtime::spawn(async move { crate::commands::press_session(&app) });
            // Stopping the session finishes narration; nothing is left to hear.
            break;
        }
    }

    recorder.stop();
    for task in pending {
        let _ = task.await;
    }
    let _ = app.emit(LEVEL_EVENT, Level { level: 0.0 });
    log_message(&format!("[narration] Stopped for session {session_id}"));
}

async fn transcribe_phrase(
    app: &AppHandle,
    session_id: i64,
    settings: &crate::settings::Settings,
    audio: Vec<f32>,
    sample_rate: u32,
    started_ms: i64,
) {
    let secs = audio.len() as f64 / sample_rate as f64;
    let wav = match to_wav(&audio, sample_rate) {
        Ok(wav) => wav,
        Err(e) => return log_message(&format!("[narration] WAV encode failed: {e}")),
    };
    let text = match crate::transcribe::transcribe(app, wav, settings, secs).await {
        Ok(text) => text.trim().to_string(),
        Err(e) => return log_message(&format!("[narration] Phrase not transcribed: {e}")),
    };
    if text.is_empty() {
        return;
    }
    let store = app.state::<Arc<Store>>();
    let item = NewItem {
        kind: speaktype_session::ItemKind::Speech,
        t_start_ms: started_ms,
        t_end_ms: Some(started_ms + (secs * 1000.0) as i64),
        text: Some(text.clone()),
        image_path: None,
        source_app: None,
        payload: serde_json::json!({ "narration": true }),
    };
    match store.add_item(session_id, item) {
        Ok(item) => {
            crate::session_cmds::emit_item_added(app, &item);
            let _ = app.emit(CAPTION_EVENT, Caption { text });
        }
        Err(e) => log_message(&format!("[narration] Could not save a phrase: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_already_heard_dates_the_first_phrase_back_to_when_it_began() {
        // 3 s of a dictation's audio at 48 kHz, taken over at t = 10 000 ms.
        assert_eq!(heard_since(10_000, 144_000, 48_000), 7_000);
        assert_eq!(heard_since(10_000, 0, 48_000), 10_000);
    }

    #[test]
    fn a_pause_after_enough_speech_ends_a_phrase() {
        assert_eq!(decide(true, 700, 1_500), Action::Cut);
        assert_eq!(decide(true, 300, 1_500), Action::Wait);
    }

    #[test]
    fn a_short_blip_waits_for_more() {
        assert_eq!(decide(true, 900, 600), Action::Wait);
    }

    #[test]
    fn long_speech_is_cut_without_a_pause_so_captions_keep_up() {
        assert_eq!(decide(true, 0, 25_000), Action::Cut);
    }

    #[test]
    fn silence_is_dropped_not_transcribed() {
        assert_eq!(decide(false, 3_000, 3_000), Action::Drop);
        assert_eq!(decide(false, 1_000, 1_000), Action::Wait);
    }

    #[test]
    fn rms_of_silence_is_zero_and_of_a_tone_is_its_level() {
        assert_eq!(rms(&[0.0; 320]), 0.0);
        let tone: Vec<f32> = (0..320)
            .map(|i| if i % 2 == 0 { 0.1 } else { -0.1 })
            .collect();
        assert!((rms(&tone) - 0.1).abs() < 1e-6);
    }
}
