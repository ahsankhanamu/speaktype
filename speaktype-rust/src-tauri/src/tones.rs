use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::f32::consts::PI;

use crate::logging::log_message;

/// Tone types for different app events
pub enum Tone {
    RecordingStart,
    RecordingStop,
    Error,
}

/// Generate a chord: multiple frequencies layered together with fade envelope.
/// Matches the Python _tone() approach exactly.
fn generate_chord(freqs: &[f32], duration_secs: f32, volume: f32, sample_rate: u32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let fade = (sample_rate as f32 * 0.015) as usize; // 15ms fade in/out

    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;

            // Mix all frequencies together
            let wave: f32 = freqs.iter().map(|f| (2.0 * PI * f * t).sin()).sum::<f32>()
                / freqs.len() as f32;

            // Smooth fade envelope (prevents clicks)
            let env = if i < fade {
                i as f32 / fade as f32
            } else if i >= num_samples - fade {
                (num_samples - i) as f32 / fade as f32
            } else {
                1.0
            };

            wave * volume * env
        })
        .collect()
}

/// Generate the waveform for a tone type — same sounds as Python version
fn generate_tone(tone: &Tone, sample_rate: u32) -> Vec<f32> {
    match tone {
        // A5+E6 — bright rising chord
        Tone::RecordingStart => generate_chord(&[880.0, 1320.0], 0.09, 0.08, sample_rate),
        // E5+A4 — soft descending chord
        Tone::RecordingStop => generate_chord(&[660.0, 440.0], 0.11, 0.08, sample_rate),
        // Low dissonant pair — gentle alert
        Tone::Error => generate_chord(&[300.0, 260.0], 0.18, 0.09, sample_rate),
    }
}

/// Play a tone asynchronously (non-blocking)
pub fn play(tone: Tone) {
    std::thread::spawn(move || {
        if let Err(e) = play_blocking(&tone) {
            log_message(&format!("[tones] Failed to play tone: {}", e));
        }
    });
}

fn play_blocking(tone: &Tone) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "No output device available".to_string())?;

    let config = device
        .default_output_config()
        .map_err(|e| format!("Output config error: {}", e))?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;
    let samples = generate_tone(tone, sample_rate);
    let total_samples = samples.len();

    let position = Arc::new(AtomicUsize::new(0));
    let pos_clone = position.clone();

    let stream_config: cpal::StreamConfig = config.into();

    let stream = device
        .build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let pos = pos_clone.fetch_add(1, Ordering::Relaxed);
                    let sample = if pos < total_samples {
                        samples[pos]
                    } else {
                        0.0
                    };
                    for s in frame.iter_mut() {
                        *s = sample;
                    }
                }
            },
            move |err| {
                log_message(&format!("[tones] Stream error: {}", err));
            },
            None,
        )
        .map_err(|e| format!("Failed to build output stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Failed to play stream: {}", e))?;

    // Wait for playback to finish + small buffer for audio device to flush
    let duration_ms = (total_samples as f64 / sample_rate as f64 * 1000.0) as u64;
    std::thread::sleep(std::time::Duration::from_millis(duration_ms + 50));

    // stream drops here, stopping playback
    Ok(())
}
