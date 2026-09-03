import { getSettings, listenEvent } from './ipc';

const MEDIA = '(prefers-color-scheme: dark)';
let currentPref: 'auto' | 'light' | 'dark' = 'auto';
let mediaQuery: MediaQueryList | null = null;
let mediaHandler: ((this: MediaQueryList, ev: MediaQueryListEvent) => unknown) | null = null;

function normalizePref(pref: string): 'auto' | 'light' | 'dark' {
  if (pref === 'light' || pref === 'dark' || pref === 'auto') return pref;
  return 'auto';
}

function systemTheme(): 'light' | 'dark' {
  try {
    return window.matchMedia(MEDIA).matches ? 'dark' : 'light';
  } catch (_) {
    return 'dark';
  }
}

export function resolveTheme(pref: string): 'light' | 'dark' {
  const p = normalizePref(pref);
  if (p === 'light' || p === 'dark') return p;
  return systemTheme();
}

function applyResolved(resolved: 'light' | 'dark') {
  document.documentElement.setAttribute('data-theme', resolved);
}

function clearMediaListener() {
  if (mediaQuery && mediaHandler) {
    try {
      mediaQuery.removeEventListener('change', mediaHandler);
    } catch (_) {
      try {
        (mediaQuery as MediaQueryList & { removeListener?: (h: unknown) => void }).removeListener?.(mediaHandler);
      } catch (__) {
        /* ignore */
      }
    }
  }
  mediaQuery = null;
  mediaHandler = null;
}

function bindMediaListener() {
  clearMediaListener();
  try {
    mediaQuery = window.matchMedia(MEDIA);
    mediaHandler = () => {
      if (currentPref === 'auto') applyResolved(systemTheme());
    };
    if (mediaQuery.addEventListener) {
      mediaQuery.addEventListener('change', mediaHandler);
    } else {
      (mediaQuery as MediaQueryList & { addListener?: (h: unknown) => void }).addListener?.(mediaHandler);
    }
  } catch (_) {
    /* ignore */
  }
}

export function applyTheme(pref: string): 'auto' | 'light' | 'dark' {
  currentPref = normalizePref(pref);
  applyResolved(resolveTheme(currentPref));
  if (currentPref === 'auto') bindMediaListener();
  else clearMediaListener();
  return currentPref;
}

async function loadThemeFromSettings() {
  try {
    const settings = await getSettings();
    const pref = settings && settings.theme ? settings.theme : 'auto';
    return applyTheme(pref);
  } catch (_) {
    return applyTheme('auto');
  }
}

export async function startTheme() {
  applyTheme('auto');
  const pref = await loadThemeFromSettings();
  try {
    await listenEvent('theme:changed', (payload: unknown) => {
      const theme = payload && typeof payload === 'object'
        ? (payload as Record<string, unknown>).theme
        : (typeof payload === 'string' ? payload : null);
      if (theme) applyTheme(theme as string);
      else loadThemeFromSettings();
    });
  } catch (_) {
    /* ignore */
  }
  return pref;
}

export function getPreference(): 'auto' | 'light' | 'dark' {
  return currentPref;
}