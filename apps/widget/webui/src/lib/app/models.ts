import { formatByteSize } from '../format';

export type Tier = 'fast' | 'balanced' | 'accurate' | 'more';

/** One catalog entry from `get_models().available`. */
export interface ModelInfo {
  id: string;
  size_bytes?: number;
  downloaded?: boolean;
  partial?: boolean;
  partial_size?: number;
  expected_size?: number;
}

/** A `model:progress` payload, plus the two fields the data layer derives. */
export interface ModelProgress {
  phase: string;
  message?: string;
  percent?: number;
  bytes_downloaded?: number;
  total_bytes?: number;
  eta_secs?: number;
  /** How long this download has been reporting, in ms. */
  elapsed_ms?: number;
  /** Plain name of the model a queued download starts after. */
  after?: string;
}

export interface ModelState {
  label: string;
  tone: 'success' | 'accent' | 'warning' | 'danger' | 'neutral';
  action: 'use' | 'download' | 'pause' | 'cancel' | 'resume' | 'retry' | null;
  /** Bytes and time left, while downloading. */
  detail?: string;
  /** 0–100, while downloading. */
  percent?: number;
}

interface PlainModel {
  name: string;
  tier: Tier;
  detail: string;
  size: string;
  recommended?: true;
}

const NAMES: Record<string, PlainModel> = {
  tiny: { name: 'Tiny', tier: 'more', detail: 'Smallest download, least accurate', size: '~75 MB' },
  base: { name: 'Fast', tier: 'fast', detail: 'Base · quickest for short messages', size: '~150 MB' },
  small: { name: 'Balanced', tier: 'balanced', detail: 'Small · best for most people', size: '~500 MB', recommended: true },
  medium: { name: 'Medium', tier: 'more', detail: 'Medium · good on Macs with 16 GB or more', size: '~1.5 GB' },
  'large-v3-turbo': { name: 'Accurate', tier: 'accurate', detail: 'Large v3 Turbo · names, accents, noisy rooms', size: '~1.5 GB' },
  'large-v3': { name: 'Most accurate', tier: 'more', detail: 'Large v3 · every language · needs 16 GB of memory', size: '~3 GB' },
};

/** Catalog base ids in slider order, least to most accurate. */
export const MODEL_ORDER = Object.keys(NAMES);

export function plainModel(id: string): PlainModel {
  return NAMES[id.replace(/\.en$/, '')] ?? { name: id, tier: 'more', detail: '', size: '' };
}

const LOADING = new Set(['preparing', 'stopping', 'starting', 'loading']);
const ETA_AFTER_MS = 4000;

function downloadDetail(p: ModelProgress): string {
  const bytes = `${formatByteSize(p.bytes_downloaded ?? 0)} of ${formatByteSize(p.total_bytes ?? 0)}`;
  if (!p.eta_secs || (p.elapsed_ms ?? 0) < ETA_AFTER_MS) return bytes;
  const left = p.eta_secs < 60 ? `${Math.round(p.eta_secs)} s` : `${Math.round(p.eta_secs / 60)} min`;
  return `${bytes} · ${left}`;
}

export function modelState(m: ModelInfo, progress: ModelProgress | undefined, activeId: string): ModelState {
  const phase = progress?.phase;
  if (progress && phase && LOADING.has(phase)) return { label: 'Getting ready…', tone: 'accent', action: null };
  if (progress && phase === 'downloading') {
    const percent = Math.round(progress.percent ?? 0);
    return { label: `Downloading · ${percent}%`, tone: 'accent', action: 'pause', detail: downloadDetail(progress), percent };
  }
  if (progress && phase === 'waiting') {
    const label = progress.after ? `Waiting · starts after ${progress.after}` : 'Waiting';
    return { label, tone: 'neutral', action: 'cancel' };
  }
  if (progress && phase === 'paused') {
    return { label: `Paused at ${Math.round(progress.percent ?? 0)}%`, tone: 'neutral', action: 'resume' };
  }
  if (progress && phase === 'retrying') return { label: progress.message ?? 'Retrying…', tone: 'warning', action: null };
  if (progress && phase === 'error') return { label: progress.message ?? 'Download failed', tone: 'danger', action: 'retry' };
  if (m.downloaded) {
    return m.id === activeId
      ? { label: 'In use', tone: 'success', action: null }
      : { label: 'Downloaded', tone: 'neutral', action: 'use' };
  }
  if (m.partial && m.expected_size) {
    const pct = Math.min(99, Math.round(((m.partial_size ?? 0) / m.expected_size) * 100));
    return { label: `Paused at ${pct}%`, tone: 'neutral', action: 'resume' };
  }
  const { size } = plainModel(m.id);
  return { label: size ? `Download · ${size}` : 'Download', tone: 'neutral', action: 'download' };
}
