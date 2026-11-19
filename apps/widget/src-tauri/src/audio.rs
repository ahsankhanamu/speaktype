use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::logging::log_message;

/// Wraps `cpal::Stream` which is !Send on some backends (Linux ALSA).
/// On macOS (CoreAudio) dropping from any thread is safe. The `pause()`
/// call in `stop()` ensures the audio callback has stopped before we drop.
struct SendStream(cpal::Stream);
unsafe impl Send for SendStream {}

pub struct AudioRecorder {
    stream: Option<SendStream>,
    fallback: Option<FallbackMic>,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    builtin_fallback: bool,
}

impl AudioRecorder {
    pub fn buffer(&self) -> Arc<Mutex<Vec<f32>>> {
        self.buffer.clone()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// True when a stream error mid-take switched capture to the built-in
    /// microphone; the dictation controller reports it as `micFallback`.
    pub fn builtin_fallback(&self) -> bool {
        self.builtin_fallback
    }

    pub fn new() -> Result<Self, String> {
        // Defer device probing until start() so launch succeeds without a mic
        // (e.g. Mac mini with no input device). Onboarding covers the no-mic case.
        const DEFAULT_SAMPLE_RATE: u32 = 16_000;
        log_message(&format!(
            "[audio] Recorder ready (sample_rate default {}Hz; device opens on start)",
            DEFAULT_SAMPLE_RATE
        ));

        Ok(Self {
            stream: None,
            fallback: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: DEFAULT_SAMPLE_RATE,
            builtin_fallback: false,
        })
    }

    /// Start capturing using a specific named input device, or the system
    /// default when `selected` is None/empty.
    pub(crate) fn start_with_input(
        &mut self,
        app_handle: AppHandle,
        selected: Option<&str>,
    ) -> Result<(), String> {
        let host = cpal::default_host();
        let device = resolve_input_device(&host, selected)?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        self.sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        log_message(&format!(
            "[audio] Device: {:?}, sample_rate: {}, channels: {}",
            device.name().unwrap_or_default(),
            self.sample_rate,
            channels
        ));

        {
            let mut buf = self.buffer.lock().unwrap();
            buf.clear();
        }

        let buffer = self.buffer.clone();
        let app = app_handle.clone();
        let sample_rate = self.sample_rate;
        self.builtin_fallback = false;
        let fallback = FallbackMic {
            buffer: buffer.clone(),
            stream: Arc::new(Mutex::new(None)),
            used: Arc::new(AtomicBool::new(false)),
        };
        let on_error_fallback = fallback.clone();

        // Pre-allocate per-frame downmix buffer (max 4096 frames × mono)
        let mut local_downmix = Vec::with_capacity(4096);

        let last_level_time = Arc::new(Mutex::new(std::time::Instant::now()));

        let stream_config: cpal::StreamConfig = config.into();

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    local_downmix.clear();
                    if channels == 1 {
                        local_downmix.extend_from_slice(data);
                    } else {
                        local_downmix.extend(
                            data.chunks(channels)
                                .map(|frame| frame.iter().sum::<f32>() / channels as f32),
                        );
                    };

                    if local_downmix.is_empty() {
                        return;
                    }

                    if let Ok(mut buf) = buffer.try_lock() {
                        buf.extend_from_slice(&local_downmix);
                    }

                    let now = std::time::Instant::now();
                    if let Ok(mut last) = last_level_time.try_lock() {
                        if now.duration_since(*last).as_millis() >= 33 {
                            *last = now;
                            let mut sum = 0.0f32;
                            let mut peak = 0.0f32;
                            for s in local_downmix.iter() {
                                sum += s * s;
                                let mag = s.abs();
                                if mag > peak {
                                    peak = mag;
                                }
                            }
                            let rms = (sum / local_downmix.len() as f32).sqrt();
                            let level = (rms * 10000.0).round() / 10000.0;
                            let peak = (peak * 10000.0).round() / 10000.0;
                            let app_clone = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = app_clone.emit(
                                    "sidecar:audio_level",
                                    serde_json::json!({"level": level, "peak": peak}),
                                );
                            });
                        }
                    }
                },
                move |err| {
                    log_message(&format!("[audio] Stream error: {}", err));
                    on_error_fallback.on_error(&format!("{}", err));
                },
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        self.stream = Some(SendStream(stream));
        self.fallback = Some(fallback);
        log_message(&format!(
            "[audio] Recording started at {}Hz, {} channels",
            sample_rate, channels
        ));

        Ok(())
    }

    pub fn stop(&mut self) -> (Vec<f32>, u32) {
        let (samples, sample_rate, switched) = self.take_samples();
        self.builtin_fallback = switched;

        let duration_ms = if sample_rate > 0 {
            (samples.len() as f64 / sample_rate as f64 * 1000.0) as u64
        } else {
            0
        };

        log_message(&format!(
            "[audio] Recording stopped: {} samples, {}ms",
            samples.len(),
            duration_ms
        ));

        (samples, sample_rate)
    }

    /// Drain the take: the chosen stream, plus anything the built-in fallback
    /// captured after the chosen device dropped out. The bool reports the
    /// switch, once per take.
    fn take_samples(&mut self) -> (Vec<f32>, u32, bool) {
        if let Some(ref stream) = self.stream {
            let _ = stream.0.pause();
        }
        self.stream.take();

        let switched = match self.fallback.take() {
            Some(fallback) => {
                if let Some(stream) = fallback.take_stream() {
                    let _ = stream.0.pause();
                }
                fallback.switched()
            }
            None => false,
        };
        let samples = {
            let mut buf = self.buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };
        (samples, self.sample_rate, switched)
    }
}

