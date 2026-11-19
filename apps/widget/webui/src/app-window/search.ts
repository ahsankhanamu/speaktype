import type { PageId } from './router.svelte';

/** One entry per individual setting. `anchor` is the label of the row to scroll to. */
export interface SearchEntry {
  page: PageId;
  label: string;
  keywords: string[];
  anchor: string;
}

const entry = (page: PageId, label: string, ...keywords: string[]): SearchEntry => ({ page, label, keywords, anchor: label });

export const index: SearchEntry[] = [
  entry('general', 'Launch at login', 'startup', 'open'),
  entry('general', 'Sounds', 'audio', 'beep'),
  entry('general', 'Reset widget position', 'move', 'lost'),
  entry('appearance', 'Appearance', 'theme', 'light', 'dark', 'auto'),
  entry('appearance', 'Widget size', 'small', 'large', 'mic'),
  entry('appearance', 'Colour', 'ink', 'pen', 'swatch'),
  entry('appearance', 'Line thickness', 'ink', 'thin', 'bold'),
  entry('appearance', 'Show the widget', 'hide', 'always', 'menu bar'),
  entry('dictation', 'Start and stop dictation', 'hotkey', 'shortcut', 'f9', 'key'),
  entry('dictation', 'Language', 'english', 'automatic', 'translate'),
  entry('dictation', 'Microphone', 'input', 'device', 'level'),
  entry('dictation', 'Paste into', 'window', 'app'),
  entry('dictation', 'After pasting, press', 'return', 'enter', 'tab', 'send'),
  entry('dictation', 'Save recording audio', 'keep', 'replay', 'wav'),
  entry('dictation', 'Speech model', 'whisper', 'fast', 'accurate', 'download', 'manage'),
  entry('sessions', 'Screen Recording', 'screenshot', 'capture'),
  entry('sessions', 'Start or stop a session', 'hotkey', 'shortcut'),
  entry('sessions', 'Draw while holding', 'pen', 'ink', 'draw key', 'option'),
  entry('sessions', 'Draw anytime', 'pen', 'ink', 'always'),
  entry('sessions', 'Screenshot without drawing', 'capture', 'hotkey'),
  entry('sessions', 'Include text I copy', 'clipboard', 'capture'),
  entry('providers', 'Speech to text', 'provider', 'cloud', 'server', 'openai', 'groq', 'deepgram', 'api key', 'keychain'),
  entry('permissions', 'Microphone', 'permission', 'test', 'level'),
  entry('permissions', 'Accessibility', 'permission', 'typing', 'paste'),
  entry('permissions', 'Reset permissions…', 'forgot', 'update', 'onboarding'),
  entry('storage', 'Keep history', 'retention', 'limit', 'forever'),
  entry('storage', 'Speech models on disk', 'disk', 'space', 'folder'),
  entry('storage', 'Clear history', 'delete', 'erase'),
  entry('advanced', 'Hallucination filter', 'guard', 'retries', 'threshold', 'repetition'),
  entry('advanced', 'Transcription server', 'stop', 'restart', 'api url', 'check', 'whisper'),
  entry('advanced', 'Recent dictation debug records', 'debug', 'chunks', 'diagnostics'),
  entry('about', 'About SpeakType', 'version', 'update', 'licence', 'license'),
];

export function filter(query: string): SearchEntry[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [];
  return index.filter(({ label, keywords }) => [label, ...keywords].some((text) => text.toLowerCase().includes(needle)));
}
