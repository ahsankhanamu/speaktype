// SpeakType Widget — Simplified: click to toggle, global shortcut for drag-free use

const widget = document.getElementById('widget');
const timer = document.querySelector('.timer');
const bars = document.querySelectorAll('.bar');

let currentState = 'idle';
let timerInterval = null;
let recordingStart = 0;

// Debug: pipe webview logs to Rust stdout AND show on page
const _dbg = document.createElement('div');
_dbg.style.cssText = 'position:fixed;top:70px;left:0;width:300px;max-height:200px;overflow:auto;background:black;color:lime;font:10px monospace;padding:4px;z-index:9999;opacity:0.9;pointer-events:none;';
document.body.appendChild(_dbg);
function dlog(msg) {
  console.log(msg);
  _dbg.textContent += msg + '\n';
  _dbg.scrollTop = _dbg.scrollHeight;
  ipc.invoke('debug_log', { msg }).catch(() => {});
}

function setState(s) {
  currentState = s;
  widget.className = 'widget ' + s;
  // Resize window for recording pill
  ipc.invoke('plugin:window|set_size', {
    label: 'main',
    value: { type: 'Logical', data: { width: s === 'recording' ? 160 : 68, height: 68 } },
  }).catch(() => {});
}

function startTimer() {
  recordingStart = Date.now();
  timer.textContent = '0.0s';
  timerInterval = setInterval(() => {
    timer.textContent = ((Date.now() - recordingStart) / 1000).toFixed(1) + 's';
  }, 100);
}

function stopTimer() {
  clearInterval(timerInterval);
  timerInterval = null;
}

function updateWaveform(level) {
  const s = Math.min(level * 15, 1);
  bars.forEach(b => { b.style.height = Math.round(6 + 22 * s * (0.6 + Math.random() * 0.8)) + 'px'; });
}

function flashState(s, ms) {
  setState(s);
  setTimeout(() => setState('idle'), ms || 1500);
}

// === CLICK TO TOGGLE ===
widget.addEventListener('mousedown', () => dlog('[mousedown] fired'));
widget.addEventListener('mouseup', () => dlog('[mouseup] fired'));
widget.addEventListener('click', () => {
  dlog('[click] state=' + currentState);
  dlog('[click] tauri=' + !!window.__TAURI_INTERNALS__);
  if (currentState === 'idle') {
    dlog('[click] calling toggleRecording(false) to START');
    ipc.toggleRecording(false)
      .then((r) => dlog('[click] start sent OK'))
      .catch(e => dlog('[click] start error: ' + e));
  } else if (currentState === 'recording') {
    dlog('[click] calling toggleRecording(true) to STOP');
    ipc.toggleRecording(true)
      .then((r) => dlog('[click] stop sent OK'))
      .catch(e => dlog('[click] stop error: ' + e));
  }
});

// Right-click → settings
widget.addEventListener('contextmenu', (e) => {
  e.preventDefault();
  ipc.openSettings().catch(() => {});
});

// === SIDECAR EVENTS ===
if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.listen) {
  const L = (ev, fn) => ipc.listen(ev, fn);

  L('sidecar:ready', () => console.log('[sidecar] ready'));

  L('sidecar:recording_started', () => {
    console.log('[sidecar] recording_started');
    setState('recording');
    startTimer();
  });

  L('sidecar:audio_level', (e) => {
    if (currentState !== 'recording') return;
    const d = typeof e.payload === 'string' ? JSON.parse(e.payload) : e.payload;
    updateWaveform(d.level || 0);
  });

  L('sidecar:recording_stopped', () => { console.log('[sidecar] stopped'); stopTimer(); });

  L('sidecar:transcribing', () => {
    console.log('[sidecar] transcribing');
    stopTimer();
    bars.forEach(b => b.style.height = '6px');
    setState('transcribing');
  });

  L('sidecar:pasted', (e) => {
    const d = typeof e.payload === 'string' ? JSON.parse(e.payload) : e.payload;
    console.log('[sidecar] pasted:', d.text);
    flashState('success');
  });

  L('sidecar:no_speech', () => { console.log('[sidecar] no_speech'); stopTimer(); flashState('error'); });
  L('sidecar:error', (e) => {
    const d = typeof e.payload === 'string' ? JSON.parse(e.payload) : e.payload;
    console.error('[sidecar] error:', d.message);
    stopTimer();
    flashState('error');
  });
  L('sidecar:crashed', () => { console.error('[sidecar] CRASHED'); stopTimer(); flashState('error', 3000); });
} else {
  console.warn('[widget] No Tauri listen() — events will not work');
}

dlog('[widget] loaded, tauri=' + !!window.__TAURI_INTERNALS__);