/// Keeps recording on the built-in microphone when the chosen input errors
/// mid-take (unplugged, Bluetooth dropped). Its samples append to the same
/// buffer, so the take continues instead of ending on a silent gap. Picking
/// the device and building the stream are separate: only the first belongs in
/// an audio callback.
#[derive(Clone)]
struct FallbackMic {
    buffer: Arc<Mutex<Vec<f32>>>,
    stream: Arc<Mutex<Option<SendStream>>>,
    used: Arc<AtomicBool>,
}

impl FallbackMic {
    fn on_error(&self, error: &str) {
        if self.used.swap(true, Ordering::SeqCst) {
            return;
        }
        log_message(&format!(
            "[audio] Chosen input failed ({}), switching to the built-in microphone",
            error
        ));
        let built = cpal::default_host()
            .default_input_device()
            .ok_or_else(|| "no default input device".to_string())
            .and_then(|device| {
                let config = device
                    .default_input_config()
                    .map_err(|e| format!("no input config: {}", e))?;
                mix_into_buffer(self.buffer.clone(), device, config)
            });
        match built {
            Ok(stream) => {
                if let Ok(mut slot) = self.stream.lock() {
                    *slot = Some(stream);
                }
            }
            Err(e) => log_message(&format!("[audio] Built-in fallback did not start: {}", e)),
        }
    }

    fn take_stream(&self) -> Option<SendStream> {
        self.stream.lock().ok().and_then(|mut slot| slot.take())
    }

    fn switched(&self) -> bool {
        self.used.load(Ordering::SeqCst)
    }
}

/// Downmix one device into `buffer` as it plays; the only audio-callback work
/// a fallback does.
fn mix_into_buffer(
    buffer: Arc<Mutex<Vec<f32>>>,
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
) -> Result<SendStream, String> {
    let channels = config.channels().max(1) as usize;
    let stream_config: cpal::StreamConfig = config.into();
    let stream = device
        .build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if let Ok(mut buf) = buffer.try_lock() {
                    buf.extend(
                        data.chunks(channels)
                            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32),
                    );
                }
            },
            |err| log_message(&format!("[audio] Fallback stream error: {}", err)),
            None,
        )
        .map_err(|e| format!("failed to build input stream: {}", e))?;
    stream
        .play()
        .map_err(|e| format!("failed to start stream: {}", e))?;
    Ok(SendStream(stream))
}

/// Metering interval for the level monitor. Matches the recorder's cadence so
/// the settings meter and the recording waveform behave identically.
const MONITOR_EMIT_MS: u128 = 33;
/// A sample this close to full scale is clipped for practical purposes: the
/// converter has already lost the peak even if it never reaches exactly 1.0.
const MONITOR_CLIP_LEVEL: f32 = 0.98;

static MONITOR_GENERATION: AtomicU64 = AtomicU64::new(0);

pub struct MonitorInfo {
    pub device: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub generation: u64,
}

/// Metering-only capture for the settings microphone check. Opens its own
/// input stream and emits `mictest:level`; it never writes to the recorder
/// buffer, so verifying the microphone cannot produce a transcription or
/// disturb an in-flight dictation.
pub struct LevelMonitor {
    stream: Option<SendStream>,
    generation: u64,
}

