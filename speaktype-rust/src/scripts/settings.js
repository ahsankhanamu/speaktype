// SpeakType Settings — Tab switching, hotkey capture, form handling

const tabs = document.querySelectorAll('.tab');
const tabContents = document.querySelectorAll('.tab-content');

const actionsSettings = document.getElementById('actions-settings');
const actionsHistory = document.getElementById('actions-history');
const saveBtn = document.getElementById('save-btn');

// Snapshot of loaded settings for dirty detection
let loadedSnapshot = {};

// === Tab Switching ===
function switchTab(tabName) {
  tabs.forEach(t => t.classList.remove('active'));
  tabContents.forEach(tc => tc.classList.remove('active'));
  document.querySelector(`.tab[data-tab="${tabName}"]`).classList.add('active');
  document.getElementById(`tab-${tabName}`).classList.add('active');

  // Show the right actions bar
  if (tabName === 'history') {
    actionsSettings.style.display = 'none';
    actionsHistory.style.display = '';
    loadHistory();
  } else {
    actionsSettings.style.display = '';
    actionsHistory.style.display = 'none';
    if (tabName === 'general') checkPermissions();
  }
}

tabs.forEach(tab => {
  tab.addEventListener('click', () => switchTab(tab.dataset.tab));
});

// === Dirty Detection ===
function getFormValues() {
  return {
    hotkey: capturedHotkey || loadedSnapshot.hotkey || '',
    api_url: document.getElementById('api-url').value,
    model: document.getElementById('model-select').value,
    language: document.getElementById('language-select').value,
    paste_mode: document.getElementById('paste-mode-select').value,
  };
}

function checkDirty() {
  const current = getFormValues();
  const dirty = current.hotkey !== loadedSnapshot.hotkey ||
    current.api_url !== loadedSnapshot.api_url ||
    current.model !== loadedSnapshot.model ||
    current.language !== loadedSnapshot.language ||
    current.paste_mode !== loadedSnapshot.paste_mode;

  saveBtn.disabled = !dirty;
  saveBtn.classList.toggle('disabled', !dirty);
}

// Listen for changes on all form inputs
['api-url'].forEach(id => {
  document.getElementById(id).addEventListener('input', checkDirty);
});
['model-select', 'language-select', 'paste-mode-select'].forEach(id => {
  document.getElementById(id).addEventListener('change', checkDirty);
});

