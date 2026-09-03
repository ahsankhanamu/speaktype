// Shared settings-session helpers.
//
// The settings form keeps a "snapshot" of the last-loaded/saved values and
// marks the form dirty when any editable field diverges from that snapshot.
// `currentForm()` and `computeDirty()` are the pure logic behind
// `loadedSnapshot` / `checkDirty`; the calling component owns the reactive
// `$state` fields it feeds in.

export interface FormSnapshot {
  hotkey: string;
  api_url: string;
  model: string;
  language: string;
  paste_mode: string;
  post_paste_keys: string;
  save_recordings: boolean;
  hallucination_guard: boolean;
  theme: 'auto' | 'light' | 'dark';
  input_device: string;
}

export interface FormValues {
  hotkey: string;
  api_url: string;
  model: string;
  language: string;
  paste_mode: string;
  post_paste_keys: string;
  save_recordings: boolean;
  hallucination_guard: boolean;
  theme: 'auto' | 'light' | 'dark';
  input_device: string;
}

// Fields persisted with a distinct blank default (post_paste_keys is
// nullable on the Rust side, persisted as an empty string snapshot).
const FIELDS = Object.freeze([
  'hotkey',
  'api_url',
  'model',
  'language',
  'paste_mode',
  'post_paste_keys',
  'save_recordings',
  'hallucination_guard',
  'theme',
  'input_device',
] as const);

export function computeDirty(current: FormValues, snapshot: FormSnapshot): boolean {
  return (FIELDS as readonly string[]).some((key) => {
    const c = (current as unknown as Record<string, unknown>)[key];
    const s = (snapshot as unknown as Record<string, unknown>)[key] ?? (key === 'post_paste_keys' ? '' : c);
    return c !== s;
  });
}

export function defaultSnapshot(): FormSnapshot {
  return {
    hotkey: 'Super+Control',
    api_url: 'http://127.0.0.1:8002/inference',
    model: 'medium',
    language: 'auto',
    paste_mode: 'active',
    post_paste_keys: 'enter',
    save_recordings: false,
    hallucination_guard: true,
    theme: 'auto',
    input_device: '',
  };
}