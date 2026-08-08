/**
 * SpeakType appearance: auto | light | dark
 * Resolves to html[data-theme="light"|"dark"] and syncs across windows.
 */
(function () {
  const MEDIA = '(prefers-color-scheme: dark)';
  let currentPref = 'auto';
  let mediaQuery = null;
  let mediaHandler = null;
  let started = false;

  function normalizePref(pref) {
    if (pref === 'light' || pref === 'dark' || pref === 'auto') return pref;
    return 'auto';
  }

  function systemTheme() {
    try {
      return window.matchMedia(MEDIA).matches ? 'dark' : 'light';
    } catch (_) {
      return 'dark';
    }
  }

  function resolveTheme(pref) {
    const p = normalizePref(pref);
    if (p === 'light' || p === 'dark') return p;
    return systemTheme();
  }

  function applyResolved(resolved) {
    document.documentElement.setAttribute('data-theme', resolved);
  }

  function clearMediaListener() {
    if (mediaQuery && mediaHandler) {
      try {
        mediaQuery.removeEventListener('change', mediaHandler);
      } catch (_) {
        try { mediaQuery.removeListener(mediaHandler); } catch (__) { /* ignore */ }
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
      } else if (mediaQuery.addListener) {
        mediaQuery.addListener(mediaHandler);
      }
    } catch (_) { /* ignore */ }
  }

  function applyTheme(pref) {
    currentPref = normalizePref(pref);
    applyResolved(resolveTheme(currentPref));
    if (currentPref === 'auto') bindMediaListener();
    else clearMediaListener();
    return currentPref;
  }

  function getInvoke() {
    if (window.__TAURI__ && window.__TAURI__.core) return window.__TAURI__.core.invoke;
    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
      return window.__TAURI_INTERNALS__.invoke;
    }
    return null;
  }

  function getListen() {
    if (window.__TAURI__ && window.__TAURI__.event) return window.__TAURI__.event.listen;
    if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.listen) {
      return window.__TAURI_INTERNALS__.listen;
    }
    return null;
  }

  async function loadThemeFromSettings() {
    const invoke = getInvoke();
    if (!invoke) {
      applyTheme('auto');
      return 'auto';
    }
    try {
      const settings = await invoke('get_settings');
      const pref = settings && settings.theme ? settings.theme : 'auto';
      return applyTheme(pref);
    } catch (_) {
      return applyTheme('auto');
    }
  }

  async function startTheme() {
    if (started) return currentPref;
    started = true;
    // Optimistic system theme before settings load (avoids flash of wrong theme).
    applyTheme('auto');
    const pref = await loadThemeFromSettings();

    const listen = getListen();
    if (listen) {
      try {
        await listen('theme:changed', (event) => {
          const payload = event && event.payload;
          const theme = payload && typeof payload === 'object'
            ? payload.theme
            : (typeof payload === 'string' ? payload : null);
          if (theme) applyTheme(theme);
          else loadThemeFromSettings();
        });
      } catch (_) { /* ignore */ }
    }
    return pref;
  }

  window.SpeakTypeTheme = {
    resolveTheme,
    applyTheme,
    loadThemeFromSettings,
    startTheme,
    getPreference: () => currentPref,
  };

  // Apply ASAP; full start (settings + listen) when DOM is ready.
  applyTheme('auto');
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => { startTheme(); });
  } else {
    startTheme();
  }
})();
