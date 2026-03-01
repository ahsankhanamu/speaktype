// SpeakType Settings — Tab switching, hotkey capture, form handling

const tabs = document.querySelectorAll('.tab');
const tabContents = document.querySelectorAll('.tab-content');

// === Tab Switching ===
tabs.forEach(tab => {
  tab.addEventListener('click', () => {
    tabs.forEach(t => t.classList.remove('active'));
    tabContents.forEach(tc => tc.classList.remove('active'));
    tab.classList.add('active');
    document.getElementById(`tab-${tab.dataset.tab}`).classList.add('active');
  });
});

// === Load Settings ===
async function loadSettings() {
  try {
    const settings = await ttipc.getSettings();
    if (!settings) return;

    document.getElementById('current-hotkey').textContent = settings.hotkey || '(not set)';
    document.getElementById('api-url').value = settings.api_url || '';
    document.getElementById('python-path').value = settings.python_path || '';

    const modelSelect = document.getElementById('model-select');
    if (settings.model) modelSelect.value = settings.model;

    const langSelect = document.getElementById('language-select');
    if (settings.language) {
      langSelect.value = settings.language === 'auto' ? 'auto' : settings.language;
    }

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

// === Save ===
document.getElementById('save-btn').addEventListener('click', async () => {
  const btn = document.getElementById('save-btn');
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
    settings.python_path = document.getElementById('python-path').value;

    await ttipc.saveSettings(settings);
    document.getElementById('current-hotkey').textContent = settings.hotkey;

    // Visual feedback
    btn.textContent = 'Saved!';
    btn.style.background = '#16a34a';
    setTimeout(() => {
      btn.textContent = 'Save';
      btn.style.background = '';
    }, 1500);
  } catch (e) {
    console.error('Failed to save settings:', e);
    btn.textContent = 'Error!';
    btn.style.background = '#dc2626';
    setTimeout(() => {
      btn.textContent = 'Save';
      btn.style.background = '';
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

    entries.forEach(entry => {
      const div = document.createElement('div');
      div.className = 'history-entry';
      const time = document.createElement('div');
      time.className = 'history-time';
      time.textContent = formatTimestamp(entry.timestamp);
      const text = document.createElement('div');
      text.className = 'history-text';
      text.textContent = entry.text;
      div.appendChild(time);
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

// Load history when History tab is activated
tabs.forEach(tab => {
  tab.addEventListener('click', () => {
    if (tab.dataset.tab === 'history') {
      loadHistory();
    }
  });
});

// === Init ===
loadSettings();
