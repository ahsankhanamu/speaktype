import '../lib/theme.css';
import { mount } from 'svelte';
import Widget from './Widget.svelte';
import { startTheme } from '../lib/theme';
import { logFrontend } from '../lib/ipc';

startTheme();

// Forward webview console errors/warnings into speaktype.log so a frozen
// widget leaves a trace next time it happens.
let reporting = false;
const report = (level: string, args: unknown[]) => {
  if (reporting) return;
  reporting = true;
  try {
    const msg = args
      .map((x) => (typeof x === 'string' ? x : JSON.stringify(x)))
      .join(' ');
    logFrontend(level, msg);
  } catch (e) {
    /* ignore */
  } finally {
    reporting = false;
  }
};
const origErr = console.error;
const origWarn = console.warn;
console.error = function (...args: unknown[]) {
  report('error', [...args]);
  return (origErr as Function).apply(console, args);
};
console.warn = function (...args: unknown[]) {
  report('warn', [...args]);
  return (origWarn as Function).apply(console, args);
};

const target = document.getElementById('app');
if (target) {
  mount(Widget, { target });
} else {
  mount(Widget, { target: document.body });
}