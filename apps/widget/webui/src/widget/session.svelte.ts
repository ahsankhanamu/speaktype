import { useEvent } from '../lib/data';
import type { Item, Session, SessionState } from '../lib/bindings';
import { commands } from '../lib/ipc';

/** How long the "Session saved" bar stays up with its Copy button. */
export const SAVED_MS = 10_000;
/** How long a caption stays under the bar after its phrase. */
export const CAPTION_MS = 3_000;

export interface Saved {
  sessionId: number;
  marks: number;
}

/** Whole seconds since a session started, as m:ss. */
export function elapsed(startedAtMs: number, nowMs: number): string {
  const total = Math.max(0, Math.floor((nowMs - startedAtMs) / 1000));
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`;
}

/**
 * The Show & Tell session as the widget shows it: the running session (timer, mute, marks, live
 * captions), then a short "Session saved" with Copy once it stops.
 */
export function useSession() {
  let active = $state<Session | null>(null);
  let muted = $state(false);
  let marks = $state(0);
  let level = $state(0);
  let caption = $state<string | null>(null);
  let saved = $state<Saved | null>(null);
  let now = $state(Date.now());
  let captionTimer: ReturnType<typeof setTimeout> | undefined;
  let savedTimer: ReturnType<typeof setTimeout> | undefined;

  function begin(session: Session) {
    active = session;
    saved = null;
    marks = 0;
    clearTimeout(savedTimer);
    commands
      .sessionGet(session.id)
      .then((detail) => {
        if (active?.id === session.id) marks = detail.items.filter((item) => item.kind === 'mark').length;
      })
      .catch(() => {});
  }

  // A session already running when the widget loads (the app restarted mid-session).
  commands
    .sessionActive()
    .then((session) => session && !active && begin(session))
    .catch(() => {});

  useEvent<SessionState>('session-state', ({ session, phase, muted: isMuted }) => {
    muted = isMuted;
    if (phase === 'active' && session) {
      if (active?.id !== session.id) begin(session);
      return;
    }
    if (active) {
      saved = { sessionId: active.id, marks };
      clearTimeout(savedTimer);
      savedTimer = setTimeout(() => (saved = null), SAVED_MS);
    }
    active = null;
    level = 0;
    caption = null;
  });
  useEvent<{ item: Item }>('session-item-added', ({ item }) => {
    if (item.kind === 'mark' && item.sessionId === active?.id) marks += 1;
  });
  useEvent<{ level: number }>('session:level', (event) => (level = event.level));
  useEvent<{ text: string }>('session:caption', ({ text }) => {
    caption = text;
    clearTimeout(captionTimer);
    captionTimer = setTimeout(() => (caption = null), CAPTION_MS);
  });

  // The timer ticks once a second, only while a session runs.
  $effect(() => {
    if (!active) return;
    let timer: ReturnType<typeof setTimeout>;
    const tick = () => {
      now = Date.now();
      timer = setTimeout(tick, 1000 - (now % 1000));
    };
    tick();
    return () => clearTimeout(timer);
  });
  $effect(() => () => {
    clearTimeout(captionTimer);
    clearTimeout(savedTimer);
  });

  return {
    get active() {
      return active;
    },
    get muted() {
      return muted;
    },
    get marks() {
      return marks;
    },
    get level() {
      return level;
    },
    get caption() {
      return caption;
    },
    get saved() {
      return saved;
    },
    get elapsed() {
      return active ? elapsed(active.startedAtMs, now) : '0:00';
    },
    toggleMute: () => commands.sessionSetMute(!muted),
    stop: () => commands.sessionStop(),
    copy: (sessionId: number) => commands.promptCopy(sessionId, 'codingAgent'),
    dismissSaved() {
      saved = null;
    },
  };
}
