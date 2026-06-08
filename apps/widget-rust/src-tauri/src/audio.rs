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
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No input device available".to_string())?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = config.sample_rate().0;
        log_message(&format!(
            "[audio] Device: {:?}, sample_rate: {}, channels: {}",
            device.name().unwrap_or_default(),
            sample_rate,
            config.channels()
        ));

        Ok(Self {
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate,
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
