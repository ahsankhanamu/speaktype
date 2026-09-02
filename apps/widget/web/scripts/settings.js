(() => {
  // Forward webview console errors/warnings into speaktype.log so a frozen
  // settings window leaves a trace next time it happens.
  let reporting = false;
  const report = (level, args) => {
    if (reporting) return;
    reporting = true;
    try {
      const msg = Array.from(args)
        .map((x) => (typeof x === 'string' ? x : JSON.stringify(x)))
        .join(' ');
      if (window.ttipc) window.ttipc.invoke('log_frontend', { level, message: msg });
    } catch (e) { /* ignore */ }
    finally {
      reporting = false;
    }
  };
  const origErr = console.error;
  const origWarn = console.warn;
  console.error = function () { report('error', arguments); return origErr.apply(console, arguments); };
  console.warn = function () { report('warn', arguments); return origWarn.apply(console, arguments); };
})();

const tabs = document.querySelectorAll('.tab');
const tabContents = document.querySelectorAll('.tab-content');
const paneTitle = document.getElementById('pane-title');
const paneDesc = document.getElementById('pane-desc');
const paneBody = document.querySelector('.pane-body');
const modelsDownloadBadge = document.getElementById('models-download-badge');

const actionsSettings = document.getElementById('actions-settings');
const actionsHistory = document.getElementById('actions-history');
const actionsModels = document.getElementById('actions-models');
const actionsDebug = document.getElementById('actions-debug');
const resetPositionBtn = document.getElementById('reset-position-btn');
const saveBtn = document.getElementById('save-btn');

const WHISPER_LANGUAGES = [
  { code: 'auto', name: 'Auto-detect' },
  { code: 'af', name: 'Afrikaans' },
  { code: 'am', name: 'Amharic' },
  { code: 'ar', name: 'Arabic' },
  { code: 'as', name: 'Assamese' },
  { code: 'az', name: 'Azerbaijani' },
  { code: 'ba', name: 'Bashkir' },
  { code: 'be', name: 'Belarusian' },
  { code: 'bg', name: 'Bulgarian' },
  { code: 'bn', name: 'Bengali' },
  { code: 'bo', name: 'Tibetan' },
  { code: 'br', name: 'Breton' },
  { code: 'bs', name: 'Bosnian' },
  { code: 'ca', name: 'Catalan' },
  { code: 'cs', name: 'Czech' },
  { code: 'cy', name: 'Welsh' },
  { code: 'da', name: 'Danish' },
  { code: 'de', name: 'German' },
  { code: 'el', name: 'Greek' },
  { code: 'en', name: 'English' },
  { code: 'es', name: 'Spanish' },
  { code: 'et', name: 'Estonian' },
  { code: 'eu', name: 'Basque' },
  { code: 'fa', name: 'Persian' },
  { code: 'fi', name: 'Finnish' },
  { code: 'fo', name: 'Faroese' },
  { code: 'fr', name: 'French' },
  { code: 'gl', name: 'Galician' },
  { code: 'gu', name: 'Gujarati' },
  { code: 'ha', name: 'Hausa' },
  { code: 'haw', name: 'Hawaiian' },
  { code: 'he', name: 'Hebrew' },
  { code: 'hi', name: 'Hindi' },
  { code: 'hr', name: 'Croatian' },
  { code: 'ht', name: 'Haitian' },
  { code: 'hu', name: 'Hungarian' },
  { code: 'hy', name: 'Armenian' },
  { code: 'id', name: 'Indonesian' },
  { code: 'is', name: 'Icelandic' },
  { code: 'it', name: 'Italian' },
  { code: 'ja', name: 'Japanese' },
  { code: 'jw', name: 'Javanese' },
  { code: 'ka', name: 'Georgian' },
  { code: 'kk', name: 'Kazakh' },
  { code: 'km', name: 'Khmer' },
  { code: 'kn', name: 'Kannada' },
  { code: 'ko', name: 'Korean' },
  { code: 'la', name: 'Latin' },
  { code: 'lb', name: 'Luxembourgish' },
  { code: 'ln', name: 'Lingala' },
  { code: 'lo', name: 'Lao' },
  { code: 'lt', name: 'Lithuanian' },
  { code: 'lv', name: 'Latvian' },
  { code: 'mg', name: 'Malagasy' },
  { code: 'mi', name: 'Maori' },
  { code: 'mk', name: 'Macedonian' },
  { code: 'ml', name: 'Malayalam' },
  { code: 'mn', name: 'Mongolian' },
  { code: 'mr', name: 'Marathi' },
  { code: 'ms', name: 'Malay' },
  { code: 'mt', name: 'Maltese' },
  { code: 'my', name: 'Myanmar (Burmese)' },
  { code: 'ne', name: 'Nepali' },
  { code: 'nl', name: 'Dutch' },
  { code: 'nn', name: 'Norwegian Nynorsk' },
  { code: 'no', name: 'Norwegian' },
  { code: 'oc', name: 'Occitan' },
  { code: 'pa', name: 'Punjabi' },
  { code: 'pl', name: 'Polish' },
  { code: 'ps', name: 'Pashto' },
  { code: 'pt', name: 'Portuguese' },
  { code: 'ro', name: 'Romanian' },
  { code: 'ru', name: 'Russian' },
  { code: 'sa', name: 'Sanskrit' },
  { code: 'sd', name: 'Sindhi' },
  { code: 'si', name: 'Sinhala' },
  { code: 'sk', name: 'Slovak' },
  { code: 'sl', name: 'Slovenian' },
  { code: 'sn', name: 'Shona' },
  { code: 'so', name: 'Somali' },
  { code: 'sq', name: 'Albanian' },
  { code: 'sr', name: 'Serbian' },
  { code: 'su', name: 'Sundanese' },
  { code: 'sv', name: 'Swedish' },
  { code: 'sw', name: 'Swahili' },
  { code: 'ta', name: 'Tamil' },
  { code: 'te', name: 'Telugu' },
  { code: 'tg', name: 'Tajik' },
  { code: 'th', name: 'Thai' },
  { code: 'tk', name: 'Turkmen' },
  { code: 'tl', name: 'Tagalog' },
  { code: 'tr', name: 'Turkish' },
  { code: 'tt', name: 'Tatar' },
  { code: 'uk', name: 'Ukrainian' },
  { code: 'ur', name: 'Urdu' },
  { code: 'uz', name: 'Uzbek' },
  { code: 'vi', name: 'Vietnamese' },
  { code: 'yi', name: 'Yiddish' },
  { code: 'yo', name: 'Yoruba' },
  { code: 'zh', name: 'Chinese' },
];

const LANGUAGE_FILTER_DEBOUNCE_MS = 120;

let languageOptions = null;
let languageFilterTimer = null;

function buildLanguageOptions() {
  return WHISPER_LANGUAGES.map(lang => {
    const el = document.createElement('option');
    el.value = lang.code;
    el.textContent = lang.code === 'auto' ? lang.name : `${lang.name} (${lang.code})`;
    return { code: lang.code, name: lang.name.toLowerCase(), el };
  });
}

function populateLanguageSelect(filter) {
  const select = document.getElementById('language-select');
  if (!languageOptions) languageOptions = buildLanguageOptions();
  const current = select.value || 'auto';
  const q = (filter || '').toLowerCase();
  const matches = q
    ? languageOptions.filter(o => o.name.includes(q) || o.code.includes(q))
    : languageOptions;

  select.replaceChildren(...matches.map(o => o.el));
  select.value = matches.some(o => o.code === current) ? current : 'auto';
}

document.getElementById('language-search').addEventListener('input', (e) => {
  const value = e.target.value;
  clearTimeout(languageFilterTimer);
  languageFilterTimer = setTimeout(() => populateLanguageSelect(value), LANGUAGE_FILTER_DEBOUNCE_MS);
});

let loadedSnapshot = {};
let modelStatusTimer = null;
let activeTab = 'hotkey';

const modelGrid = new ModelGrid({
  container: 'model-grid',
  modelSelect: document.getElementById('model-select'),
  mode: 'settings',
  getActiveModel() {
    const select = document.getElementById('model-select');
    return (select && select.value) || loadedSnapshot.model || '';
  },
  hooks: {
    onLoading(text) { setStatusLoading(text); },
    onHideStatus() { hideModelStatus(); },
    onStatus(type, html) { showModelStatus(type, html); },
    onScheduleHideStatus(ms) { scheduleHideModelStatus(ms); },
    onModelSaved(model) {
      loadedSnapshot.model = model;
      checkDirty();
    },
    onDownloadCount(count) { setModelsDownloadBadge(count); },
  },
});

function setModelsDownloadBadge(count) {
  if (!modelsDownloadBadge) return;
  const n = Number(count) || 0;
  modelsDownloadBadge.hidden = n === 0;
  modelsDownloadBadge.textContent = n === 0 ? '' : String(n);
  modelsDownloadBadge.setAttribute(
    'aria-label',
    n === 1 ? '1 model downloading' : `${n} models downloading`
  );
}

