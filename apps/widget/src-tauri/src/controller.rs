use crate::history::DictationOutcome;
use crate::logging::log_message;
use crate::notice::{Notice, NoticeKind};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// The dictation pill's phases (spec §6.1). `preparing` is the "Getting ready…"
/// state: the hotkey fired while the model server was still loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DictationPhase {
    Idle,
    Preparing,
    Recording,
    Transcribing,
    Done,
    Error,
}

impl DictationPhase {
    pub fn is_recording(self) -> bool {
        self == DictationPhase::Recording
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct DictationState {
    pub phase: DictationPhase,
    pub notice: Option<Notice>,
}

impl DictationState {
    pub fn idle() -> Self {
        Self {
            phase: DictationPhase::Idle,
            notice: None,
        }
    }

    pub fn new(phase: DictationPhase, notice: Option<Notice>) -> Self {
        Self { phase, notice }
    }
}

pub fn idle() -> DictationState {
    DictationState::idle()
}

pub fn recording() -> DictationState {
    DictationState::new(DictationPhase::Recording, None)
}

pub fn transcribing() -> DictationState {
    DictationState::new(DictationPhase::Transcribing, None)
}

pub fn server_error() -> DictationState {
    DictationState::new(
        DictationPhase::Error,
        Some(Notice::of(NoticeKind::ServerError)),
    )
}

pub fn mic_fallback() -> DictationState {
    DictationState::new(
        DictationPhase::Done,
        Some(Notice::of(NoticeKind::MicFallback)),
    )
}

pub fn mic_unavailable() -> DictationState {
    DictationState::new(
        DictationPhase::Error,
        Some(Notice::of(NoticeKind::MicUnavailable)),
    )
}

pub fn busy() -> DictationState {
    DictationState::new(DictationPhase::Error, Some(Notice::of(NoticeKind::Busy)))
}

pub fn no_speech() -> DictationState {
    DictationState::new(
        DictationPhase::Error,
        Some(Notice::of(NoticeKind::NoSpeech)),
    )
}

/// How one finished take left the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteResult {
    Pasted,
    /// Nothing could be pasted; the text sits on the clipboard.
    Copied,
    /// The paste was attempted and did not land.
    Failed,
}

pub fn finish_outcome(paste: PasteResult) -> (DictationOutcome, DictationState) {
    let (outcome, kind) = match paste {
        PasteResult::Pasted => (DictationOutcome::Pasted, NoticeKind::Pasted),
        PasteResult::Copied | PasteResult::Failed => {
            let outcome = match paste {
                PasteResult::Copied => DictationOutcome::Copied,
                _ => DictationOutcome::PasteFailed,
            };
            (outcome, NoticeKind::CopiedNotPasted)
        }
    };
    (
        outcome,
        DictationState::new(DictationPhase::Done, Some(Notice::of(kind))),
    )
}

/// What the controller needs from the app, behind a trait so the state machine
/// runs without Tauri, a microphone or a model server.
pub trait DictationHost {
    /// The state the app currently holds; a press re-reads it, so a background
    /// stop or cancel is never overruled by a stale copy.
    fn state(&self) -> DictationState;
    /// Publish the state (the pill) and arm or release Esc.
    fn set_state(&self, state: DictationState);
    /// Open the mic and keep recording until `stop` or `cancel`.
    fn start_recording(&self) -> Result<(), DictationState>;
    /// Close the mic and hand back `(samples, sample_rate)`.
    fn stop_recording(&self) -> Result<(Vec<f32>, u32), String>;
    /// Discard the take; nothing is transcribed.
    fn cancel_recording(&self);
    /// Start-transcription in the background; it drives the states from here.
    fn transcribe(&self, samples: Vec<f32>, sample_rate: u32);
}

/// The model server's startup timeout (60 s, `server.rs`) plus a margin: a
/// press waits at most this long for "Getting ready…" to end either way.
pub const SERVER_WAIT_MAX: Duration = Duration::from_millis(65_000);

/// Model loading's span, so a press during a load shows "Getting ready…" and
/// records by itself once the server answers. Reads as a pure state machine in
/// the tests and as the real load in the app.
pub trait ServerGate {
    /// True once the embedded server answers. Opening the server is the host's
    /// job (it runs on the load task).
    fn ready(&self) -> bool;
    /// Wait, bounded by the host, until the server is ready (`true`) or its
    /// startup has failed for good (`false`). Never waits forever: a dictation
    /// that cannot start must end in an error the user can retry.
    async fn wait(&self) -> bool;
}

/// One dictation controller per press; `press` needs it `&mut`.
pub struct Dictation<'a, H: DictationHost, G: ServerGate> {
    host: &'a H,
    gate: &'a G,
    state: DictationState,
    /// True from the moment a press starts a take until recording begins.
    /// Owned by the app, shared by every press, so two presses 50 ms apart
    /// cannot start two takes.
    pressing: &'a AtomicBool,
}