// === Load Settings ===
async function loadSettings() {
  try {
    const settings = await ttipc.getSettings();
    if (!settings) return;

    document.getElementById('current-hotkey').textContent = settings.hotkey || '(not set)';
    document.getElementById('api-url').value = settings.api_url || '';

    const modelSelect = document.getElementById('model-select');
    if (settings.model) modelSelect.value = settings.model;

    const langSelect = document.getElementById('language-select');
    if (settings.language) {
      langSelect.value = settings.language === 'auto' ? 'auto' : settings.language;
    }

    const pasteModeSelect = document.getElementById('paste-mode-select');
    if (settings.paste_mode) pasteModeSelect.value = settings.paste_mode;

    // Store snapshot for dirty detection
    loadedSnapshot = {
      hotkey: settings.hotkey || '',
      api_url: settings.api_url || '',
      model: modelSelect.value,
      language: langSelect.value,
      paste_mode: pasteModeSelect.value,
    };

    checkDirty();
    checkServer(settings.api_url);
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

// === Hotkey Capture ===
const captureBtn = document.getElementById('capture-btn');
let capturing = false;
let capturedHotkey = '';

captureBtn.addEventListener('click', () => {
  if (capturing) {
    // Stop capturing
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
  e.preventDefault();
  e.stopPropagation();

  const parts = [];
  if (e.metaKey || e.ctrlKey) parts.push('CmdOrCtrl');
  if (e.shiftKey) parts.push('Shift');
  if (e.altKey) parts.push('Alt');

  // Add the actual key (skip if just a modifier)
  const modKeys = ['Control', 'Shift', 'Alt', 'Meta'];
  if (!modKeys.includes(e.key)) {
    // Normalize key names for Tauri accelerator format
    let key = e.key;
    if (key === ' ') key = 'Space';
    else if (key.length === 1) key = key.toUpperCase();
    // F-keys are already like "F9"
    parts.push(key);

    // Got a full combo (modifier + key) — auto-stop capturing
    if (parts.length >= 2) {
      capturedHotkey = parts.join('+');
      captureBtn.textContent = capturedHotkey;
      captureBtn.classList.remove('capturing');
      capturing = false;
      // Update display immediately
      document.getElementById('current-hotkey').textContent = capturedHotkey;
      checkDirty();
    }
  }

  // Show partial combo while still capturing
  if (capturing && parts.length > 0) {
    captureBtn.textContent = parts.join('+') + '...';
  }
});

// Preset buttons
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

// === Server Check ===
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

// === Reset Position ===
document.getElementById('reset-position-btn').addEventListener('click', async () => {
  try {
    const settings = await ttipc.getSettings();
    if (!settings) return;
    settings.window_x = null;
    settings.window_y = null;
    await ttipc.saveSettings(settings);
    const btn = document.getElementById('reset-position-btn');
    btn.textContent = 'Position Reset!';
    setTimeout(() => { btn.textContent = 'Reset Widget Position'; }, 1500);
  } catch (e) {
    console.error('Failed to reset position:', e);
  }
});

// === Restore Defaults ===
document.getElementById('restore-defaults-btn').addEventListener('click', () => {
  const defaults = {
    hotkey: 'CmdOrCtrl+Alt+L',
    api_url: 'http://localhost:8002/transcribe',
    model: 'base',
    language: 'auto',
    paste_mode: 'original',
  };

  capturedHotkey = defaults.hotkey;
  document.getElementById('current-hotkey').textContent = defaults.hotkey;
  captureBtn.textContent = defaults.hotkey;
  document.getElementById('api-url').value = defaults.api_url;
  document.getElementById('model-select').value = defaults.model;
  document.getElementById('language-select').value = defaults.language;
  document.getElementById('paste-mode-select').value = defaults.paste_mode;

  checkDirty();
});

// === Save ===
saveBtn.addEventListener('click', async () => {
  if (saveBtn.disabled) return;

  try {
    const settings = await ttipc.getSettings();
    if (!settings) {
      console.error('Could not load current settings');
      return;
    }

    // Update from form
    if (capturedHotkey) {
      settings.hotkey = capturedHotkey;
    }
    settings.api_url = document.getElementById('api-url').value;
    settings.model = document.getElementById('model-select').value;
    settings.language = document.getElementById('language-select').value;
    settings.paste_mode = document.getElementById('paste-mode-select').value;

    await ttipc.saveSettings(settings);
    document.getElementById('current-hotkey').textContent = settings.hotkey;

    // Update snapshot so button goes back to disabled
    loadedSnapshot = {
      hotkey: settings.hotkey,
      api_url: settings.api_url,
      model: settings.model,
      language: settings.language,
      paste_mode: settings.paste_mode,
    };
    capturedHotkey = '';
    checkDirty();

    // Visual feedback
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

// === History ===
async function loadHistory() {
  const list = document.getElementById('history-list');
  const empty = document.getElementById('history-empty');
  try {
    const history = await ttipc.getHistory();
    const entries = (history && history.entries) || [];

    // Clear existing entries (keep empty placeholder)
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
        });
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

// === Permissions ===
async function checkPermissions() {
  const micEl = document.getElementById('perm-mic');
  const accEl = document.getElementById('perm-acc');
  const grantBtn = document.getElementById('request-acc-btn');

  try {
    const result = await ttipc.checkPermissions();
    if (!result) return;

    if (result.microphone) {
      micEl.textContent = 'Granted';
      micEl.className = 'permission-status granted';
    } else {
      micEl.textContent = 'Not Granted';
      micEl.className = 'permission-status denied';
    }

    if (result.accessibility) {
      accEl.textContent = 'Granted';
      accEl.className = 'permission-status granted';
      grantBtn.style.display = 'none';
    } else {
      accEl.textContent = 'Not Granted';
      accEl.className = 'permission-status denied';
      grantBtn.style.display = '';
    }
  } catch (e) {
    console.error('Failed to check permissions:', e);
  }
}

document.getElementById('request-acc-btn').addEventListener('click', async () => {
  try {
    await ttipc.requestAccessibility();
    // Re-check after a short delay (user may need to approve in System Settings)
    setTimeout(checkPermissions, 1000);
  } catch (e) {
    console.error('Failed to request accessibility:', e);
  }
});

// === About ===
document.getElementById('about-btn').addEventListener('click', () => {
  ttipc.openAbout().catch(e => console.error('Failed to open about:', e));
});

// === Init ===
loadSettings();
