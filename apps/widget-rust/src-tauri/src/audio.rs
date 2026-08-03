use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

use crate::logging::log_message;

/// Wraps `cpal::Stream` which is !Send on some backends (Linux ALSA).
/// On macOS (CoreAudio) dropping from any thread is safe. The `pause()`
/// call in `stop()` ensures the audio callback has stopped before we drop.
struct SendStream(cpal::Stream);
unsafe impl Send for SendStream {}

pub struct AudioRecorder {
    stream: Option<SendStream>,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl AudioRecorder {
    pub fn buffer(&self) -> Arc<Mutex<Vec<f32>>> {
        self.buffer.clone()
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
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
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: DEFAULT_SAMPLE_RATE,
        })
    }

    pub fn start(&mut self, app_handle: AppHandle) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No input device available".to_string())?;

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
                        if now.duration_since(*last).as_millis() >= 66 {
                            *last = now;
                            let sum: f32 = local_downmix.iter().map(|s| s * s).sum();
                            let rms = (sum / local_downmix.len() as f32).sqrt();
                            let level = (rms * 10000.0).round() / 10000.0;
                            let _ = app.emit(
                                "sidecar:audio_level",
                                serde_json::json!({"level": level}),
                            );
                        }
                    }
                },
                move |err| {
                    log_message(&format!("[audio] Stream error: {}", err));
                },
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        self.stream = Some(SendStream(stream));
        log_message(&format!(
            "[audio] Recording started at {}Hz, {} channels",
            sample_rate, channels
        ));

        Ok(())
    }

    pub fn stop(&mut self) -> (Vec<f32>, u32) {
        if let Some(ref stream) = self.stream {
            let _ = stream.0.pause();
        }
        self.stream.take();

        let samples = {
            let mut buf = self.buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };

        let duration_ms = if self.sample_rate > 0 {
            (samples.len() as f64 / self.sample_rate as f64 * 1000.0) as u64
        } else {
            0
        };

        log_message(&format!(
            "[audio] Recording stopped: {} samples, {}ms",
            samples.len(),
            duration_ms
        ));

        (samples, self.sample_rate)
    }
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
                    speech_secs,
                    required_ms
                ));
                return Some(split);
            }
        }
        offset += segment_samples;
    }

    None
}

/// Split completed audio into phrase chunks on silence boundaries.
pub fn split_on_phrase_boundaries(audio: &[f32], sample_rate: u32) -> Vec<Vec<f32>> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < audio.len() {
        let slice = &audio[start..];
        if let Some(rel_end) = find_phrase_flush_point(slice, sample_rate) {
            let end = start + rel_end;
            if end > start {
                chunks.push(audio[start..end].to_vec());
            }
            start = end;
        } else {
            chunks.push(audio[start..].to_vec());
            break;
        }
    }
    chunks.retain(|c| has_speech(c, sample_rate));
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
    host.default_input_device()
        .and_then(|d| d.name().ok())
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
        samples.iter().map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16),
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