impl LevelMonitor {
    pub fn new() -> Self {
        Self {
            stream: None,
            generation: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.stream.is_some()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Capture levels from a specific named input device, or the system default
    /// when `selected` is None/empty.
    pub(crate) fn start_with_input(
        &mut self,
        app_handle: AppHandle,
        selected: Option<&str>,
    ) -> Result<MonitorInfo, String> {
        self.stop();

        let host = cpal::default_host();
        let device = resolve_input_device(&host, selected)?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let device_name = device
            .name()
            .unwrap_or_else(|_| "Unknown input".to_string());

        let app = app_handle.clone();
        let stream = open_level_stream(device, config, move |rms, peak, clipped| {
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = app_clone.emit(
                    "mictest:level",
                    serde_json::json!({
                        "rms": (rms * 10000.0).round() / 10000.0,
                        "peak": (peak * 10000.0).round() / 10000.0,
                        "clipped": clipped,
                    }),
                );
            });
        })?;

        self.stream = Some(stream);
        self.generation = MONITOR_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

        log_message(&format!(
            "[mictest] Monitoring {:?} at {}Hz, {} channels (gen {})",
            device_name, sample_rate, channels, self.generation
        ));

        Ok(MonitorInfo {
            device: device_name,
            sample_rate,
            channels,
            generation: self.generation,
        })
    }

    /// Release the input stream. Returns whether a stream was actually open, so
    /// callers can avoid emitting a stop event for a monitor that never ran.
    pub fn stop(&mut self) -> bool {
        let Some(stream) = self.stream.take() else {
            return false;
        };
        let _ = stream.0.pause();
        drop(stream);
        log_message(&format!(
            "[mictest] Monitoring stopped (gen {})",
            self.generation
        ));
        true
    }
}

impl Drop for LevelMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A metering-only input stream: it never touches a recorder buffer, and it
/// calls `report` with the round's RMS, peak and clip flag every
/// `MONITOR_EMIT_MS`. Shared by the settings mic test and the mic picker.
fn open_level_stream(
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    report: impl FnMut(f32, f32, bool) + Send + 'static,
) -> Result<SendStream, String> {
    let frame_channels = config.channels().max(1) as usize;
    let stream_config: cpal::StreamConfig = config.into();
    let mut report = report;

    // Energy is accumulated across callbacks rather than measured on the
    // most recent one: a callback can be shorter than the emit interval, so
    // sampling only the newest block would miss transients between emits.
    let mut sum_sq = 0f64;
    let mut frames = 0usize;
    let mut peak = 0f32;
    let mut clipped = false;
    let mut last_emit = Instant::now();

    let stream = device
        .build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for frame in data.chunks(frame_channels) {
                    let mut mono = 0f32;
                    for &s in frame {
                        mono += s;
                        if s.abs() >= MONITOR_CLIP_LEVEL {
                            clipped = true;
                        }
                    }
                    mono /= frame.len() as f32;
                    sum_sq += (mono * mono) as f64;
                    frames += 1;
                    let mag = mono.abs();
                    if mag > peak {
                        peak = mag;
                    }
                }

                let now = Instant::now();
                if frames == 0 || now.duration_since(last_emit).as_millis() < MONITOR_EMIT_MS {
                    return;
                }
                last_emit = now;

                report(
                    (sum_sq / frames as f64).sqrt() as f32,
                    peak,
                    std::mem::take(&mut clipped),
                );
                sum_sq = 0.0;
                frames = 0;
                peak = 0.0;
            },
            move |err| {
                log_message(&format!("[mictest] Stream error: {}", err));
            },
            None,
        )
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Failed to start stream: {}", e))?;
    Ok(SendStream(stream))
}

/// How often the microphone picker receives its per-device levels.
pub const MIC_LEVELS_EMIT_MS: u64 = 50;
/// How long one `mic_levels_start` round may keep input streams open. The
/// picker re-calls `start` every 20 s while it is open, which restarts this.
pub const MIC_LEVELS_MAX: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, serde::Serialize, specta::Type)]
pub struct DeviceLevel {
    /// The cpal device name, the picker's unique key.
    pub device: String,
    /// Smoothed 0..1 RMS.
    pub level: f32,
}

