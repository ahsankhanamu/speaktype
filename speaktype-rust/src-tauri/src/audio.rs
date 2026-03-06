use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

use crate::logging::log_message;

/// Wrapper to make cpal::Stream Send + Sync.
/// cpal::Stream is intentionally !Send on some platforms as a conservative measure,
/// but it's safe when access is synchronized via Mutex.
/// The inner field keeps the stream alive — dropping it stops recording.
#[allow(dead_code)]
struct SendStream(cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

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

        // Clear buffer
        {
            let mut buf = self.buffer.lock().unwrap();
            buf.clear();
        }

        let buffer = self.buffer.clone();
        let app = app_handle.clone();
        let sample_rate = self.sample_rate;

        // Track time for throttled audio level events (~15fps)
        let last_level_time = Arc::new(Mutex::new(std::time::Instant::now()));

        let stream_config: cpal::StreamConfig = config.into();

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Convert to mono by averaging channels
                    let mono: Vec<f32> = if channels == 1 {
                        data.to_vec()
                    } else {
                        data.chunks(channels)
                            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                            .collect()
                    };

                    // Push to buffer
                    {
                        let mut buf = buffer.lock().unwrap();
                        buf.extend_from_slice(&mono);
                    }

                    // Emit audio level at ~15fps
                    let now = std::time::Instant::now();
                    let mut last = last_level_time.lock().unwrap();
                    if now.duration_since(*last).as_millis() >= 66 {
                        *last = now;
                        // Calculate RMS level
                        let sum: f32 = mono.iter().map(|s| s * s).sum();
                        let rms = (sum / mono.len() as f32).sqrt();
                        let level = (rms * 10000.0).round() / 10000.0;
                        let _ = app.emit(
                            "sidecar:audio_level",
                            serde_json::json!({"level": level}),
                        );
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

    /// Stop recording and return (samples, sample_rate)
    pub fn stop(&mut self) -> (Vec<f32>, u32) {
        // Drop the stream to stop recording
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

/// Check if audio contains actual speech using segment-based RMS detection.
/// Matches Python's has_speech() logic.
pub fn has_speech(audio: &[f32], sample_rate: u32) -> bool {
    let threshold: f32 = 0.01;
    let segment_ms: u32 = 50;
    let segment_samples = (sample_rate * segment_ms / 1000) as usize;

    for chunk in audio.chunks(segment_samples) {
        if chunk.len() < segment_samples / 2 {
            continue; // Skip tiny trailing segments
        }
        let sum: f32 = chunk.iter().map(|s| s * s).sum();
        let energy = (sum / chunk.len() as f32).sqrt();
        if energy > threshold {
            return true;
        }
    }

    false
}

/// Return the name of the current default input device.
pub fn get_default_input_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_input_device()
        .and_then(|d| d.name().ok())
}

/// Count the number of available input devices.
pub fn count_input_devices() -> usize {
    let host = cpal::default_host();
    host.input_devices().map(|d| d.count()).unwrap_or(0)
}

/// Encode f32 samples as WAV bytes (16-bit mono PCM)
pub fn to_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, String> {
    let num_samples = samples.len();
    let data_size = (num_samples * 2) as u32; // 16-bit = 2 bytes per sample
    let file_size = 36 + data_size;

    let mut buf = Vec::with_capacity(44 + data_size as usize);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());

    for &sample in samples {
        let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        buf.extend_from_slice(&s.to_le_bytes());
    }

    Ok(buf)
}
