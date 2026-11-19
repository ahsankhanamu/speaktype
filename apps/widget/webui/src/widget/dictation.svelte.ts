import { useEvent } from '../lib/data';
import type { DictationState, NoticeKind } from '../lib/bindings';

export const LONG_AUDIO_MESSAGE = 'Long recording, this may take a moment';

export type Halo = 'recording' | 'active' | 'warning' | 'success' | null;
export type Action = 'pasteAgain' | 'tryAgain' | 'chooseMic';

export interface Notice {
  message: string;
  dot: boolean;
  halo: Halo;
  action?: Action;
  copiedKeys?: boolean;
}

export type View =
  | { kind: 'idle' }
  | { kind: 'preparing' }
  | { kind: 'recording' }
  | { kind: 'transcribing' }
  | { kind: 'pasted' }
  | ({ kind: 'notice' } & Notice);

const HOLD_MS: Record<NoticeKind, number> = {
  pasted: 500,
  noSpeech: 2000,
  busy: 2000,
  copiedNotPasted: 10_000,
  serverError: 10_000,
  micUnavailable: 10_000,
  micFallback: 3000,
  providerFallback: 3000,
  screenshotLimit: 3000,
  longAudio: 3000,
};

const STYLE: Partial<Record<NoticeKind, Omit<Notice, 'message'>>> = {
  noSpeech: { dot: true, halo: 'warning' },
  busy: { dot: true, halo: 'warning' },
  copiedNotPasted: { dot: false, halo: null, action: 'pasteAgain', copiedKeys: true },
  serverError: { dot: true, halo: null, action: 'tryAgain' },
  micUnavailable: { dot: true, halo: null, action: 'chooseMic' },
};
const QUIET = { dot: true, halo: null } as const;

/**
 * What the pill shows, from the typed `dictation-state` event. The app stays in `done` or `error`
 * after a take, so a notice takes itself down once its hold time has passed.
 */
export function useDictation() {
  let state = $state<DictationState>({ phase: 'idle', notice: null });
  let expired = $state(false);
  let longAudio = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  useEvent<DictationState>('dictation-state', (next) => {
    clearTimeout(timer);
    state = next;
    expired = false;
    longAudio = false;
    if (next.notice) timer = setTimeout(() => (expired = true), HOLD_MS[next.notice.kind]);
  });
  useEvent('dictation:long-audio', () => (longAudio = true));
  $effect(() => () => clearTimeout(timer));

  const view = $derived.by((): View => {
    const { phase, notice } = state;
    if (phase === 'preparing' || phase === 'recording') return { kind: phase };
    if (phase === 'transcribing') {
      return longAudio ? { kind: 'notice', message: LONG_AUDIO_MESSAGE, ...QUIET } : { kind: 'transcribing' };
    }
    if (!notice || expired) return { kind: 'idle' };
    if (notice.kind === 'pasted') return { kind: 'pasted' };
    return { kind: 'notice', message: notice.message, ...(STYLE[notice.kind] ?? QUIET) };
  });

  return {
    get view() {
      return view;
    },
    dismiss() {
      expired = true;
    },
  };
}