/// A new reading moves a bar by at most half the distance, so the meter eases
/// instead of flickering between callbacks. A decayed level snaps to exactly 0
/// rather than creeping asymptotically at the bar's edge.
pub fn smooth_levels(previous: &[f32], raw: &[f32]) -> Vec<f32> {
    const SMOOTHING: f32 = 0.5;
    const REST: f32 = 0.002;
    raw.iter()
        .enumerate()
        .map(|(i, &reading)| {
            let reading = reading.clamp(0.0, 1.0);
            let smoothed = match previous.get(i) {
                Some(&prev) => prev * (1.0 - SMOOTHING) + reading * SMOOTHING,
                None => reading,
            };
            if reading == 0.0 && smoothed < REST {
                0.0
            } else {
                smoothed
            }
        })
        .collect()
}

/// The outside world of a levels round: the emitter, the clock the round times
/// itself against and the release of its streams. A trait so the 30 s auto-stop
/// and the keep-alive rule run without real audio hardware.
pub trait MicLevelsUi: Clone + Send + 'static {
    fn emit_levels(&self, levels: &[DeviceLevel]);
    fn emit_error(&self, message: &str);
    /// Release the round's streams.
    fn stop(&self);
    /// A monotonic clock on the same time base the round sleeps on.
    fn now(&self) -> Duration;
    /// Sleep, without holding the caller's future across a real timer so a
    /// non-Send clock (the app's) still fits `spawn`.
    fn sleep(
        &self,
        duration: Duration,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
}

/// One metering stream per input device, live only while the mic picker is
/// open. `start` replaces the round, and a round closes itself 30 s after it
/// began, so no stream outlives the picker.
pub struct DeviceLevels<U: MicLevelsUi> {
    ui: U,
    streams: Vec<SendStream>,
}