impl<'a, H: DictationHost, G: ServerGate> Dictation<'a, H, G> {
    pub fn new(host: &'a H, gate: &'a G, pressing: &'a AtomicBool) -> Self {
        Self {
            host,
            gate,
            state: DictationState::idle(),
            pressing,
        }
    }

    fn set(&mut self, state: DictationState) {
        self.state = state.clone();
        self.host.set_state(state);
    }

    #[cfg(test)]
    fn state(&self) -> &DictationState {
        &self.state
    }

    #[cfg(test)]
    fn is_recording(&self) -> bool {
        self.state.phase.is_recording()
    }

    /// The hotkey (or tray) press. A press while loading is remembered, not
    /// lost: it opens the microphone the moment the server answers.
    pub async fn press(&mut self) {
        self.state = self.host.state();
        match self.state.phase {
            DictationPhase::Preparing => {
                log_message("[dictation] already getting ready");
            }
            DictationPhase::Recording => {
                self.stop();
            }
            DictationPhase::Transcribing => {
                log_message("[dictation] still working on the last one");
                self.set(busy());
            }
            DictationPhase::Idle | DictationPhase::Done | DictationPhase::Error => {
                if self.pressing.swap(true, Ordering::SeqCst) {
                    log_message("[dictation] a start is already in flight");
                    return;
                }
                if self.gate.ready() {
                    self.pressing.store(false, Ordering::SeqCst);
                    self.start();
                } else {
                    self.set(DictationState::new(DictationPhase::Preparing, None));
                    let ready = self.gate.wait().await;
                    self.pressing.store(false, Ordering::SeqCst);
                    if ready {
                        self.start();
                    } else {
                        log_message("[dictation] Speech model did not start");
                        self.set(server_error());
                    }
                }
            }
        }
    }

    fn start(&mut self) {
        match self.host.start_recording() {
            Ok(()) => self.set(recording()),
            Err(error) => {
                log_message(&format!("[dictation] could not start: {:?}", error.notice));
                self.set(error);
            }
        }
    }

    fn stop(&mut self) {
        match self.host.stop_recording() {
            Ok((samples, sample_rate)) => {
                self.set(transcribing());
                self.host.transcribe(samples, sample_rate);
            }
            Err(e) => {
                log_message(&format!("[dictation] could not stop the recorder: {}", e));
                self.set(mic_unavailable());
            }
        }
    }

