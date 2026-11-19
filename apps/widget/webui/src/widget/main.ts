import '@speaktype/ui/tokens.css';
import { mount } from 'svelte';
import { logFrontend } from '../lib/ipc';
import Widget from './Widget.svelte';

// The widget is dark in both themes (spec §6.1).
document.documentElement.dataset.theme = 'dark';

// A frozen widget leaves a trace in speaktype.log.
for (const level of ['error', 'warn'] as const) {
  const original = console[level];
  console[level] = (...args: unknown[]) => {
    original(...args);
    void logFrontend(level, args.map(String).join(' '));
  };
}

mount(Widget, { target: document.getElementById('app')! });