impl<U: MicLevelsUi> DeviceLevels<U> {
    pub fn new(ui: U) -> Self {
        Self {
            ui,
            streams: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.stop();
        self.ui.emit_levels(&[]);

        let levels = Arc::new(Mutex::new(Vec::<DeviceLevel>::new()));
        for device in cpal::default_host().input_devices().into_iter().flatten() {
            let Ok(name) = device.name() else {
                continue;
            };
            let Ok(config) = device.default_input_config() else {
                continue;
            };
            let key = name.clone();
            let cell = levels.clone();
            match open_level_stream(device, config, move |rms, _, _| {
                if let Ok(mut levels) = cell.try_lock() {
                    match levels.iter_mut().find(|l| l.device == key) {
                        Some(entry) => entry.level = smooth_levels(&[entry.level], &[rms])[0],
                        None => levels.push(DeviceLevel {
                            device: key.clone(),
                            level: smooth_levels(&[], &[rms])[0],
                        }),
                    }
                }
            }) {
                Ok(stream) => self.streams.push(stream),
                // A device can vanish between listing and opening; the picker
                // shows it as "Not connected", so the round carries on.
                Err(e) => log_message(&format!("[miclevels] {}: {}", name, e)),
            }
        }

        if self.streams.is_empty() {
            self.ui.emit_error("Microphone unavailable");
            return;
        }
        self.run(levels);
        log_message("[miclevels] Metering every input device");
    }

    fn run(&mut self, levels: Arc<Mutex<Vec<DeviceLevel>>>) {
        spawn_round(self.ui.clone(), levels);
    }

    /// Idempotent: closing a picker, hiding a window and the round's own
    /// deadline can all arrive for the same round.
    pub fn stop(&mut self) {
        if self.streams.is_empty() {
            return;
        }
        for stream in self.streams.drain(..) {
            let _ = stream.0.pause();
            drop(stream);
        }
        self.ui.stop();
    }
}

impl<U: MicLevelsUi> Drop for DeviceLevels<U> {
    fn drop(&mut self) {
        self.stop();
    }
}

/// One round's tick loop. `sleep` is the keep-alive's single wake-up: while
/// the mic picker stays open it re-calls `start` every 20 s, which bumps the
/// clock this loop measures itself against, so only the live round reaches
/// its 30 s deadline.
fn spawn_round<U: MicLevelsUi>(ui: U, levels: Arc<Mutex<Vec<DeviceLevel>>>) {
    tauri::async_runtime::spawn(async move {
        let mut elapsed = Duration::ZERO;
        loop {
            let before = ui.now();
            ui.sleep(Duration::from_millis(MIC_LEVELS_EMIT_MS)).await;
            elapsed += ui.now().saturating_sub(before);
            if elapsed >= MIC_LEVELS_MAX {
                log_message("[miclevels] 30 s round over — releasing every input");
                ui.stop();
                return;
            }
            let mut round = levels.lock().map(|l| l.clone()).unwrap_or_default();
            round.sort_by(|a, b| a.device.cmp(&b.device));
            ui.emit_levels(&round);
        }
    });
}

pub fn has_speech(audio: &[f32], sample_rate: u32) -> bool {
    let threshold: f32 = 0.01;
    let segment_ms: u32 = 50;
    let segment_samples = (sample_rate * segment_ms / 1000) as usize;
    let mut segments_above_threshold = 0u32;
    let mut total_valid_segments = 0u32;

    for chunk in audio.chunks(segment_samples) {
        if chunk.len() < segment_samples / 2 {
            continue;
        }
        total_valid_segments += 1;
        let sum: f32 = chunk.iter().map(|s| s * s).sum();
        let energy = (sum / chunk.len() as f32).sqrt();
        if energy > threshold {
            segments_above_threshold += 1;
            if segments_above_threshold >= 3 {
                return true;
            }
        }
    }

    if total_valid_segments > 0 && total_valid_segments < 10 && segments_above_threshold > 0 {
        return true;
    }

    false
}

fn segment_rms(chunk: &[f32]) -> f32 {
    if chunk.is_empty() {
        return 0.0;
    }
    let sum: f32 = chunk.iter().map(|s| s * s).sum();
    (sum / chunk.len() as f32).sqrt()
}

const SEGMENT_MS: u32 = 50;
const SPEECH_THRESHOLD: f32 = 0.01;
const MIN_CHUNK_MS: u32 = 2000;
/// Preferred split: a comfortable natural pause.
const IDEAL_SILENCE_MS: u32 = 500;
/// Shortest silence we'll accept when someone has been talking a long time without pausing.
const MIN_SILENCE_MS: u32 = 120;
/// Continuous speech shorter than this always uses the ideal (500ms) silence threshold.
const SILENCE_RAMP_START_SECS: f64 = 3.0;
/// By this much continuous speech, the required silence has eased down to `MIN_SILENCE_MS`.
const SILENCE_RAMP_FULL_SECS: f64 = 50.0;

/// How much silence is required to split, given how long someone has been speaking
/// without a break. Starts at 500ms (natural pause), eases down logarithmically
/// toward 120ms during long uninterrupted speech.
pub fn adaptive_silence_ms(continuous_speech_secs: f64) -> u32 {
    if continuous_speech_secs <= SILENCE_RAMP_START_SECS {
        return IDEAL_SILENCE_MS;
    }

    let span = SILENCE_RAMP_FULL_SECS - SILENCE_RAMP_START_SECS;
    if span <= 0.0 {
        return MIN_SILENCE_MS;
    }

    let progress = ((continuous_speech_secs - SILENCE_RAMP_START_SECS) / span).clamp(0.0, 1.0);
    // Logarithmic ease: slow at first, reaches floor near the end of the ramp.
    let curve = 1.0 - (1.0 + 9.0 * progress).ln() / 10.0_f64.ln();
    let ideal = IDEAL_SILENCE_MS as f64;
    let floor = MIN_SILENCE_MS as f64;
    (ideal - curve * (ideal - floor)).round() as u32
}

/// Find a sample index at which to flush a phrase during live recording.
/// Returns `None` if the audio should keep accumulating.
/// Splits on silence only — threshold adapts: natural pauses early, shorter
/// pauses accepted the longer someone talks without stopping.
pub fn find_phrase_flush_point(audio: &[f32], sample_rate: u32) -> Option<usize> {
    let segment_samples = (sample_rate * SEGMENT_MS / 1000).max(1) as usize;
    let min_chunk_samples = (sample_rate * MIN_CHUNK_MS / 1000) as usize;

    if audio.len() < min_chunk_samples {
        return None;
    }

    let mut silence_run_segments = 0usize;
    let mut speech_samples_in_chunk = 0usize;
    let mut had_speech = false;

    let mut offset = 0usize;
    while offset < audio.len() {
        let end = (offset + segment_samples).min(audio.len());
        if end <= offset {
            break;
        }
        let energy = segment_rms(&audio[offset..end]);
        let segment_len = end - offset;

        if energy > SPEECH_THRESHOLD {
            had_speech = true;
            silence_run_segments = 0;
            speech_samples_in_chunk += segment_len;
        } else if had_speech && speech_samples_in_chunk >= min_chunk_samples {
            silence_run_segments += 1;
            let speech_secs = speech_samples_in_chunk as f64 / sample_rate as f64;
            let required_ms = adaptive_silence_ms(speech_secs);
            let required_segments = (required_ms / SEGMENT_MS).max(1) as usize;

            if silence_run_segments >= required_segments {
                let split = offset.saturating_sub(silence_run_segments * segment_samples);
                let split = split.max(min_chunk_samples);
                crate::logging::log_message(&format!(
                    "[chunk] Split at {:.1}s speech (silence threshold {}ms)",
                    speech_secs, required_ms
                ));
                return Some(split);
            }
        }
        offset += segment_samples;
    }

    None
}

/// Split completed audio into phrase chunks on silence boundaries.
/// Split audio on phrase boundaries, returning each chunk together with its
/// start offset so callers can map a chunk back into the full recording.
pub fn split_on_phrase_boundaries(audio: &[f32], sample_rate: u32) -> Vec<(usize, Vec<f32>)> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < audio.len() {
        let slice = &audio[start..];
        if let Some(rel_end) = find_phrase_flush_point(slice, sample_rate) {
            let end = start + rel_end;
            if end > start {
                chunks.push((start, audio[start..end].to_vec()));
            }
            start = end;
        } else {
            chunks.push((start, audio[start..].to_vec()));
            break;
        }
    }
    chunks.retain(|(_, c)| has_speech(c, sample_rate));
    chunks
}

