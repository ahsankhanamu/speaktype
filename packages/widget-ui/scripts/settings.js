const tabs = document.querySelectorAll('.tab');
const tabContents = document.querySelectorAll('.tab-content');
const paneTitle = document.getElementById('pane-title');
const paneDesc = document.getElementById('pane-desc');
const paneBody = document.querySelector('.pane-body');
const modelsDownloadBadge = document.getElementById('models-download-badge');

const actionsSettings = document.getElementById('actions-settings');
const actionsHistory = document.getElementById('actions-history');
const actionsModels = document.getElementById('actions-models');
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

  if (tabName === 'history') {
    actionsHistory.style.display = '';
    loadHistory();
  } else if (tabName === 'models') {
    actionsModels.style.display = '';
    modelGrid.ensureListeners();
    modelGrid.refresh(true);
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
    current.theme !== loadedSnapshot.theme;

  saveBtn.disabled = !dirty;
  saveBtn.classList.toggle('disabled', !dirty);
}

['api-url'].forEach(id => {
  document.getElementById(id).addEventListener('input', checkDirty);
});
['model-select', 'language-select', 'paste-mode-select', 'post-paste-keys-select'].forEach(id => {
  document.getElementById(id).addEventListener('change', checkDirty);
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

  const seek = document.createElement('input');
  seek.type = 'range';
  seek.className = 'history-seek';
  seek.min = '0';
  seek.max = '1000';
  seek.value = '0';
  seek.disabled = true;

  const timeLabel = document.createElement('span');
  timeLabel.className = 'history-audio-time';
  const totalHint = durationHint ? formatDuration(durationHint) : '0:00';
  timeLabel.textContent = `0:00 / ${totalHint}`;

  const audio = document.createElement('audio');
  audio.preload = 'metadata';

  wrap.appendChild(playBtn);
  wrap.appendChild(seek);
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
  const audio = wrap.querySelector('audio');

  let objectUrl = null;
  let loaded = false;
  let loading = false;
  let duration = Number(wrap.dataset.durationHint) || 0;
  let seeking = false;

  function setPlayingUI(playing) {
    playBtn.textContent = playing ? '⏸' : '▶';
    playBtn.title = playing ? 'Pause recording' : 'Play recording';
  }

  function updateTimeUI() {
    const current = audio.currentTime || 0;
    const total = audio.duration && Number.isFinite(audio.duration) ? audio.duration : duration;
    if (total > 0) duration = total;
    if (!seeking && duration > 0) {
      seek.value = String(Math.round((current / duration) * 1000));
    }
    timeLabel.textContent = `${formatDuration(current)} / ${formatDuration(duration)}`;
  }

  function resetPlayerUI() {
    setPlayingUI(false);
    seek.value = '0';
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
    timeLabel.textContent = `${formatDuration(t)} / ${formatDuration(duration)}`;
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

loadSettings();
modelGrid.ensureListeners();
modelGrid.refreshDownloadCount();
