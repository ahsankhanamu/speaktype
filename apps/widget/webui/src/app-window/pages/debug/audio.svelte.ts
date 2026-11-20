import { getDebugAudioSlice } from '../../../lib/ipc';

/** 16-bit mono PCM WAV (what the debug recorder keeps) to samples in -1..1. */
export function decodeWav(bytes: Uint8Array): { samples: Float32Array; rate: number } {
  if (bytes.byteLength <= 44) return { samples: new Float32Array(0), rate: 16_000 };
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const rate = view.getUint32(24, true);
  const samples = new Float32Array(Math.floor((bytes.byteLength - 44) / 2));
  for (let i = 0; i < samples.length; i++) samples[i] = view.getInt16(44 + i * 2, true) / 32768;
  return { samples, rate };
}

/** The loudest sample in each of `bars` equal slices, for drawing a waveform. */
export function peaks(samples: Float32Array, bars: number): number[] {
  if (!samples.length || bars <= 0) return [];
  const per = samples.length / bars;
  return Array.from({ length: bars }, (_, i) => {
    let peak = 0;
    for (let j = Math.floor(i * per); j < Math.min(samples.length, Math.floor((i + 1) * per)); j++) peak = Math.max(peak, Math.abs(samples[j]));
    return peak;
  });
}

export interface Clip {
  sessionId: number;
  start: number | null;
  end: number | null;
}

/** A recorded stretch of a debug session, as WAV bytes. */
export async function loadClip({ sessionId, start, end }: Clip): Promise<Uint8Array<ArrayBuffer>> {
  return Uint8Array.from(await getDebugAudioSlice(sessionId, start, end));
}

const key = (clip: Clip) => `${clip.sessionId}:${clip.start}:${clip.end}`;

/** One player for the whole page: starting a clip stops the one playing. */
class Player {
  playing = $state<string | null>(null);
  progress = $state(0);
  #audio: HTMLAudioElement | null = null;
  #url: string | null = null;

  isPlaying(clip: Clip) {
    return this.playing === key(clip);
  }

  stop() {
    this.#audio?.pause();
    if (this.#url) URL.revokeObjectURL(this.#url);
    this.#audio = null;
    this.#url = null;
    this.playing = null;
    this.progress = 0;
  }

  /** Play `clip` from `from` (0..1 of its length); playing it again stops it. */
  async toggle(clip: Clip, from = 0) {
    const id = key(clip);
    if (this.playing === id && from === 0) return this.stop();
    this.stop();
    const bytes = await loadClip(clip);
    if (!bytes.length) return;
    const url = URL.createObjectURL(new Blob([bytes], { type: 'audio/wav' }));
    const audio = new Audio(url);
    this.#audio = audio;
    this.#url = url;
    this.playing = id;
    audio.addEventListener('loadedmetadata', () => (audio.currentTime = from * audio.duration), { once: true });
    audio.addEventListener('timeupdate', () => (this.progress = audio.duration ? audio.currentTime / audio.duration : 0));
    audio.addEventListener('ended', () => this.stop(), { once: true });
    await audio.play();
  }
}

export const player = new Player();