    /// Esc while recording: discard the take and return to idle.
    pub fn cancel(&mut self) {
        self.state = self.host.state();
        if !self.state.phase.is_recording() {
            return;
        }
        self.host.cancel_recording();
        self.set(DictationState::idle());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Clone, Default)]
    struct FakeHost {
        states: Arc<Mutex<Vec<DictationState>>>,
        open: Arc<Mutex<bool>>,
        cancelled: Arc<Mutex<u32>>,
        transcribed: Arc<Mutex<Vec<usize>>>,
    }

    impl DictationHost for FakeHost {
        fn state(&self) -> DictationState {
            self.states
                .lock()
                .unwrap()
                .last()
                .cloned()
                .map(|s| match s.phase {
                    // A finished take takes a moment to clear on the pill; a
                    // press during it starts the next one.
                    DictationPhase::Done => DictationState::idle(),
                    _ => s,
                })
                .unwrap_or_else(DictationState::idle)
        }
        fn set_state(&self, state: DictationState) {
            self.states.lock().unwrap().push(state);
        }
        fn start_recording(&self) -> Result<(), DictationState> {
            *self.open.lock().unwrap() = true;
            Ok(())
        }
        fn stop_recording(&self) -> Result<(Vec<f32>, u32), String> {
            *self.open.lock().unwrap() = false;
            Ok((vec![0.0; 1600], 16_000))
        }
        fn cancel_recording(&self) {
            *self.open.lock().unwrap() = false;
            *self.cancelled.lock().unwrap() += 1;
        }
        fn transcribe(&self, samples: Vec<f32>, _sample_rate: u32) {
            self.transcribed.lock().unwrap().push(samples.len());
        }
    }

    impl FakeHost {
        fn phases(&self) -> Vec<DictationPhase> {
            self.states
                .lock()
                .unwrap()
                .iter()
                .map(|s| s.phase)
                .collect()
        }
    }

    /// Each test gets its own latch, so parallel tests cannot see each
    /// other's in-flight press.
    fn pressing() -> Box<AtomicBool> {
        Box::new(AtomicBool::new(false))
    }

    #[derive(Clone, Copy)]
    struct Ready;

    impl ServerGate for Ready {
        fn ready(&self) -> bool {
            true
        }
        async fn wait(&self) -> bool {
            true
        }
    }

    /// A server that only answers after `load()` — the "hotkey while the model
    /// loads" review focus.
    #[derive(Clone, Default)]
    struct Loading {
        ready: Arc<Mutex<bool>>,
    }

    impl ServerGate for Loading {
        fn ready(&self) -> bool {
            *self.ready.lock().unwrap()
        }
        async fn wait(&self) -> bool {
            while !*self.ready.lock().unwrap() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            true
        }
    }

    #[tokio::test]
    async fn start_records_and_stop_transcribes() {
        let host = FakeHost::default();
        let latch = pressing();
        let mut d = Dictation::new(&host, &Ready, &latch);
        d.press().await;
        assert!(d.is_recording());
        d.press().await;
        assert_eq!(d.state().phase, DictationPhase::Transcribing);
        assert_eq!(*host.transcribed.lock().unwrap(), vec![1600]);
        assert_eq!(
            host.phases(),
            vec![DictationPhase::Recording, DictationPhase::Transcribing]
        );
    }

    #[tokio::test]
    async fn dictation_emits_preparing_then_recording_when_server_loading() {
        let host = FakeHost::default();
        let gate = Loading::default();
        let loader = gate.clone();
        let loading = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            *loader.ready.lock().unwrap() = true;
        });
        let latch = pressing();
        let mut d = Dictation::new(&host, &gate, &latch);

        d.press().await;
        loading.await.unwrap();

        assert_eq!(
            host.phases(),
            vec![DictationPhase::Preparing, DictationPhase::Recording],
            "the press is not lost while the model loads"
        );
        assert!(*host.open.lock().unwrap(), "recording starts by itself");

        d.press().await;
        assert_eq!(d.state().phase, DictationPhase::Transcribing);
    }

    /// A server that never comes up, giving up after `deadline` — the bound
    /// the real host applies (65 s, or earlier when the load task fails).
    #[derive(Clone, Copy)]
    struct NeverReady {
        deadline: Duration,
    }

    impl ServerGate for NeverReady {
        fn ready(&self) -> bool {
            false
        }
        async fn wait(&self) -> bool {
            tokio::time::sleep(self.deadline).await;
            false
        }
    }

    #[tokio::test(start_paused = true)]
    async fn dictation_reports_server_error_when_server_never_starts() {
        let host = FakeHost::default();
        let gate = NeverReady {
            deadline: SERVER_WAIT_MAX,
        };
        let latch = pressing();
        let mut d = Dictation::new(&host, &gate, &latch);

        d.press().await;

        assert_eq!(
            host.phases(),
            vec![DictationPhase::Preparing, DictationPhase::Error],
            "a server that never starts ends in the error state, not a stuck pill"
        );
        let error = host.states.lock().unwrap().last().cloned().unwrap();
        assert_eq!(
            error.notice.as_ref().map(|n| n.message.as_str()),
            Some("Speech model couldn't start")
        );
        assert!(!*host.open.lock().unwrap(), "no recording starts");

        // The latch was reset and the pill is in `error`, so the next press is
        // handled as a fresh attempt (it shows `preparing` again, then times
        // out again) instead of being ignored as "already getting ready".
        d.press().await;
        assert_eq!(
            &host.phases()[2..],
            &[DictationPhase::Preparing, DictationPhase::Error],
            "a later press starts a fresh attempt"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_load_ends_the_wait_early() {
        let host = FakeHost::default();
        let gate = NeverReady {
            deadline: Duration::from_millis(500),
        };
        let latch = pressing();
        let mut d = Dictation::new(&host, &gate, &latch);

        d.press().await;

        assert_eq!(
            host.phases(),
            vec![DictationPhase::Preparing, DictationPhase::Error],
            "a load that reports failure does not wait out the full bound"
        );
        assert!(!*host.open.lock().unwrap());
    }

    #[tokio::test]
    async fn esc_cancels_a_recording_without_transcribing() {
        let host = FakeHost::default();
        let latch = pressing();
        let mut d = Dictation::new(&host, &Ready, &latch);
        d.press().await;
        d.cancel();
        assert_eq!(*host.cancelled.lock().unwrap(), 1);
        assert!(host.transcribed.lock().unwrap().is_empty());
        assert_eq!(d.state().phase, DictationPhase::Idle);
        assert!(host.states.lock().unwrap().last().unwrap().notice.is_none());
    }

    #[tokio::test]
    async fn esc_outside_recording_does_nothing() {
        let host = FakeHost::default();
        let latch = pressing();
        let mut d = Dictation::new(&host, &Ready, &latch);
        d.cancel();
        assert_eq!(*host.cancelled.lock().unwrap(), 0);
        assert!(host.states.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_start_failure_shows_its_notice_instead_of_recording() {
        struct Deaf;
        impl DictationHost for Deaf {
            fn state(&self) -> DictationState {
                DictationState::idle()
            }
            fn set_state(&self, _state: DictationState) {}
            fn start_recording(&self) -> Result<(), DictationState> {
                Err(mic_unavailable())
            }
            fn stop_recording(&self) -> Result<(Vec<f32>, u32), String> {
                unreachable!()
            }
            fn cancel_recording(&self) {}
            fn transcribe(&self, _samples: Vec<f32>, _sample_rate: u32) {}
        }
        let latch = pressing();
        let mut d = Dictation::new(&Deaf, &Ready, &latch);
        d.press().await;
        assert_eq!(d.state().phase, DictationPhase::Error);
        assert_eq!(
            d.state().notice.as_ref().map(|n| n.message.as_str()),
            Some("Microphone unavailable")
        );
    }

    #[test]
    fn paste_outcomes_map_to_history_outcomes_and_notices() {
        let (outcome, state) = finish_outcome(PasteResult::Pasted);
        assert_eq!(outcome, DictationOutcome::Pasted);
        assert_eq!(state.phase, DictationPhase::Done);

        for paste in [PasteResult::Copied, PasteResult::Failed] {
            let (outcome, state) = finish_outcome(paste);
            assert_eq!(
                outcome,
                if paste == PasteResult::Copied {
                    DictationOutcome::Copied
                } else {
                    DictationOutcome::PasteFailed
                }
            );
            assert_eq!(state.phase, DictationPhase::Done);
            assert_eq!(
                state.notice.as_ref().map(|n| n.message.as_str()),
                Some("Copied. Press ⌘V to paste")
            );
        }
    }
}