pub fn normalize_audio(audio: &[f32]) -> Vec<f32> {
    let peak = audio.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
    if peak > 0.0 && peak < 1.0 {
        let gain = (1.0 / peak).min(5.0);
        audio.iter().map(|&s| (s * gain).clamp(-1.0, 1.0)).collect()
    } else {
        audio.to_vec()
    }
}

pub fn get_default_input_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_input_device().and_then(|d| d.name().ok())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InputDeviceInfo {
    /// Raw cpal device name, used as the unique key for selection.
    pub name: String,
    /// Human-friendly label with a transport/sample-rate hint.
    pub label: String,
    pub is_default: bool,
}

/// Enumerate connected input devices, labeling each with a transport/sample-rate
/// hint so a user can tell a Bluetooth hands-free mic (16 kHz) from the built-in
/// (48 kHz). Falls back to an empty list on any error.
pub fn list_input_devices() -> Vec<InputDeviceInfo> {
    let host = cpal::default_host();
    let default_name = get_default_input_device_name();

    let Ok(devices) = host.input_devices() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for device in devices {
        let Ok(name) = device.name() else {
            continue;
        };
        let rate = device
            .default_input_config()
            .ok()
            .map(|c| c.sample_rate().0);
        let label = match rate {
            Some(16000) => format!("{} (Bluetooth hands-free, 16 kHz)", name),
            Some(r) => format!("{} ({} kHz)", name, r / 1000),
            None => name.clone(),
        };
        out.push(InputDeviceInfo {
            is_default: default_name.as_deref() == Some(&*name),
            name,
            label,
        });
    }
    out
}

/// Resolve the device to open for capture. `selected` is a stored name (empty =
/// macOS default). If the selected device no longer exists, falls back to default.
fn resolve_input_device(host: &cpal::Host, selected: Option<&str>) -> Result<cpal::Device, String> {
    if let Some(selected) = selected {
        if !selected.is_empty() {
            if let Ok(devices) = host.input_devices() {
                for device in devices {
                    if let Ok(name) = device.name() {
                        if name == selected {
                            return Ok(device);
                        }
                    }
                }
            }
        }
    }
    host.default_input_device()
        .ok_or_else(|| "No input device available".to_string())
}

pub fn count_input_devices() -> usize {
    let host = cpal::default_host();
    host.input_devices().map(|d| d.count()).unwrap_or(0)
}

pub fn to_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, String> {
    let max_wav_size = 100 * 1024 * 1024;
    let num_samples = samples.len();
    let expected_wav_size = 44 + (num_samples * 2);

    if expected_wav_size > max_wav_size {
        return Err(format!(
            "Audio too long for single WAV ({} MB max). Duration: {:.1}s",
            max_wav_size / 1024 / 1024,
            num_samples as f64 / sample_rate as f64
        ));
    }

    let data_size = (num_samples * 2) as u32;
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(44 + data_size as usize);

    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());

    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());

    let mut i16_buf = Vec::with_capacity(samples.len());
    i16_buf.extend(
        samples
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16),
    );
    for &s in &i16_buf {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    Ok(buf)
}