function switchTab(tabName) {
  const navItem = document.querySelector(`.tab[data-tab="${tabName}"]`);
  tabs.forEach(t => t.classList.remove('active'));
  tabContents.forEach(tc => tc.classList.remove('active'));
  navItem.classList.add('active');
  document.getElementById(`tab-${tabName}`).classList.add('active');

  activeTab = tabName;
  paneTitle.textContent = navItem.dataset.title || tabName;
  paneDesc.textContent = navItem.dataset.desc || '';
  paneBody.scrollTop = 0;
  syncPermissionPoll();
  if (window.MicTest) MicTest.setSectionVisible(tabName === 'general');

  actionsSettings.style.display = 'none';
  actionsHistory.style.display = 'none';
  actionsModels.style.display = 'none';
  actionsDebug.style.display = 'none';

  if (tabName === 'history') {
    actionsHistory.style.display = '';
    loadHistory();
  } else if (tabName === 'models') {
    actionsModels.style.display = '';
    modelGrid.ensureListeners();
    modelGrid.refresh(true);
  } else if (tabName === 'debug') {
    actionsDebug.style.display = '';
    loadDebugSessions();
  } else {
    actionsSettings.style.display = '';
    resetPositionBtn.hidden = tabName !== 'general';
    if (tabName === 'general') checkPermissions();
    if (tabName === 'server') refreshServerControls();
  }
}

tabs.forEach(tab => {
  tab.addEventListener('click', () => switchTab(tab.dataset.tab));
});

function getPostPasteKeys() {
  const custom = document.getElementById('post-paste-keys-custom').value.trim();
  if (custom) return custom;
  return document.getElementById('post-paste-keys-select').value;
}

function getThemePref() {
  const checked = document.querySelector('input[name="theme-pref"]:checked');
  return checked ? checked.value : 'auto';
}

function setThemePref(pref) {
  const normalized = pref === 'light' || pref === 'dark' ? pref : 'auto';
  const target = document.querySelector(`input[name="theme-pref"][value="${normalized}"]`);
  if (target) target.checked = true;
  return normalized;
}

function getFormValues() {
  return {
    hotkey: capturedHotkey || loadedSnapshot.hotkey || '',
    api_url: document.getElementById('api-url').value,
    model: document.getElementById('model-select').value,
    language: document.getElementById('language-select').value,
    paste_mode: document.getElementById('paste-mode-select').value,
    post_paste_keys: getPostPasteKeys(),
    save_recordings: document.getElementById('save-recordings').checked,
    hallucination_guard: document.getElementById('hallucination-guard').checked,
    theme: getThemePref(),
    input_device: document.getElementById('input-device-select').value,
  };
}

function checkDirty() {
  const current = getFormValues();
  const dirty = current.hotkey !== loadedSnapshot.hotkey ||
    current.api_url !== loadedSnapshot.api_url ||
    current.model !== loadedSnapshot.model ||
    current.language !== loadedSnapshot.language ||
    current.paste_mode !== loadedSnapshot.paste_mode ||
    current.post_paste_keys !== loadedSnapshot.post_paste_keys ||
    current.save_recordings !== loadedSnapshot.save_recordings ||
    current.hallucination_guard !== loadedSnapshot.hallucination_guard ||
    current.theme !== loadedSnapshot.theme ||
    current.input_device !== loadedSnapshot.input_device;

  saveBtn.disabled = !dirty;
  saveBtn.classList.toggle('disabled', !dirty);
}

['api-url'].forEach(id => {
  document.getElementById(id).addEventListener('input', checkDirty);
});
['model-select', 'language-select', 'paste-mode-select', 'post-paste-keys-select'].forEach(id => {
  document.getElementById(id).addEventListener('change', checkDirty);
});
document.getElementById('input-device-select').addEventListener('change', () => {
  checkDirty();
  if (window.MicTest) window.MicTest.refreshDevice();
});
document.getElementById('save-recordings').addEventListener('change', checkDirty);
document.getElementById('hallucination-guard').addEventListener('change', checkDirty);
document.querySelectorAll('input[name="theme-pref"]').forEach(input => {
  input.addEventListener('change', () => {
    const pref = getThemePref();
    if (window.SpeakTypeTheme) window.SpeakTypeTheme.applyTheme(pref);
    ttipc.emit('theme:changed', { theme: pref });
    checkDirty();
  });
});
document.getElementById('post-paste-keys-custom').addEventListener('input', checkDirty);

document.getElementById('post-paste-keys-custom').addEventListener('input', () => {
  if (document.getElementById('post-paste-keys-custom').value.trim()) {
    document.getElementById('post-paste-keys-select').value = '';
  }
});
document.getElementById('post-paste-keys-select').addEventListener('change', () => {
  if (document.getElementById('post-paste-keys-select').value) {
    document.getElementById('post-paste-keys-custom').value = '';
  }
});

async function populateInputDevices(selected) {
  const select = document.getElementById('input-device-select');
  if (!select) return;
  try {
    const data = await ttipc.getInputDevices();
    const devices = (data && data.devices) || [];
    // Preserve the current value unless an explicit stored selection was passed
    // in (e.g. on initial load), so background refreshes never discard a choice
    // the user has made but not yet saved.
    const current = select.value;
    select.innerHTML = '';
    const defaultOpt = document.createElement('option');
    defaultOpt.value = '';
    defaultOpt.textContent = 'System default (follows Display Audio settings)';
    select.appendChild(defaultOpt);
    devices.forEach((d) => {
      const opt = document.createElement('option');
      opt.value = d.name;
      opt.textContent = d.label + (d.is_default ? '  — current default' : '');
      select.appendChild(opt);
    });
    const hasValid = (v) => [...select.options].some(o => o.value === v);
    let value = current;
    if (value && !hasValid(value)) value = '';
    if (!value && selected) value = selected && hasValid(selected) ? selected : '';
    if (!value) value = (data.default && hasValid(data.default)) ? data.default : '';
    select.value = value;
  } catch (e) {
    console.error('Failed to load input devices:', e);
  }
}

