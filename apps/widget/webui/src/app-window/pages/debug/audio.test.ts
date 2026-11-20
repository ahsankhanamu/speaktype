import { describe, expect, it } from 'vitest';
import { decodeWav, peaks } from './audio.svelte';

function wav(samples: number[], rate = 16_000): Uint8Array {
  const bytes = new Uint8Array(44 + samples.length * 2);
  const view = new DataView(bytes.buffer);
  view.setUint32(24, rate, true);
  samples.forEach((s, i) => view.setInt16(44 + i * 2, Math.round(s * 32767), true));
  return bytes;
}

describe('debug audio', () => {
  it('decodes 16-bit PCM WAV to samples and rate', () => {
    const { samples, rate } = decodeWav(wav([0, 0.5, -0.5], 22_050));
    expect(rate).toBe(22_050);
    expect(samples.length).toBe(3);
    expect(samples[1]).toBeCloseTo(0.5, 3);
  });

  it('reads an empty or header-only file as silence', () => {
    expect(decodeWav(new Uint8Array(10)).samples.length).toBe(0);
  });

  it('takes the loudest sample in each slice for the waveform', () => {
    const p = peaks(new Float32Array([0.1, -0.8, 0.2, 0.3]), 2);
    expect(p[0]).toBeCloseTo(0.8);
    expect(p[1]).toBeCloseTo(0.3);
  });
});