/// Decode a mono 16-bit PCM WAV file produced by `to_wav`.
pub fn from_wav(data: &[u8]) -> Result<(Vec<f32>, u32), String> {
    if data.len() < 44 {
        return Err("WAV file too short".into());
    }
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err("Invalid WAV file".into());
    }

    let audio_format = u16::from_le_bytes(data[20..22].try_into().map_err(|_| "Invalid WAV")?);
    if audio_format != 1 {
        return Err("Only PCM WAV files are supported".into());
    }

    let sample_rate = u32::from_le_bytes(data[24..28].try_into().map_err(|_| "Invalid WAV")?);
    let bits_per_sample = u16::from_le_bytes(data[34..36].try_into().map_err(|_| "Invalid WAV")?);
    if bits_per_sample != 16 {
        return Err("Only 16-bit WAV files are supported".into());
    }

    let data_offset = data
        .windows(4)
        .position(|w| w == b"data")
        .ok_or_else(|| "WAV data chunk not found".to_string())?
        + 4;
    if data.len() < data_offset + 4 {
        return Err("WAV data chunk truncated".into());
    }
    let data_size = u32::from_le_bytes(
        data[data_offset..data_offset + 4]
            .try_into()
            .map_err(|_| "Invalid WAV data size")?,
    ) as usize;
    let pcm_start = data_offset + 4;
    let pcm_end = pcm_start.saturating_add(data_size).min(data.len());
    let pcm = &data[pcm_start..pcm_end];

    let samples: Vec<f32> = pcm
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32767.0)
        .collect();

    Ok((samples, sample_rate))
}