async function loadSettings() {
  try {
    const settings = await ttipc.getSettings();
    if (!settings) return;

    setHotkeyDisplay(formatCapturedHotkey(settings.hotkey));
    document.getElementById('api-url').value = settings.api_url || '';

    populateLanguageSelect();
    const langSelect = document.getElementById('language-select');
    if (settings.language) {
      langSelect.value = settings.language === 'auto' ? 'auto' : settings.language;
    }

    populateModelSelect();
    const modelSelect = document.getElementById('model-select');
    if (settings.model) modelSelect.value = settings.model;

    const pasteModeSelect = document.getElementById('paste-mode-select');
    if (settings.paste_mode) pasteModeSelect.value = settings.paste_mode;

    const postPasteKeys = settings.post_paste_keys || '';
    const postPasteSelect = document.getElementById('post-paste-keys-select');
    const postPasteCustom = document.getElementById('post-paste-keys-custom');
    const presetOptions = [...postPasteSelect.options].map(o => o.value);
    if (presetOptions.includes(postPasteKeys)) {
      postPasteSelect.value = postPasteKeys;
      postPasteCustom.value = '';
    } else if (postPasteKeys) {
      postPasteSelect.value = '';
      postPasteCustom.value = postPasteKeys;
    }

    document.getElementById('save-recordings').checked = !!settings.save_recordings;
    document.getElementById('hallucination-guard').checked = settings.hallucination_guard !== false;

    await populateInputDevices(settings.input_device || '');

    const themePref = setThemePref(settings.theme);
    if (window.SpeakTypeTheme) window.SpeakTypeTheme.applyTheme(themePref);

    loadedSnapshot = {
      hotkey: settings.hotkey || '',
      api_url: settings.api_url || '',
      model: modelSelect.value,
      language: langSelect.value,
      paste_mode: pasteModeSelect.value,
      post_paste_keys: postPasteKeys,
      save_recordings: !!settings.save_recordings,
      hallucination_guard: settings.hallucination_guard !== false,
      theme: themePref,
      input_device: document.getElementById('input-device-select').value || '',
    };

    checkDirty();
    checkServer(settings.api_url);
    refreshServerControls();
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

function populateModelSelect() {
  const select = document.getElementById('model-select');
  if (!select) return;
  const models = [
    'tiny.en', 'tiny', 'base.en', 'base', 'small.en', 'small',
    'medium.en', 'medium', 'large-v3', 'large-v3-turbo',
  ];
  select.innerHTML = '';
  models.forEach(m => {
    const opt = document.createElement('option');
    opt.value = m;
    opt.textContent = m;
    select.appendChild(opt);
  });
}

const captureBtn = document.getElementById('capture-btn');
let capturing = false;
let capturedHotkey = '';
let superControlPending = false;

function setHotkeyDisplay(text) {
  const el = document.getElementById('current-hotkey');
  el.textContent = text || '(not set)';
}

function formatCapturedHotkey(hotkey) {
  return hotkey ? formatHotkey(hotkey) : '';
}

function initPresetLabels() {
  document.querySelectorAll('.preset').forEach((btn) => {
    btn.textContent = formatHotkey(btn.dataset.hotkey);
  });
}

initPresetLabels();

function finishHotkeyCapture(hotkey) {
  capturedHotkey = normalizeHotkey(hotkey);
  captureBtn.textContent = formatCapturedHotkey(capturedHotkey);
  captureBtn.classList.remove('capturing');
  capturing = false;
  superControlPending = false;
  setHotkeyDisplay(formatCapturedHotkey(capturedHotkey));
  checkDirty();
}

function updateCaptureFeedback(parts) {
  if (!capturing || parts.length === 0) return;
  if (isModifierChordHotkey(parts)) {
    captureBtn.textContent = formatHotkey(parts.join('+')) + ' (release to confirm)…';
    return;
  }
  if (parts.every(isModifierKey)) {
    captureBtn.textContent = formatHotkey(parts.join('+')) + ' + key…';
    return;
  }
  captureBtn.textContent = formatHotkey(parts.join('+')) + '…';
}

captureBtn.addEventListener('click', () => {
  if (capturing) {
    capturing = false;
    superControlPending = false;
    captureBtn.classList.remove('capturing');
    captureBtn.textContent = formatCapturedHotkey(capturedHotkey) || 'Click to capture...';
    return;
  }

  capturing = true;
  capturedHotkey = '';
  superControlPending = false;
  captureBtn.classList.add('capturing');
  captureBtn.textContent = 'Press keys now...';
});

document.addEventListener('keydown', (e) => {
  if (!capturing) return;

  if (e.key === 'Escape') {
    capturing = false;
    capturedHotkey = '';
    superControlPending = false;
    captureBtn.classList.remove('capturing');
    captureBtn.textContent = 'Click to capture...';
    return;
  }

  e.preventDefault();
  e.stopPropagation();

  const parts = captureHotkeyModifiers(e);

  if (e.metaKey && e.ctrlKey) {
    superControlPending = true;
    updateCaptureFeedback(['Super', 'Control']);
    return;
  }

  superControlPending = false;

  const modKeys = ['Control', 'Shift', 'Alt', 'Meta'];
  if (!modKeys.includes(e.key)) {
    const key = keyFromKeyboardEvent(e);
    if (key) {
      parts.push(key);

      if (isAcceptedHotkey(parts)) {
        finishHotkeyCapture(parts.join('+'));
        return;
      }
    }
  }

  updateCaptureFeedback(parts);
});

document.addEventListener('keyup', (e) => {
  if (!capturing || !superControlPending) return;

  e.preventDefault();
  e.stopPropagation();

  if (!e.metaKey && !e.ctrlKey) {
    finishHotkeyCapture('Super+Control');
  }
});

document.querySelectorAll('.preset').forEach(btn => {
  btn.addEventListener('click', () => {
    capturedHotkey = normalizeHotkey(btn.dataset.hotkey);
    setHotkeyDisplay(formatCapturedHotkey(capturedHotkey));
    captureBtn.textContent = formatCapturedHotkey(capturedHotkey);
    capturing = false;
    captureBtn.classList.remove('capturing');
    checkDirty();
  });
});

async function checkServer(url) {
  const dot = document.getElementById('server-status');
  dot.className = 'status-dot checking';
  dot.title = 'Checking...';

  try {
    const result = await ttipc.checkServer(url || document.getElementById('api-url').value);
    if (result && result.status === 'connected') {
      dot.className = 'status-dot connected';
      dot.title = 'Connected';
    } else {
      dot.className = 'status-dot disconnected';
      dot.title = (result && result.message) || 'Disconnected';
    }
    return result;
  } catch (e) {
    dot.className = 'status-dot disconnected';
    dot.title = 'Error: ' + e;
    return null;
  }
}

async function refreshServerControls() {
  const actions = document.getElementById('server-actions');
  const hint = document.getElementById('server-actions-hint');
  const stopBtn = document.getElementById('stop-server-btn');
  const restartBtn = document.getElementById('restart-server-btn');
  if (!actions || !stopBtn || !restartBtn) return;

  try {
    const status = await ttipc.getServerStatus();
    if (!status || !status.embedded) {
      actions.style.display = 'none';
      if (hint) {
        hint.textContent = 'Server controls are only available in builds with the bundled whisper sidecar.';
      }
      return;
    }

    actions.style.display = '';
    stopBtn.disabled = !status.running;
    restartBtn.disabled = false;
    if (hint) {
      hint.textContent = status.running
        ? `Whisper server running on port ${status.port}. Stop or restart the sidecar below.`
        : 'Whisper server is stopped. Restart to load the active model.';
    }
    await checkServer(document.getElementById('api-url').value);
  } catch (e) {
    console.error('Failed to get server status:', e);
  }
}

function setServerActionBusy(busy, label) {
  const stopBtn = document.getElementById('stop-server-btn');
  const restartBtn = document.getElementById('restart-server-btn');
  if (stopBtn) stopBtn.disabled = busy;
  if (restartBtn) {
    restartBtn.disabled = busy;
    restartBtn.textContent = busy && label ? label : 'Restart Server';
  }
}

document.getElementById('stop-server-btn').addEventListener('click', async () => {
  const stopBtn = document.getElementById('stop-server-btn');
  stopBtn.disabled = true;
  try {
    await ttipc.stopWhisperServer();
    await refreshServerControls();
  } catch (e) {
    console.error('Failed to stop server:', e);
    alert('Failed to stop server: ' + e);
    await refreshServerControls();
  }
});

document.getElementById('restart-server-btn').addEventListener('click', async () => {
  setServerActionBusy(true, 'Restarting…');
  try {
    const result = await ttipc.restartWhisperServer();
    if (result && result.api_url) {
      document.getElementById('api-url').value = result.api_url;
      if (loadedSnapshot) loadedSnapshot.api_url = result.api_url;
      checkDirty();
    }
    await refreshServerControls();
  } catch (e) {
    console.error('Failed to restart server:', e);
    alert('Failed to restart server: ' + e);
    await refreshServerControls();
  } finally {
    setServerActionBusy(false);
    await refreshServerControls();
  }
});

document.getElementById('open-models-folder-btn').addEventListener('click', async () => {
  try {
    await ttipc.openModelsFolder();
  } catch (e) {
    console.error('Failed to open models folder:', e);
    alert('Failed to open models folder: ' + e);
  }
});

document.getElementById('open-recordings-folder-btn').addEventListener('click', async () => {
  try {
    await ttipc.openRecordingsFolder();
  } catch (e) {
    console.error('Failed to open recordings folder:', e);
    alert('Failed to open recordings folder: ' + e);
  }
});

document.getElementById('check-server-btn').addEventListener('click', () => {
  checkServer(document.getElementById('api-url').value);
});

document.getElementById('reset-position-btn').addEventListener('click', async () => {
  try {
    await ttipc.resetWidgetPosition();
    const btn = document.getElementById('reset-position-btn');
    btn.textContent = 'Position Reset!';
    setTimeout(() => { btn.textContent = 'Reset Widget Position'; }, 1500);
  } catch (e) {
    console.error('Failed to reset position:', e);
  }
});

document.getElementById('restore-defaults-btn').addEventListener('click', () => {
  const defaults = {
    hotkey: 'Super+Control',
    api_url: 'http://127.0.0.1:8002/inference',
    model: 'medium',
    language: 'auto',
    paste_mode: 'active',
    post_paste_keys: 'enter',
    save_recordings: false,
    hallucination_guard: true,
    theme: 'auto',
  };

  capturedHotkey = defaults.hotkey;
  setHotkeyDisplay(formatCapturedHotkey(defaults.hotkey));
  captureBtn.textContent = formatCapturedHotkey(defaults.hotkey);
  document.getElementById('api-url').value = defaults.api_url;
  document.getElementById('model-select').value = defaults.model;
  document.getElementById('language-select').value = defaults.language;
  document.getElementById('paste-mode-select').value = defaults.paste_mode;
  document.getElementById('post-paste-keys-select').value = defaults.post_paste_keys;
  document.getElementById('post-paste-keys-custom').value = '';
  document.getElementById('save-recordings').checked = defaults.save_recordings;
  document.getElementById('hallucination-guard').checked = defaults.hallucination_guard;
  const devSelect = document.getElementById('input-device-select');
  if (devSelect) devSelect.value = '';
  setThemePref(defaults.theme);
  if (window.SpeakTypeTheme) window.SpeakTypeTheme.applyTheme(defaults.theme);

  checkDirty();
});

saveBtn.addEventListener('click', async () => {
  if (saveBtn.disabled) return;

  try {
    const settings = await ttipc.getSettings();
    if (!settings) {
      console.error('Could not load current settings');
      return;
    }

    if (capturedHotkey) {
      settings.hotkey = normalizeHotkey(capturedHotkey);
    }
    settings.api_url = document.getElementById('api-url').value;
    settings.model = document.getElementById('model-select').value;
    settings.language = document.getElementById('language-select').value;
    settings.paste_mode = document.getElementById('paste-mode-select').value;
    settings.post_paste_keys = getPostPasteKeys() || null;
    settings.save_recordings = document.getElementById('save-recordings').checked;
    settings.hallucination_guard = document.getElementById('hallucination-guard').checked;
    settings.theme = getThemePref();
    settings.input_device = document.getElementById('input-device-select').value || null;

    const modelChanged = settings.model !== loadedSnapshot.model;
    const hotkeyChanged = settings.hotkey !== loadedSnapshot.hotkey;

    await ttipc.saveSettings(settings);
    setHotkeyDisplay(formatCapturedHotkey(settings.hotkey));
    if (window.SpeakTypeTheme) window.SpeakTypeTheme.applyTheme(settings.theme);

    loadedSnapshot = {
      hotkey: settings.hotkey,
      api_url: settings.api_url,
      model: settings.model,
      language: settings.language,
      paste_mode: settings.paste_mode,
      post_paste_keys: settings.post_paste_keys || '',
      save_recordings: settings.save_recordings,
      hallucination_guard: settings.hallucination_guard,
      theme: settings.theme,
      input_device: settings.input_device || '',
    };
    capturedHotkey = '';
    checkDirty();

    if (modelChanged) {
      modelGrid.activate(settings.model);
    }

    saveBtn.textContent = hotkeyChanged ? 'Saved — hotkey active' : 'Saved!';
    saveBtn.style.background = '#16a34a';
    saveBtn.style.borderColor = '#16a34a';
    setTimeout(() => {
      saveBtn.textContent = 'Save';
      saveBtn.style.background = '';
      saveBtn.style.borderColor = '';
    }, 1500);
  } catch (e) {
    console.error('Failed to save settings:', e);
    saveBtn.textContent = 'Error!';
    saveBtn.style.background = '#dc2626';
    saveBtn.style.borderColor = '#dc2626';
    setTimeout(() => {
      saveBtn.textContent = 'Save';
      saveBtn.style.background = '';
      saveBtn.style.borderColor = '';
    }, 1500);
  }
});

let statusSpinner = null;

function setStatusLoading(text) {
  const el = document.getElementById('model-status');
  el.hidden = false;
  el.style.display = 'flex';
  el.className = 'model-status loading';
  if (!statusSpinner || !el.contains(statusSpinner)) {
    el.innerHTML = '<span class="spinner"></span><span class="status-text"></span>';
    statusSpinner = el.querySelector('.spinner');
  }
  el.querySelector('.status-text').textContent = text;
}

function showModelStatus(type, html) {
  const el = document.getElementById('model-status');
  el.hidden = false;
  el.style.display = 'flex';
  el.className = 'model-status ' + type;
  el.innerHTML = html;
  statusSpinner = null;
}

function clearModelStatusTimer() {
  if (modelStatusTimer) {
    clearTimeout(modelStatusTimer);
    modelStatusTimer = null;
  }
}

function hideModelStatus() {
  const el = document.getElementById('model-status');
  el.hidden = true;
  el.style.display = 'none';
  clearModelStatusTimer();
}

function scheduleHideModelStatus(ms) {
  clearModelStatusTimer();
  modelStatusTimer = setTimeout(hideModelStatus, ms);
}

const HISTORY_PAGE_SIZE = 30;
const HISTORY_COPY_ICON = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>';
const HISTORY_COPIED_ICON = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#4ade80" stroke-width="2.5"><path d="M4.5 12.75l6 6 9-13.5"/></svg>';
const HISTORY_DELETE_ICON = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"/></svg>';

let activeHistoryPlayer = null;
let historyEntries = [];
let historyRenderedCount = 0;
let historyObserver = null;
let historySentinel = null;
let historyListBound = false;
let armedHistoryBtn = null;
let armedHistoryTimer = null;
const historyPlayers = new Map();

function bindConfirmButton(button, { confirmLabel, onConfirm, armedClass = 'confirm-armed' }) {
  button.type = 'button';
  let confirmTimer = null;
  const defaultHtml = button.innerHTML;

  function restore() {
    if (confirmTimer) {
      clearTimeout(confirmTimer);
      confirmTimer = null;
    }
    button.dataset.armed = '0';
    button.innerHTML = defaultHtml;
    button.classList.remove(armedClass);
  }

  button.addEventListener('click', async (e) => {
    e.stopPropagation();
    e.preventDefault();

    if (button.dataset.armed !== '1') {
      button.dataset.armed = '1';
      button.textContent = confirmLabel;
      button.classList.add(armedClass);
      confirmTimer = setTimeout(restore, 3000);
      return;
    }

    restore();
    try {
      await onConfirm();
    } catch (err) {
      console.error(err);
      alert(String(err));
    }
  });

  return restore;
}

function stopActiveHistoryPlayer() {
  if (!activeHistoryPlayer) return;
  activeHistoryPlayer.audio.pause();
  activeHistoryPlayer.reset();
  activeHistoryPlayer = null;
}

function releaseHistoryPlayers() {
  stopActiveHistoryPlayer();
  historyPlayers.forEach(player => player.release());
  historyPlayers.clear();
}

function toAudioBytes(raw) {
  if (raw instanceof Uint8Array) return raw;
  if (raw instanceof ArrayBuffer) return new Uint8Array(raw);
  if (Array.isArray(raw)) return new Uint8Array(raw);
  return new Uint8Array(raw || []);
}

function createHistoryAudioPlayer(durationHint) {
  const wrap = document.createElement('div');
  wrap.className = 'history-audio-player';
  if (durationHint) wrap.dataset.durationHint = String(durationHint);

  const playBtn = document.createElement('button');
  playBtn.type = 'button';
  playBtn.className = 'history-play-btn';
  playBtn.title = 'Play recording';
  playBtn.dataset.action = 'play';
  playBtn.textContent = '▶';

  const seekbar = document.createElement('div');
  seekbar.className = 'history-seekbar';
  const seek = document.createElement('input');
  seek.type = 'range';
  seek.className = 'history-seek';
  seek.min = '0';
  seek.max = '1000';
  seek.value = '0';
  seek.disabled = true;
  seek.setAttribute('aria-label', 'Playback position');
  seekbar.appendChild(seek);

  const timeLabel = document.createElement('span');
  timeLabel.className = 'history-audio-time';
  const totalHint = durationHint ? formatDuration(durationHint) : '0:00';
  const curTime = document.createElement('span');
  curTime.className = 'history-time-cur';
  curTime.textContent = '0:00';
  const totalTime = document.createElement('span');
  totalTime.className = 'history-time-total';
  totalTime.textContent = ` / ${totalHint}`;
  timeLabel.appendChild(curTime);
  timeLabel.appendChild(totalTime);

  const audio = document.createElement('audio');
  audio.preload = 'metadata';

  wrap.appendChild(playBtn);
  wrap.appendChild(seekbar);
  wrap.appendChild(timeLabel);
  wrap.appendChild(audio);

  return wrap;
}

// Player state is built on first interaction so a long history only pays for
// the rows the user actually plays.
function getHistoryPlayer(wrap, index) {
  const cached = historyPlayers.get(index);
  if (cached) return cached;

  const playBtn = wrap.querySelector('.history-play-btn');
  const seek = wrap.querySelector('.history-seek');
  const timeLabel = wrap.querySelector('.history-audio-time');
  const curTime = wrap.querySelector('.history-time-cur');
  const totalTime = wrap.querySelector('.history-time-total');
  const audio = wrap.querySelector('audio');

  let objectUrl = null;
  let loaded = false;
  let loading = false;
  let duration = Number(wrap.dataset.durationHint) || 0;
  let seeking = false;

  function setProgressUI(frac) {
    seek.style.setProperty('--progress', `${Math.round(frac * 100)}%`);
  }

  function setPlayingUI(playing) {
    playBtn.textContent = playing ? '⏸' : '▶';
    playBtn.title = playing ? 'Pause recording' : 'Play recording';
    playBtn.classList.toggle('playing', playing);
  }

  function updateTimeUI() {
    const current = audio.currentTime || 0;
    const total = audio.duration && Number.isFinite(audio.duration) ? audio.duration : duration;
    if (total > 0) duration = total;
    if (!seeking && duration > 0) {
      setProgressUI(current / duration);
    }
    curTime.textContent = formatDuration(current);
    totalTime.textContent = ` / ${formatDuration(duration)}`;
  }

  function resetPlayerUI() {
    setPlayingUI(false);
    setProgressUI(0);
    updateTimeUI();
  }

  async function ensureLoaded() {
    if (loaded || loading) return;
    loading = true;
    playBtn.disabled = true;
    try {
      const raw = await ttipc.getHistoryAudio(index);
      const bytes = toAudioBytes(raw);
      objectUrl = URL.createObjectURL(new Blob([bytes], { type: 'audio/wav' }));
      audio.src = objectUrl;
      await new Promise((resolve, reject) => {
        const onReady = () => {
          audio.removeEventListener('loadedmetadata', onReady);
          audio.removeEventListener('error', onErr);
          resolve();
        };
        const onErr = () => {
          audio.removeEventListener('loadedmetadata', onReady);
          audio.removeEventListener('error', onErr);
          reject(new Error('Failed to load audio'));
        };
        audio.addEventListener('loadedmetadata', onReady);
        audio.addEventListener('error', onErr);
        audio.load();
      });
      loaded = true;
      seek.disabled = false;
    } finally {
      loading = false;
      playBtn.disabled = false;
    }
  }

  seek.addEventListener('input', () => {
    if (!loaded || !duration) return;
    seeking = true;
    const t = (Number(seek.value) / 1000) * duration;
    setProgressUI(Number(seek.value) / 1000);
    curTime.textContent = formatDuration(t);
  });

  seek.addEventListener('change', () => {
    if (!loaded || !duration) {
      seeking = false;
      return;
    }
    audio.currentTime = (Number(seek.value) / 1000) * duration;
    seeking = false;
    updateTimeUI();
  });

  audio.addEventListener('timeupdate', updateTimeUI);
  audio.addEventListener('loadedmetadata', () => {
    if (audio.duration && Number.isFinite(audio.duration)) {
      duration = audio.duration;
      updateTimeUI();
    }
  });
  audio.addEventListener('ended', () => {
    resetPlayerUI();
    if (activeHistoryPlayer === playerState) activeHistoryPlayer = null;
  });

  const playerState = {
    audio,
    ensureLoaded,
    setPlaying: setPlayingUI,
    reset: resetPlayerUI,
    release() {
      audio.pause();
      audio.removeAttribute('src');
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
        objectUrl = null;
      }
      loaded = false;
    },
  };

  historyPlayers.set(index, playerState);
  return playerState;
}

function createHistoryActionBtn(action, title, html, extraClass) {
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = 'history-action-btn' + (extraClass ? ' ' + extraClass : '');
  btn.title = title;
  btn.dataset.action = action;
  btn.innerHTML = html;
  return btn;
}

function createHistoryEntry(entry, index) {
  const div = document.createElement('div');
  div.className = 'history-entry';
  div.dataset.index = String(index);

  const header = document.createElement('div');
  header.className = 'history-header';

  const time = document.createElement('span');
  time.className = 'history-time';
  time.textContent = formatTimestamp(entry.timestamp);
  header.appendChild(time);

  if (entry.audio_file) {
    const audioTag = document.createElement('span');
    audioTag.className = 'history-audio-tag';
    audioTag.title = 'Audio saved';
    audioTag.textContent = entry.duration_secs
      ? formatDuration(entry.duration_secs)
      : 'audio';
    header.appendChild(audioTag);
  }

  const actions = document.createElement('span');
  actions.className = 'history-entry-actions';
  actions.appendChild(createHistoryActionBtn('copy', 'Copy', HISTORY_COPY_ICON));
  if (entry.audio_file) {
    actions.appendChild(createHistoryActionBtn('reprocess', 'Reprocess audio', '↻'));
    actions.appendChild(
      createHistoryActionBtn('delete-audio', 'Delete audio only (keep text)', '♪⌫', 'delete-audio')
    );
  }
  actions.appendChild(createHistoryActionBtn('delete', 'Delete entry and audio', HISTORY_DELETE_ICON, 'delete'));
  header.appendChild(actions);

  const text = document.createElement('div');
  text.className = 'history-text';
  text.textContent = entry.text;

  div.appendChild(header);
  div.appendChild(text);
  if (entry.audio_file) {
    div.appendChild(createHistoryAudioPlayer(entry.duration_secs));
  }
  return div;
}

function disarmHistoryButton() {
  if (armedHistoryTimer) {
    clearTimeout(armedHistoryTimer);
    armedHistoryTimer = null;
  }
  if (!armedHistoryBtn) return;
  armedHistoryBtn.innerHTML = armedHistoryBtn.dataset.defaultHtml || '';
  armedHistoryBtn.classList.remove('confirm-armed');
  armedHistoryBtn = null;
}

async function confirmHistoryAction(btn, confirmLabel, onConfirm) {
  if (armedHistoryBtn !== btn) {
    disarmHistoryButton();
    btn.dataset.defaultHtml = btn.innerHTML;
    btn.textContent = confirmLabel;
    btn.classList.add('confirm-armed');
    armedHistoryBtn = btn;
    armedHistoryTimer = setTimeout(disarmHistoryButton, 3000);
    return;
  }

  disarmHistoryButton();
  try {
    await onConfirm();
  } catch (err) {
    console.error(err);
    alert(String(err));
  }
}

function copyHistoryText(btn, text) {
  navigator.clipboard.writeText(text).then(() => {
    btn.innerHTML = HISTORY_COPIED_ICON;
    setTimeout(() => { btn.innerHTML = HISTORY_COPY_ICON; }, 1200);
  }).catch(() => {});
}

async function toggleHistoryPlayback(entryEl, index) {
  const wrap = entryEl.querySelector('.history-audio-player');
  if (!wrap) return;
  const player = getHistoryPlayer(wrap, index);
  try {
    await player.ensureLoaded();
    if (activeHistoryPlayer && activeHistoryPlayer !== player) {
      stopActiveHistoryPlayer();
    }
    if (player.audio.paused) {
      activeHistoryPlayer = player;
      await player.audio.play();
      player.setPlaying(true);
    } else {
      player.audio.pause();
      player.setPlaying(false);
      if (activeHistoryPlayer === player) activeHistoryPlayer = null;
    }
  } catch (e) {
    console.error('Failed to play history audio:', e);
    alert('Could not play audio: ' + e);
    player.reset();
  }
}

async function reprocessHistoryEntry(btn, index) {
  btn.disabled = true;
  btn.textContent = '…';
  try {
    await ttipc.reprocessHistoryEntry(index);
    await loadHistory();
  } catch (e) {
    console.error('Failed to reprocess:', e);
    alert('Reprocess failed: ' + e);
  }
}

function onHistoryListClick(e) {
  const btn = e.target.closest('[data-action]');
  if (!btn) return;
  const entryEl = btn.closest('.history-entry');
  if (!entryEl) return;
  const index = Number(entryEl.dataset.index);
  const entry = historyEntries[index];
  if (!entry) return;

  e.preventDefault();
  e.stopPropagation();

  if (armedHistoryBtn && armedHistoryBtn !== btn) disarmHistoryButton();

  switch (btn.dataset.action) {
    case 'copy':
      copyHistoryText(btn, entry.text);
      break;
    case 'play':
      toggleHistoryPlayback(entryEl, index);
      break;
    case 'reprocess':
      reprocessHistoryEntry(btn, index);
      break;
    case 'delete-audio':
      confirmHistoryAction(btn, 'Sure?', async () => {
        stopActiveHistoryPlayer();
        await ttipc.deleteHistoryAudio(index);
        await loadHistory();
      });
      break;
    case 'delete':
      confirmHistoryAction(btn, 'Delete?', async () => {
        stopActiveHistoryPlayer();
        await ttipc.deleteHistoryEntry(index);
        await loadHistory();
      });
      break;
  }
}

function ensureHistoryObserver() {
  if (historyObserver) return historyObserver;
  historyObserver = new IntersectionObserver((records) => {
    if (records.some(r => r.isIntersecting)) renderHistoryPage();
  }, { root: paneBody, rootMargin: '200px' });
  return historyObserver;
}

function renderHistoryPage() {
  const list = document.getElementById('history-list');
  const end = Math.min(historyRenderedCount + HISTORY_PAGE_SIZE, historyEntries.length);
  const fragment = document.createDocumentFragment();
  for (let i = historyRenderedCount; i < end; i++) {
    fragment.appendChild(createHistoryEntry(historyEntries[i], i));
  }
  list.insertBefore(fragment, historySentinel);
  historyRenderedCount = end;

  const observer = ensureHistoryObserver();
  observer.unobserve(historySentinel);
  if (historyRenderedCount >= historyEntries.length) {
    historySentinel.hidden = true;
    return;
  }
  // Re-observing re-fires the callback when the sentinel is still on screen,
  // so a tall window keeps paging until it is filled.
  observer.observe(historySentinel);
}

async function loadHistory() {
  releaseHistoryPlayers();
  disarmHistoryButton();

  const list = document.getElementById('history-list');
  const empty = document.getElementById('history-empty');

  if (!historyListBound) {
    list.addEventListener('click', onHistoryListClick);
    historyListBound = true;
  }
  if (!historySentinel) {
    historySentinel = document.createElement('div');
    historySentinel.className = 'history-sentinel';
    historySentinel.setAttribute('aria-hidden', 'true');
  }

  try {
    const history = await ttipc.getHistory();
    historyEntries = (history && history.entries) || [];
  } catch (e) {
    console.error('Failed to load history:', e);
    return;
  }

  ensureHistoryObserver().unobserve(historySentinel);
  list.querySelectorAll('.history-entry').forEach(el => el.remove());
  list.appendChild(historySentinel);
  historyRenderedCount = 0;

  if (historyEntries.length === 0) {
    empty.style.display = '';
    historySentinel.hidden = true;
    return;
  }

  empty.style.display = 'none';
  historySentinel.hidden = false;
  renderHistoryPage();
}

function formatTimestamp(ts) {
  try {
    const d = new Date(ts);
    return d.toLocaleString();
  } catch {
    return ts;
  }
}

bindConfirmButton(document.getElementById('clear-history-btn'), {
  confirmLabel: 'Clear all?',
  onConfirm: async () => {
    stopActiveHistoryPlayer();
    await ttipc.clearHistory();
    await loadHistory();
  },
});

/* ---- Debug panel ---- */

const debugList = document.getElementById('debug-list');
const debugEmpty = document.getElementById('debug-empty');

const DEBUG_MODE_LABEL = {
  live: 'Live (pipelined)',
  single_shot: 'Single-shot',
  live_pipelined: 'Live (chunked)',
  post_hoc: 'Fallback (post-hoc chunks)',
  post_hoc_fallback: 'Fallback (post-hoc chunks)',
  chunked: 'Chunked (no live session)',
  reprocess: 'Reprocess from history',
};

const DEBUG_STATUS_LABEL = {
  pipelined: 'queued',
  accepted: 'accepted',
  accepted_cleaned: 'accepted (cleaned)',
  rejected: 'rejected',
  failed: 'failed',
  silence: 'silence',
  info: 'info',
};

function escapeHtml(str) {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function debugChunkBadge(chunk) {
  const label = DEBUG_STATUS_LABEL[chunk.status] || chunk.status;
  return `<span class="debug-chip debug-chip-${chunk.status}">${escapeHtml(label)}</span>`;
}

function truncateText(text, limit) {
  if (!text) return '';
  return text.length > limit ? text.slice(0, limit) + '…' : text;
}

function debugTimeRange(startSecs, endSecs, baseStartSecs) {
  const base = baseStartSecs || 0;
  const a = (startSecs != null ? startSecs : 0) + base;
  const b = (endSecs != null ? endSecs : startSecs || 0) + base;
  return `${a.toFixed(1)}s–${b.toFixed(1)}s`;
}

/* ---- Debug audio playback ---- */

let debugAudio = null;
let activeWaveEl = null;

function resetActiveWave() {
  if (!activeWaveEl) return;
  const canvas = activeWaveEl.querySelector('.debug-chunk-wave canvas');
  if (canvas && canvas.__wave) drawWaveform(canvas, canvas.__wave.samples, 0);
  activeWaveEl = null;
}

function stopDebugPlayback() {
  if (debugAudio) {
    debugAudio.audio.pause();
    if (debugAudio.objectUrl) {
      URL.revokeObjectURL(debugAudio.objectUrl);
      debugAudio.objectUrl = null;
    }
    debugAudio = null;
  }
  document.querySelectorAll('.debug-play.active').forEach(el => el.classList.remove('active'));
  resetActiveWave();
}

/**
 * Play one slice of a debug session's recording. Passing the same `key` again
 * pauses the current clip. `seekSecs` (relative to the slice start) seeks
 * before playback starts. The returned promise resolves when playback starts
 * (or immediately if there is no audio for that range).
 */
function playDebugSlice(sessionId, startSecs, endSecs, key, btnEl, seekSecs) {
  if (debugAudio && debugAudio.key === key) {
    stopDebugPlayback();
    return Promise.resolve();
  }
  stopDebugPlayback();

  return ttipc
    .getDebugAudioSlice(sessionId, startSecs, endSecs)
    .then(raw => {
      const bytes = toAudioBytes(raw);
      if (!bytes.length) return;
      debugAudio = {
        key,
        audio: new Audio(),
        objectUrl: URL.createObjectURL(new Blob([bytes], { type: 'audio/wav' })),
      };
      debugAudio.audio.src = debugAudio.objectUrl;
      if (seekSecs > 0) {
        const applySeek = () => {
          try {
            debugAudio.audio.currentTime = Math.min(seekSecs, debugAudio.audio.duration || seekSecs);
          } catch (e) {}
        };
        if (debugAudio.audio.readyState >= 1) applySeek();
        else debugAudio.audio.addEventListener('loadedmetadata', applySeek, { once: true });
      }
      debugAudio.audio.play();
      debugAudio.audio.addEventListener('timeupdate', onDebugTimeUpdate);
      if (btnEl) btnEl.classList.add('active');
      if (btnEl && btnEl.classList.contains('debug-play-wave')) {
        debugAudio.audio.addEventListener('ended', () => {
          btnEl.classList.remove('active');
          resetActiveWave();
        });
      }
    })
    .catch(err => console.error('Failed to load debug audio slice:', err));
}

/* ---- Chunk waveform ---- */

/** Decode a mono 16-bit PCM WAV produced by the backend into raw samples + rate. */
function decodeWavSamples(bytes) {
  if (!bytes || bytes.byteLength <= 44) return { samples: [], rate: 16000 };
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const rate = dv.getUint32(24, true);
  const n = Math.floor((bytes.byteLength - 44) / 2);
  const samples = new Float32Array(n);
  for (let i = 0; i < n; i++) samples[i] = dv.getInt16(44 + i * 2, true) / 32768;
  return { samples, rate };
}

function drawWaveform(canvas, samples, progress) {
  const dpr = window.devicePixelRatio || 1;
  const w = canvas.clientWidth || canvas.offsetWidth || 320;
  const h = canvas.clientHeight || 44;
  const pxW = Math.round(w * dpr);
  const pxH = Math.round(h * dpr);
  if (canvas.width !== pxW || canvas.height !== pxH) {
    canvas.width = pxW;
    canvas.height = pxH;
  }
  const ctx = canvas.getContext('2d');
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);

  if (!samples || !samples.length) return;

  if (progress > 0 && progress < 1) {
    ctx.fillStyle = 'rgba(128, 128, 128, 0.25)';
    ctx.fillRect(0, 0, w * progress, h);
  }

  const color = getComputedStyle(canvas).color || 'rgba(128,128,128,0.7)';
  ctx.fillStyle = color;
  const mid = h / 2;
  const barCount = Math.max(20, Math.floor(w / 3));
  const per = Math.max(1, Math.floor(samples.length / barCount));
  const barW = Math.max(1, w / barCount - 1);
  for (let i = 0; i < barCount; i++) {
    const from = i * per;
    const to = Math.min(samples.length, from + per);
    let peak = 0;
    for (let j = from; j < to; j++) {
      const a = Math.abs(samples[j]);
      if (a > peak) peak = a;
    }
    const bh = Math.max(1, peak * h * 0.9);
    ctx.fillRect((w / barCount) * i, mid - bh / 2, barW, bh);
  }
}

function onDebugTimeUpdate() {
  if (!debugAudio || !activeWaveEl) return;
  const canvas = activeWaveEl.querySelector('.debug-chunk-wave canvas');
  if (!canvas || !canvas.__wave) return;
  const dur = debugAudio.audio.duration || canvas.__wave.duration || 1;
  const progress = dur > 0 ? debugAudio.audio.currentTime / dur : 0;
  drawWaveform(canvas, canvas.__wave.samples, Math.min(1, Math.max(0, progress)));
}

/** Fetch a chunk's audio slice, render its waveform, and wire click-to-seek. */
function wireChunkWaveform(chunkEl, sessionId) {
  const waveEl = chunkEl.querySelector('.debug-chunk-wave');
  const canvas = waveEl && waveEl.querySelector('canvas');
  if (!waveEl || !canvas || canvas.__waveLoaded) return;
  canvas.__waveLoaded = true;

  const startSecs = Number(chunkEl.dataset.start);
  const endSecs = Number(chunkEl.dataset.end);
  const key = `chunk:${sessionId}:${chunkEl.dataset.index}`;

  waveEl.classList.add('loading');
  ttipc.getDebugAudioSlice(sessionId, startSecs, endSecs)
    .then(raw => {
      const bytes = toAudioBytes(raw);
      const { samples, rate } = decodeWavSamples(bytes);
      canvas.__wave = {
        samples,
        duration: samples.length ? samples.length / rate : (endSecs - startSecs || 1),
      };
      drawWaveform(canvas, samples, 0);
    })
    .catch(err => console.error('Failed to load chunk waveform:', err))
    .finally(() => waveEl.classList.remove('loading'));

  canvas.addEventListener('click', (ev) => {
    ev.stopPropagation();
    const rect = canvas.getBoundingClientRect();
    const frac = Math.min(1, Math.max(0, (ev.clientX - rect.left) / rect.width));
    const seek = (canvas.__wave ? canvas.__wave.duration : (endSecs - startSecs)) * frac;
    activeWaveEl = chunkEl;
    playDebugSlice(sessionId, startSecs, endSecs, key, null, seek);
  });
}

/* ---- Debug chunk / pass rendering ---- */

function debugSegmentBadges(sessionId, chunkBaseSecs, segments) {
  return (segments || [])
    .filter(s => s.text && String(s.text).trim())
    .map(seg => {
      const start = (seg.start != null ? seg.start : 0) + chunkBaseSecs;
      const end = (seg.end != null ? seg.end : start) + chunkBaseSecs;
      const flags = (seg.flags || []).join(', ') || '';
      const label = truncateText(seg.text, 80);
      return `
        <div class="debug-seg ${seg.bad ? 'debug-seg-bad' : 'debug-seg-ok'}">
          <button type="button" class="debug-play debug-play-seg" draggable="false"
            title="Play this segment" data-session="${sessionId}" data-start="${start.toFixed(3)}"
            data-end="${end.toFixed(3)}" data-key="seg:${sessionId}:${start.toFixed(3)}:${end.toFixed(3)}">▶</button>
          <span class="debug-seg-time">${debugTimeRange(seg.start != null ? seg.start : 0, seg.end, chunkBaseSecs)}</span>
          <span class="debug-seg-text">${escapeHtml(label)}</span>
          ${flags ? `<em class="debug-seg-flags">(${escapeHtml(flags)})</em>` : ''}
        </div>`;
    })
    .join('');
}

function debugPlayableSegments(chunkBaseSecs, segments) {
  return (segments || [])
    .filter(s => s.text && String(s.text).trim())
    .map(seg => {
      const start = (seg.start != null ? seg.start : 0) + chunkBaseSecs;
      return {
        start,
        end: (seg.end != null ? seg.end : seg.start || 0) + chunkBaseSecs,
      };
    });
}

function debugPassEl(sessionId, chunkBaseSecs, pass) {
  return `
    <div class="debug-pass">
      <div class="debug-pass-head">
        <span class="debug-tag">pass ${escapeHtml(pass.pass)}</span>
        <span class="debug-tag">temp ${escapeHtml(pass.temperature)}</span>
        <span class="debug-tag">${pass.promptUsed ? 'with' : 'without'} prompt</span>
        ${pass.hadMetrics ? '<span class="debug-tag debug-tag-metrics">metrics</span>' : ''}
        <span class="debug-tag">score ${escapeHtml((pass.score != null ? pass.score : 0).toFixed(1))}</span>
        <span class="debug-tag${pass.dropped > 0 ? ' debug-tag-warn' : ''}">${escapeHtml(pass.dropped)} dropped</span>
        <span class="debug-tag">raw ${escapeHtml(pass.rawChars)} chars</span>
        <button type="button" class="debug-play debug-play-all" draggable="false"
          title="Play all kept segments in order" data-session="${sessionId}"
          data-play-segments="${escapeHtml(JSON.stringify(debugPlayableSegments(chunkBaseSecs, pass.segments)))}"
          >▶ all</button>
      </div>
      <div class="debug-pass-reason">${escapeHtml(pass.reason || 'ok')}</div>
      <div class="debug-segments">${debugSegmentBadges(sessionId, chunkBaseSecs, pass.segments)}</div>
    </div>`;
}

function debugChunkEl(session, chunk, passes) {
  const sessionId = session.id;
  const isSilence = chunk.status === 'silence';
  const chunkStart = chunk.startSecs || 0;
  const chunkEnd = chunk.endSecs || chunkStart;

  const titleText = isSilence
    ? 'silence'
    : `${escapeHtml(chunk.source)} · chunk ${escapeHtml(chunk.idx)}`;

  const head = `
    <div class="debug-chunk-head" role="button" tabindex="0" aria-expanded="false"
      aria-label="Toggle decode passes">
      <span class="debug-chunk-chevron" aria-hidden="true"></span>
      ${debugChunkBadge(chunk)}
      <span class="debug-chunk-title">${titleText}</span>
      ${chunk.chars ? `<span class="debug-chunk-chars">${escapeHtml(chunk.chars)} chars</span>` : ''}
      <span class="debug-chunk-range">${escapeHtml(chunk.durationSecs != null ? chunk.durationSecs.toFixed(1) : '?')}s · ${debugTimeRange(chunk.startSecs, chunk.endSecs, 0)}</span>
      ${chunk.seededPrompt ? '<span class="debug-chunk-seeded">seeded next prompt</span>' : ''}
      ${chunk.reason ? `<span class="debug-chunk-reason">${escapeHtml(chunk.reason)}</span>` : ''}
    </div>`;

  const passesHtml = isSilence
    ? '<div class="debug-chunk-passnote">Complete silence — skipped by the speech gate, no decode.</div>'
    : (passes || []).map(p => debugPassEl(sessionId, chunkStart, p)).join('')
        || '<div class="debug-chunk-passnote">No decode passes for this chunk.</div>';

  const waveHtml = `
    <div class="debug-chunk-wave">
      <canvas></canvas>
      <button type="button" class="debug-play debug-play-wave" draggable="false"
        title="${isSilence ? 'Play this silence range' : 'Play this chunk'}"
        data-session="${sessionId}" data-start="${chunkStart.toFixed(3)}"
        data-end="${chunkEnd.toFixed(3)}"
        data-key="chunk:${sessionId}:${escapeHtml(chunk.idx)}">▶</button>
    </div>`;

  return `
    <div class="debug-chunk" data-index="${escapeHtml(chunk.idx)}"
      data-start="${chunkStart.toFixed(3)}" data-end="${chunkEnd.toFixed(3)}">
      ${head}
      <div class="debug-chunk-body" hidden>${waveHtml}${passesHtml}</div>
    </div>`;
}

/**
 * Render one debug session as an accordion. Chunks are the top-level items;
 * each chunk expands to show its decode passes (with per-segment audio), and
 * any recording-wide passes appear under their own block.
 */
function createDebugSessionEl(session) {
  const el = document.createElement('div');
  el.className = 'debug-session';

  const head = document.createElement('div');
  head.className = 'debug-session-head';

  const toggle = document.createElement('button');
  toggle.type = 'button';
  toggle.className = 'debug-session-toggle';
  toggle.setAttribute('aria-expanded', 'false');
  toggle.innerHTML = `
    <span class="debug-session-title">
      <span class="debug-session-id">#${escapeHtml(session.id)}</span>
      <span class="debug-session-mode">${escapeHtml(DEBUG_MODE_LABEL[session.mode] || session.mode)}</span>
    </span>
    <span class="debug-session-meta">
      <span class="debug-session-time">${escapeHtml(session.startedAt || '')}</span>
      <span class="debug-tag">${escapeHtml(session.model || '?')}</span>
      <span class="debug-tag">${escapeHtml(session.durationSecs != null ? session.durationSecs.toFixed(1) : '?')}s audio</span>
      <span class="debug-tag">${escapeHtml(session.chunks ? session.chunks.length : 0)} chunks</span>
      <span class="debug-tag">${escapeHtml(session.elapsedMs != null ? (session.elapsedMs / 1000).toFixed(1) : '?')}s wall</span>
      <span class="debug-tag">${escapeHtml(session.finalChars != null ? session.finalChars : 0)} chars out</span>
      <span class="debug-caret" aria-hidden="true"></span>
    </span>
  `;

  const deleteBtn = document.createElement('button');
  deleteBtn.type = 'button';
  deleteBtn.className = 'debug-session-delete';
  deleteBtn.title = 'Delete this session';
  deleteBtn.setAttribute('aria-label', `Delete session #${escapeHtml(session.id)}`);
  deleteBtn.innerHTML =
    '<svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 6L6 18M6 6l12 12"/></svg>';
  deleteBtn.addEventListener('click', async (ev) => {
    ev.stopPropagation();
    ev.preventDefault();
    try {
      await ttipc.deleteDebugSession(session.id);
      el.remove();
      if (!debugList.querySelector('.debug-session')) {
        debugEmpty.style.display = '';
      }
    } catch (e) {
      console.error('Failed to delete debug session:', e);
    }
  });

  head.appendChild(toggle);
  head.appendChild(deleteBtn);

  const body = document.createElement('div');
  body.className = 'debug-session-body';
  body.hidden = true;
  body.id = `debug-body-${session.id}`;

  // Group passes by chunk so they can be nested under their chunk.
  const passesByChunk = new Map();
  const recordingPasses = [];
  (session.passes || []).forEach(p => {
    const cid = p.chunkIdx;
    if (cid === null || cid === undefined) {
      recordingPasses.push(p);
    } else {
      if (!passesByChunk.has(cid)) passesByChunk.set(cid, []);
      passesByChunk.get(cid).push(p);
    }
  });

  const chunks = session.chunks || [];
  const chunkHtml = chunks.length
    ? chunks.map(c => debugChunkEl(session, c, passesByChunk.get(c.idx) || [])).join('')
    : '<div class="debug-muted">No chunk events.</div>';

  const recordingHtml = recordingPasses.length
    ? `<div class="debug-section-label">Recording-level decode</div>
       ${recordingPasses.map(p => debugPassEl(session.id, 0, p)).join('')}`
    : '';

  body.innerHTML = `
    <div class="debug-section-label">Chunk timeline</div>
    ${chunkHtml}
    ${recordingHtml}
    <div class="debug-final">
      <div class="debug-section-label">Final text</div>
      <div class="debug-final-text">${escapeHtml(session.finalText || '')}</div>
    </div>
  `;

  el.appendChild(head);
  el.appendChild(body);

  toggle.addEventListener('click', () => {
    body.hidden = !body.hidden;
    el.classList.toggle('open', !body.hidden);
    toggle.setAttribute('aria-expanded', String(!body.hidden));
  });

  body.addEventListener('click', (ev) => {
    const playBtn = ev.target.closest('.debug-play');
    if (playBtn) {
      ev.stopPropagation();
      if (playBtn.classList.contains('debug-play-wave')) {
        activeWaveEl = playBtn.closest('.debug-chunk') || null;
      }
      if (playBtn.hasAttribute('data-play-segments')) {
        playDebugSegments(playBtn, session.id);
      } else {
        playDebugSlice(
          session.id,
          Number(playBtn.dataset.start),
          Number(playBtn.dataset.end),
          playBtn.dataset.key,
          playBtn
        );
      }
      return;
    }
    const head = ev.target.closest('.debug-chunk-head');
    if (head) {
      ev.stopPropagation();
      const chunkEl = head.closest('.debug-chunk');
      const chunkBody = chunkEl.querySelector('.debug-chunk-body');
      const wasHidden = chunkBody.hidden;
      chunkBody.hidden = !wasHidden;
      chunkEl.classList.toggle('open', wasHidden);
      head.setAttribute('aria-expanded', String(wasHidden));
      if (wasHidden) wireChunkWaveform(chunkEl, session.id);
    }
  });

  body.addEventListener('keydown', (ev) => {
    if (ev.key !== 'Enter' && ev.key !== ' ') return;
    const head = ev.target.closest('.debug-chunk-head');
    if (!head) return;
    ev.preventDefault();
    head.click();
  });

  return el;
}

/** Play the kept segments of one pass back-to-back. */
function playDebugSegments(btn, sessionId) {
  let segments = [];
  try {
    segments = JSON.parse(btn.dataset.playSegments || '[]');
  } catch (e) {
    return;
  }
  if (!segments.length) return;
  if (btn.dataset.playing === '1') {
    stopDebugPlayback();
    btn.dataset.playing = '';
    return;
  }

  btn.dataset.playing = '1';
  btn.classList.add('active');

  let index = 0;
  const playNext = () => {
    if (!btn.dataset.playing) return;
    const seg = segments[index++];
    if (!seg) {
      btn.dataset.playing = '';
      btn.classList.remove('active');
      return;
    }
    const key = `segs:${sessionId}:${index}:${seg.start.toFixed(3)}:${seg.end.toFixed(3)}`;
    playDebugSlice(sessionId, seg.start, seg.end, key, btn).then(() => {
      if (!btn.dataset.playing) return;
      if (!debugAudio) {
        playNext();
        return;
      }
      debugAudio.audio.addEventListener('ended', playNext, { once: true });
    });
  };
  playNext();
}

async function loadDebugSessions() {
  let sessions = [];
  try {
    sessions = await ttipc.getDebugSessions();
  } catch (e) {
    console.error('Failed to load debug sessions:', e);
    return;
  }
  sessions = sessions || [];

  debugList.querySelectorAll('.debug-session').forEach(el => el.remove());

  const empty = !sessions.length;
  debugEmpty.style.display = empty ? '' : 'none';
  if (empty) return;

  const fragment = document.createDocumentFragment();
  sessions.forEach(s => fragment.appendChild(createDebugSessionEl(s)));
  debugList.appendChild(fragment);
}

document.getElementById('refresh-debug-btn').addEventListener('click', loadDebugSessions);

bindConfirmButton(document.getElementById('clear-debug-btn'), {
  confirmLabel: 'Clear log?',
  onConfirm: async () => {
    await ttipc.clearDebugSessions();
    await loadDebugSessions();
  },
});

const PERMISSION_POLL_MS = 3000;

let permissionPollTimer = null;

async function checkPermissions() {
  const micEl = document.getElementById('perm-mic');
  const accEl = document.getElementById('perm-acc');
  const grantBtn = document.getElementById('request-acc-btn');
  const openMicBtn = document.getElementById('open-mic-btn');
  const openAccBtn = document.getElementById('open-acc-btn');

  try {
    const result = await ttipc.checkPermissions();
    if (!result) return;

    if (window.MicTest) MicTest.setPermission(!!result.microphone);
    // Refresh the available-input list (devices connect/disconnect, the user
    // may have switched the default in Sound settings, etc.).
    populateInputDevices();

    if (result.microphone) {
      micEl.textContent = 'Granted';
      micEl.className = 'permission-status granted';
      openMicBtn.style.display = 'none';
    } else {
      micEl.textContent = 'Not Granted';
      micEl.className = 'permission-status denied';
      openMicBtn.style.display = '';
    }

    if (result.accessibility) {
      accEl.textContent = 'Granted';
      accEl.className = 'permission-status granted';
      grantBtn.style.display = 'none';
      openAccBtn.style.display = 'none';
    } else {
      accEl.textContent = 'Not Granted';
      accEl.className = 'permission-status denied';
      grantBtn.style.display = '';
      openAccBtn.style.display = '';
    }
  } catch (e) {
    console.error('Failed to check permissions:', e);
  }
}

document.getElementById('open-mic-btn').addEventListener('click', () => {
  ttipc.openSystemPane('com.apple.preference.security?Privacy_Microphone');
});

document.getElementById('open-acc-btn').addEventListener('click', () => {
  ttipc.openSystemPane('com.apple.preference.security?Privacy_Accessibility');
});

document.getElementById('request-acc-btn').addEventListener('click', async () => {
  try {
    await ttipc.requestAccessibility();
    setTimeout(checkPermissions, 1000);
  } catch (e) {
    console.error('Failed to request accessibility:', e);
  }
});

// Polling only makes sense while the user can see the permission rows, so it is
// tied to the General section being visible and the window having focus.
function syncPermissionPoll() {
  const wanted = activeTab === 'general' && document.hasFocus() && !document.hidden;
  if (wanted && !permissionPollTimer) {
    permissionPollTimer = setInterval(checkPermissions, PERMISSION_POLL_MS);
  } else if (!wanted && permissionPollTimer) {
    clearInterval(permissionPollTimer);
    permissionPollTimer = null;
  }
}

window.addEventListener('focus', () => {
  if (activeTab === 'general') checkPermissions();
  syncPermissionPoll();
});
window.addEventListener('blur', syncPermissionPoll);
document.addEventListener('visibilitychange', syncPermissionPoll);

syncPermissionPoll();

document.getElementById('about-btn').addEventListener('click', () => {
  ttipc.openAbout().catch(e => console.error('Failed to open about:', e));
});

// Compact info-icon buttons that reveal a popover on click (close via Escape
// or clicking elsewhere). Any element with class "info-icon" and an
// aria-controls target gets this behavior.
(function initInfoPopovers() {
  const items = document.querySelectorAll('.info-icon[aria-controls]');

  const close = (icon) => {
    icon.setAttribute('aria-expanded', 'false');
    const pop = document.getElementById(icon.getAttribute('aria-controls'));
    if (pop) pop.hidden = true;
  };

  const open = (icon) => {
    items.forEach((other) => { if (other !== icon) close(other); });
    icon.setAttribute('aria-expanded', 'true');
    const pop = document.getElementById(icon.getAttribute('aria-controls'));
    if (pop) pop.hidden = false;
  };

  items.forEach((icon) => {
    icon.addEventListener('click', (e) => {
      e.stopPropagation();
      if (icon.getAttribute('aria-expanded') === 'true') close(icon);
      else open(icon);
    });
  });

  document.addEventListener('mousedown', (e) => {
    const inside = e.target.closest && e.target.closest('.info-icon, .info-popover');
    if (!inside) items.forEach(close);
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') items.forEach(close);
  });
})();

loadSettings();
modelGrid.ensureListeners();
modelGrid.refreshDownloadCount();
