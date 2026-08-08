const tabs = document.querySelectorAll('.tab');
const tabContents = document.querySelectorAll('.tab-content');

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

function populateLanguageSelect(filter) {
  const select = document.getElementById('language-select');
  const current = select.value || 'auto';
  select.innerHTML = '';
  const q = (filter || '').toLowerCase();
  WHISPER_LANGUAGES.forEach(lang => {
    if (q && !lang.name.toLowerCase().includes(q) && !lang.code.includes(q)) return;
    const opt = document.createElement('option');
    opt.value = lang.code;
    opt.textContent = lang.code === 'auto' ? lang.name : `${lang.name} (${lang.code})`;
    select.appendChild(opt);
  });
  if ([...select.options].some(o => o.value === current)) {
    select.value = current;
  } else {
    select.value = 'auto';
  }
}

let loadedSnapshot = {};
let modelStatusTimer = null;

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
  },
});

function switchTab(tabName) {
  tabs.forEach(t => t.classList.remove('active'));
  tabContents.forEach(tc => tc.classList.remove('active'));
  document.querySelector(`.tab[data-tab="${tabName}"]`).classList.add('active');
  document.getElementById(`tab-${tabName}`).classList.add('active');

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

    document.getElementById('language-search').addEventListener('input', (e) => {
      populateLanguageSelect(e.target.value);
    });

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

let activeHistoryPlayer = null;

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
  if (activeHistoryPlayer.onStop) activeHistoryPlayer.onStop();
  if (activeHistoryPlayer.objectUrl) {
    URL.revokeObjectURL(activeHistoryPlayer.objectUrl);
  }
  activeHistoryPlayer = null;
}

function toAudioBytes(raw) {
  if (raw instanceof Uint8Array) return raw;
  if (raw instanceof ArrayBuffer) return new Uint8Array(raw);
  if (Array.isArray(raw)) return new Uint8Array(raw);
  return new Uint8Array(raw || []);
}

function createHistoryAudioPlayer(index, durationHint) {
  const wrap = document.createElement('div');
  wrap.className = 'history-audio-player';

  const playBtn = document.createElement('button');
  playBtn.type = 'button';
  playBtn.className = 'history-play-btn';
  playBtn.title = 'Play recording';
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

  let objectUrl = null;
  let loaded = false;
  let loading = false;
  let duration = durationHint || 0;
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

  playBtn.addEventListener('click', async () => {
    try {
      if (!loaded) await ensureLoaded();
      if (activeHistoryPlayer && activeHistoryPlayer !== playerState) {
        stopActiveHistoryPlayer();
      }
      if (audio.paused) {
        activeHistoryPlayer = playerState;
        await audio.play();
        setPlayingUI(true);
      } else {
        audio.pause();
        setPlayingUI(false);
        if (activeHistoryPlayer === playerState) activeHistoryPlayer = null;
      }
    } catch (e) {
      console.error('Failed to play history audio:', e);
      alert('Could not play audio: ' + e);
      resetPlayerUI();
    }
  });

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
    get objectUrl() { return objectUrl; },
    onStop: resetPlayerUI,
  };

  return wrap;
}

async function loadHistory() {
  stopActiveHistoryPlayer();
  const list = document.getElementById('history-list');
  const empty = document.getElementById('history-empty');
  try {
    const history = await ttipc.getHistory();
    const entries = (history && history.entries) || [];

    list.querySelectorAll('.history-entry').forEach(el => el.remove());

    if (entries.length === 0) {
      empty.style.display = '';
      return;
    }
    empty.style.display = 'none';

    entries.forEach((entry, index) => {
      const div = document.createElement('div');
      div.className = 'history-entry';

      const header = document.createElement('div');
      header.className = 'history-header';
      const time = document.createElement('span');
      time.className = 'history-time';
      time.textContent = formatTimestamp(entry.timestamp);
      const actions = document.createElement('span');
      actions.className = 'history-entry-actions';

      const copyBtn = document.createElement('button');
      copyBtn.type = 'button';
      copyBtn.className = 'history-action-btn';
      copyBtn.title = 'Copy';
      copyBtn.innerHTML = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>';
      copyBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        navigator.clipboard.writeText(entry.text).then(() => {
          copyBtn.innerHTML = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#4ade80" stroke-width="2.5"><path d="M4.5 12.75l6 6 9-13.5"/></svg>';
          setTimeout(() => {
            copyBtn.innerHTML = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>';
          }, 1200);
        }).catch(() => {});
      });

      if (entry.audio_file) {
        const reprocessBtn = document.createElement('button');
        reprocessBtn.type = 'button';
        reprocessBtn.className = 'history-action-btn';
        reprocessBtn.title = 'Reprocess audio';
        reprocessBtn.textContent = '↻';
        reprocessBtn.addEventListener('click', async (e) => {
          e.stopPropagation();
          reprocessBtn.disabled = true;
          reprocessBtn.textContent = '…';
          try {
            await ttipc.reprocessHistoryEntry(index);
            await loadHistory();
          } catch (e) {
            console.error('Failed to reprocess:', e);
            alert('Reprocess failed: ' + e);
          }
        });

        const deleteAudioBtn = document.createElement('button');
        deleteAudioBtn.className = 'history-action-btn delete-audio';
        deleteAudioBtn.title = 'Delete audio only (keep text)';
        deleteAudioBtn.textContent = '♪⌫';
        bindConfirmButton(deleteAudioBtn, {
          confirmLabel: 'Sure?',
          onConfirm: async () => {
            stopActiveHistoryPlayer();
            await ttipc.deleteHistoryAudio(index);
            await loadHistory();
          },
        });

        actions.appendChild(copyBtn);
        actions.appendChild(reprocessBtn);
        actions.appendChild(deleteAudioBtn);
      } else {
        actions.appendChild(copyBtn);
      }

      const deleteBtn = document.createElement('button');
      deleteBtn.className = 'history-action-btn delete';
      deleteBtn.title = 'Delete entry and audio';
      deleteBtn.innerHTML = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"/></svg>';
      bindConfirmButton(deleteBtn, {
        confirmLabel: 'Delete?',
        onConfirm: async () => {
          stopActiveHistoryPlayer();
          await ttipc.deleteHistoryEntry(index);
          await loadHistory();
        },
      });

      actions.appendChild(deleteBtn);
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
      header.appendChild(actions);

      const text = document.createElement('div');
      text.className = 'history-text';
      text.textContent = entry.text;

      div.appendChild(header);
      div.appendChild(text);
      if (entry.audio_file) {
        div.appendChild(createHistoryAudioPlayer(index, entry.duration_secs));
      }
      list.appendChild(div);
    });
  } catch (e) {
    console.error('Failed to load history:', e);
  }
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

async function checkPermissions() {
  const micEl = document.getElementById('perm-mic');
  const accEl = document.getElementById('perm-acc');
  const grantBtn = document.getElementById('request-acc-btn');
  const openMicBtn = document.getElementById('open-mic-btn');
  const openAccBtn = document.getElementById('open-acc-btn');

  try {
    const result = await ttipc.checkPermissions();
    if (!result) return;

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

setInterval(checkPermissions, 3000);

document.getElementById('about-btn').addEventListener('click', () => {
  ttipc.openAbout().catch(e => console.error('Failed to open about:', e));
});

loadSettings();
modelGrid.ensureListeners();
