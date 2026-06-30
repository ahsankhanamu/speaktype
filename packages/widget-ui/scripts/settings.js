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

function getFormValues() {
  return {
    hotkey: capturedHotkey || loadedSnapshot.hotkey || '',
    api_url: document.getElementById('api-url').value,
    model: document.getElementById('model-select').value,
    language: document.getElementById('language-select').value,
    paste_mode: document.getElementById('paste-mode-select').value,
    post_paste_keys: getPostPasteKeys(),
  };
}

function checkDirty() {
  const current = getFormValues();
  const dirty = current.hotkey !== loadedSnapshot.hotkey ||
    current.api_url !== loadedSnapshot.api_url ||
    current.model !== loadedSnapshot.model ||
    current.language !== loadedSnapshot.language ||
    current.paste_mode !== loadedSnapshot.paste_mode ||
    current.post_paste_keys !== loadedSnapshot.post_paste_keys;

  saveBtn.disabled = !dirty;
  saveBtn.classList.toggle('disabled', !dirty);
}

['api-url'].forEach(id => {
  document.getElementById(id).addEventListener('input', checkDirty);
});
['model-select', 'language-select', 'paste-mode-select', 'post-paste-keys-select'].forEach(id => {
  document.getElementById(id).addEventListener('change', checkDirty);
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

    document.getElementById('current-hotkey').textContent = settings.hotkey || '(not set)';
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

    loadedSnapshot = {
      hotkey: settings.hotkey || '',
      api_url: settings.api_url || '',
      model: modelSelect.value,
      language: langSelect.value,
      paste_mode: pasteModeSelect.value,
      post_paste_keys: postPasteKeys,
    };

    checkDirty();
    checkServer(settings.api_url);
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

captureBtn.addEventListener('click', () => {
  if (capturing) {
    capturing = false;
    captureBtn.classList.remove('capturing');
    captureBtn.textContent = capturedHotkey || 'Click to capture...';
    return;
  }

  capturing = true;
  capturedHotkey = '';
  captureBtn.classList.add('capturing');
  captureBtn.textContent = 'Press keys now...';
});

document.addEventListener('keydown', (e) => {
  if (!capturing) return;

  if (e.key === 'Escape') {
    capturing = false;
    capturedHotkey = '';
    captureBtn.classList.remove('capturing');
    captureBtn.textContent = 'Click to capture...';
    return;
  }

  e.preventDefault();
  e.stopPropagation();

  const parts = [];
  if (e.metaKey || e.ctrlKey) parts.push('CmdOrCtrl');
  if (e.shiftKey) parts.push('Shift');
  if (e.altKey) parts.push('Alt');

  const modKeys = ['Control', 'Shift', 'Alt', 'Meta'];
  if (!modKeys.includes(e.key)) {
    let key = e.key;
    if (key === ' ') key = 'Space';
    else if (key.length === 1) key = key.toUpperCase();
    parts.push(key);

    if (parts.length >= 2) {
      capturedHotkey = parts.join('+');
      captureBtn.textContent = capturedHotkey;
      captureBtn.classList.remove('capturing');
      capturing = false;
      document.getElementById('current-hotkey').textContent = capturedHotkey;
      checkDirty();
    }
  }

  if (capturing && parts.length > 0) {
    captureBtn.textContent = parts.join('+') + '...';
  }
});

document.querySelectorAll('.preset').forEach(btn => {
  btn.addEventListener('click', () => {
    capturedHotkey = btn.dataset.hotkey;
    document.getElementById('current-hotkey').textContent = capturedHotkey;
    captureBtn.textContent = capturedHotkey;
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
  } catch (e) {
    dot.className = 'status-dot disconnected';
    dot.title = 'Error: ' + e;
  }
}

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
    hotkey: 'CmdOrCtrl+Alt+L',
    api_url: 'http://localhost:8002/transcribe',
    model: 'medium',
    language: 'auto',
    paste_mode: 'original',
    post_paste_keys: '',
  };

  capturedHotkey = defaults.hotkey;
  document.getElementById('current-hotkey').textContent = defaults.hotkey;
  captureBtn.textContent = defaults.hotkey;
  document.getElementById('api-url').value = defaults.api_url;
  document.getElementById('model-select').value = defaults.model;
  document.getElementById('language-select').value = defaults.language;
  document.getElementById('paste-mode-select').value = defaults.paste_mode;
  document.getElementById('post-paste-keys-select').value = defaults.post_paste_keys;
  document.getElementById('post-paste-keys-custom').value = '';

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
      settings.hotkey = capturedHotkey;
    }
    settings.api_url = document.getElementById('api-url').value;
    settings.model = document.getElementById('model-select').value;
    settings.language = document.getElementById('language-select').value;
    settings.paste_mode = document.getElementById('paste-mode-select').value;
    settings.post_paste_keys = getPostPasteKeys() || null;

    const modelChanged = settings.model !== loadedSnapshot.model;

    await ttipc.saveSettings(settings);
    document.getElementById('current-hotkey').textContent = settings.hotkey;

    loadedSnapshot = {
      hotkey: settings.hotkey,
      api_url: settings.api_url,
      model: settings.model,
      language: settings.language,
      paste_mode: settings.paste_mode,
      post_paste_keys: settings.post_paste_keys || '',
    };
    capturedHotkey = '';
    checkDirty();

    if (modelChanged) {
      modelGrid.activate(settings.model);
    }

    saveBtn.textContent = 'Saved!';
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

async function loadHistory() {
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
      copyBtn.className = 'history-action-btn';
      copyBtn.title = 'Copy';
      copyBtn.innerHTML = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>';
      copyBtn.addEventListener('click', () => {
        navigator.clipboard.writeText(entry.text).then(() => {
          copyBtn.innerHTML = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#4ade80" stroke-width="2.5"><path d="M4.5 12.75l6 6 9-13.5"/></svg>';
          setTimeout(() => {
            copyBtn.innerHTML = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>';
          }, 1200);
        }).catch(() => {});
      });

      const deleteBtn = document.createElement('button');
      deleteBtn.className = 'history-action-btn delete';
      deleteBtn.title = 'Delete';
      deleteBtn.innerHTML = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"/></svg>';
      deleteBtn.addEventListener('click', async () => {
        try {
          await ttipc.deleteHistoryEntry(index);
          await loadHistory();
        } catch (e) {
          console.error('Failed to delete entry:', e);
        }
      });

      actions.appendChild(copyBtn);
      actions.appendChild(deleteBtn);
      header.appendChild(time);
      header.appendChild(actions);

      const text = document.createElement('div');
      text.className = 'history-text';
      text.textContent = entry.text;

      div.appendChild(header);
      div.appendChild(text);
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

document.getElementById('clear-history-btn').addEventListener('click', async () => {
  try {
    await ttipc.clearHistory();
    await loadHistory();
  } catch (e) {
    console.error('Failed to clear history:', e);
  }
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