#[cfg(test)]
mod monitor_tests {
    use super::{
        smooth_levels, DeviceLevel, LevelMonitor, MicLevelsUi, MIC_LEVELS_EMIT_MS, MIC_LEVELS_MAX,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn idle_monitor_reports_nothing_to_stop() {
        let mut monitor = LevelMonitor::new();
        assert!(!monitor.is_active());
        assert!(
            !monitor.stop(),
            "a monitor that never ran must not report a stop"
        );
    }

    /// A stand-in for the emitter + round loop: `open` counts live streams,
    /// `stopped` records their release, and `clock` advances only through
    /// `sleep`, so the paused test time drives the 30 s deadline exactly.
    #[derive(Clone, Default)]
    struct Round {
        /// Streams this round's `start` would open, so a stop always has
        /// something to release.
        open: Arc<AtomicUsize>,
        stopped: Arc<AtomicUsize>,
        clock: Arc<Mutex<Duration>>,
    }

    impl MicLevelsUi for Round {
        fn emit_levels(&self, _levels: &[DeviceLevel]) {}
        fn emit_error(&self, _message: &str) {}
        fn stop(&self) {
            // A real round releases every stream at once.
            self.open.store(0, Ordering::SeqCst);
            self.stopped.fetch_add(1, Ordering::SeqCst);
        }
        fn sleep(
            &self,
            duration: Duration,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
            let clock = self.clock.clone();
            Box::pin(async move {
                tokio::time::sleep(duration).await;
                if let Ok(mut now) = clock.lock() {
                    *now += duration;
                }
            })
        }
        fn now(&self) -> Duration {
            *self.clock.lock().unwrap()
        }
    }

    impl Round {
        fn is_open(&self) -> bool {
            self.open.load(Ordering::SeqCst) > 0
        }
        fn stops(&self) -> usize {
            self.stopped.load(Ordering::SeqCst)
        }
    }

    /// The round's tick loop, run to completion on a paused clock: no
    /// spawning, so 30 s of ticks finish in microseconds. Returns how many
    /// level events it delivered before stopping.
    async fn run_round_to_end(ui: Round, levels: Arc<Mutex<Vec<DeviceLevel>>>) -> usize {
        let mut elapsed = Duration::ZERO;
        let mut emitted = 0;
        loop {
            let before = ui.now();
            ui.sleep(Duration::from_millis(MIC_LEVELS_EMIT_MS)).await;
            elapsed += ui.now().saturating_sub(before);
            if elapsed >= MIC_LEVELS_MAX {
                ui.stop();
                return emitted;
            }
            drop(levels.lock());
            emitted += 1;
        }
    }

    #[test]
    fn mic_levels_auto_stop_after_30s() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            tokio::time::pause();
            let ui = Round::default();
            ui.open.store(3, Ordering::SeqCst);
            let emitted =
                run_round_to_end(ui.clone(), Arc::new(Mutex::new(Vec::<DeviceLevel>::new()))).await;
            assert!(!ui.is_open(), "no stream may outlive 30 s");
            assert_eq!(ui.stops(), 1, "the round releases its streams exactly once");
            assert_eq!(
                emitted,
                (MIC_LEVELS_MAX.as_millis() / MIC_LEVELS_EMIT_MS as u128) as usize - 1,
                "every tick up to the 30 s limit delivers levels; none after it"
            );
        });
    }

    /// The tick loop, ticked by hand so a test can interleave a restart at an
    /// exact moment. `Ok(true)` means the round reached its deadline and
    /// stopped its streams; `Ok(false)` means the tick budget ran out first.
    async fn tick(ui: Round, levels: Arc<Mutex<Vec<DeviceLevel>>>, elapsed: &mut Duration) -> bool {
        let before = ui.now();
        ui.sleep(Duration::from_millis(MIC_LEVELS_EMIT_MS)).await;
        *elapsed += ui.now().saturating_sub(before);
        if *elapsed >= MIC_LEVELS_MAX {
            ui.stop();
            return true;
        }
        drop(levels.lock());
        false
    }

    #[test]
    fn mic_levels_start_resets_auto_stop() {
        // The picker's 20 s keep-alive: a restart at 20 s opens fresh streams
        // and arms a fresh deadline, so the take that began at 0 is never cut
        // short at 30 s and the restarted one still stops at its own 30 s.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            tokio::time::pause();
            let ui = Round::default();
            ui.open.store(3, Ordering::SeqCst);
            let levels = Arc::new(Mutex::new(Vec::<DeviceLevel>::new()));

            let mut first = Duration::ZERO;
            while ui.now() < Duration::from_secs(20) {
                let stopped = tick(ui.clone(), levels.clone(), &mut first).await;
                assert!(!stopped, "the first 20 s stay well under the limit");
            }
            // The picker re-calls `start`: the old round's streams close.
            ui.stop();
            ui.open.store(3, Ordering::SeqCst);
            assert_eq!(ui.stops(), 1, "a replaced round is released at once");

            let mut restarted = Duration::ZERO;
            while ui.now() < Duration::from_millis(29_950) {
                let stopped = tick(ui.clone(), levels.clone(), &mut restarted).await;
                assert!(
                    !stopped,
                    "the restarted round measures 30 s from the restart"
                );
            }
            assert_eq!(ui.stops(), 1, "not yet at the restarted round's deadline");
            assert!(ui.is_open(), "the live streams are still open at 49.95 s");

            let mut restarted = Duration::from_millis(29_950);
            assert!(tick(ui.clone(), levels, &mut restarted).await);
            assert!(
                !ui.is_open(),
                "the restarted round stops itself at its 30 s"
            );
            assert_eq!(ui.stops(), 2);
        });
    }

    #[test]
    fn smoothing_eases_toward_the_new_level_and_rests_at_zero() {
        assert!(smooth_levels(&[], &[]).is_empty());
        assert_eq!(smooth_levels(&[], &[1.0, 0.0]), vec![1.0, 0.0]);
        assert_eq!(smooth_levels(&[0.0], &[1.0]), vec![0.5]);
        assert_eq!(
            smooth_levels(&[0.001], &[0.0]),
            vec![0.0],
            "a decayed level sits at rest"
        );
    }

    #[test]
    fn fallback_is_a_property_of_the_take() {
        use super::FallbackMic;
        use std::sync::atomic::AtomicBool;
        let fallback = FallbackMic {
            buffer: Arc::new(Mutex::new(Vec::new())),
            stream: Arc::new(Mutex::new(None)),
            used: Arc::new(AtomicBool::new(false)),
        };
        assert!(
            !fallback.switched(),
            "a take with no stream error keeps its mic"
        );
        // Two callbacks report the same failed device; the switch is a
        // property of the take, and `used` is its idempotent latch.
        fallback.used.store(true, Ordering::SeqCst);
        fallback.on_error("device unplugged");
        assert!(fallback.switched());
        assert!(
            fallback.take_stream().is_none(),
            "a second error opens nothing"
        );
    }
}
