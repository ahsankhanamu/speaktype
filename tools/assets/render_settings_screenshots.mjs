#!/usr/bin/env node
// Renders the NEW Svelte SpeakType UI (settings / onboarding) in headless Chrome
// against a stubbed Tauri IPC layer, producing fresh promotional screenshots for
// assets/source/screenshots so the showcase composer (compose_showcase.py) always
// reflects the current UI.
//
// Usage:
//   (cd apps/widget/webui && npm run build)     # build once, or rely on dist/
//   node tools/assets/render_settings_screenshots.mjs
//
// Outputs:
//   assets/source/screenshots/screenshot-0<int>-general.png    (General / permissions)
//   assets/source/screenshots/screenshot-0<int>-hotkey.png
//   assets/source/screenshots/screenshot-0<int>-server.png
//   assets/source/screenshots/screenshot-0<int>-models.png
//   assets/source/screenshots/screenshot-0<int>-history.png
//   assets/source/screenshots/screenshot-0<int>-debug.png
//   assets/source/screenshots/screenshot-0<int>-about.png
//   assets/source/screenshots/screenshot-0<int>-onboarding.png
//
// Filenames use two-prefixed zero-padded sequence numbers; every render overwrites
// the full set so the set stays consistent with the latest UI.

import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../..');
const DIST = resolve(ROOT, 'apps/widget/webui/dist');
const OUT = resolve(ROOT, 'assets/source/screenshots/raw');
const SERVE_PORT = 9358;
const DEVTOOLS_PORT = 9357;
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const MIME = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.json': 'application/json',
  '.woff2': 'font/woff2',
};

mkdirSync(OUT, { recursive: true });

// ---------------------------------------------------------------------------
// Tauri IPC stub — returns realistic data so every tab renders fully.
// ---------------------------------------------------------------------------
const STUB = `
window.__stub = { theme: 'dark' };

const MODEL_CATALOG = [
  ['tiny.en','Tiny (English only)','39 MB',39000000,'fastest',true],
  ['tiny','Tiny','39 MB',39000000,'fastest',true],
  ['base.en','Base (English only)','74 MB',74000000,'fast',true],
  ['base','Base','74 MB',74000000,'fast',true],
  ['small.en','Small (English only)','244 MB',244000000,'balanced',true],
  ['small','Small','244 MB',244000000,'balanced',true],
  ['medium.en','Medium (English only)','769 MB',769000000,'accurate',false],
  ['medium','Medium','769 MB',769000000,'accurate',false],
  ['large-v3','Large v3','2.9 GB',2900000000,'most accurate',false],
  ['large-v3-turbo','Large v3 Turbo','809 MB',809000000,'fast + accurate',false],
];
const ACTIVE_MODEL = 'small.en';

const modelList = MODEL_CATALOG.map(([id,desc,size,sizeBytes], i) => {
  const downloaded = i < 6;
  return {
    id, desc, size, size_bytes: sizeBytes,
    downloaded, downloaded_size: downloaded ? size : undefined,
    speed: 'balanced',
  };
});

window.__stub.invoke = (cmd) => {
  switch (cmd) {
    case 'get_settings':
      return Promise.resolve({
        hotkey: 'Super+Control', api_url: 'http://127.0.0.1:8002/inference',
        model: ACTIVE_MODEL, language: 'auto',
        window_x: null, window_y: null,
        onboarding_window_x: null, onboarding_window_y: null,
        settings_x: null, settings_y: null, settings_w: null, settings_h: null,
        paste_mode: 'active', post_paste_keys: 'enter', save_recordings: false,
        theme: window.__stub.theme,
        hallucination_guard: true, hallucination_retries: 2,
        quality_no_speech_prob: 0.6, quality_avg_logprob: -1.0,
        quality_compression_ratio: 2.4, input_device: 'MacBook Pro Microphone',
      });
    case 'save_settings':
      return Promise.resolve(null);
    case 'get_input_devices':
      return Promise.resolve({
        devices: [
          { name: 'MacBook Pro Microphone', label: 'MacBook Pro Microphone (48 kHz)', is_default: true },
          { name: 'External USB Mic', label: 'External USB Mic (16 kHz)', is_default: false },
          { name: 'AirPods Pro', label: 'AirPods Pro (24 kHz)', is_default: false },
        ],
        default: 'MacBook Pro Microphone',
      });
    case 'get_audio_input_info':
      return Promise.resolve({ device: 'MacBook Pro Microphone', inputs: 2 });
    case 'check_server':
      return Promise.resolve({ status: 'connected', message: 'Connected', info: null });
    case 'get_server_status':
      return Promise.resolve({ embedded: true, running: true, port: 8002 });
    case 'check_permissions':
      return Promise.resolve({ accessibility: true, microphone: true });
    case 'get_models':
      return Promise.resolve({ available: modelList, downloading: [], waiting: [], progress: [] });
    case 'get_model_progress':
      return Promise.resolve({ downloading: [], waiting: [], progress: [] });
    case 'get_history': {
      const text = [
        'Local transcription on your machine — fast, private, no cloud required.',
        'Press your hotkey, speak, and press again — your text appears in the focused window.',
        'SpeakType fits into your normal workflow. Browsers, IDEs, Slack, and plain text fields all accept paste.',
      ];
      const now = Date.now();
      const entries = text.map((t, i) => ({
        text: t,
        timestamp: new Date(now - i * 4 * 60000).toISOString().slice(0, 19),
        duration_secs: 4 + i,
      }));
      return Promise.resolve({ entries });
    }
    case 'get_history_audio':
      return Promise.resolve([]);
    case 'get_debug_sessions':
      return Promise.resolve([{
        id: 1, startedAt: '2025-09-04 14:30:00', mode: 'live', model: ACTIVE_MODEL,
        durationSecs: 4.2, elapsedMs: 3200, finalChars: 46,
        chunks: [
          { idx: 0, source: 'pipelined', durationSecs: 2.0, startSecs: 0.0, endSecs: 2.0, status: 'accepted', chars: 46, reason: '', seededPrompt: false },
          { idx: 1, source: 'pipelined', durationSecs: 2.2, startSecs: 2.0, endSecs: 4.2, status: 'dropped', chars: 0, reason: 'low-confidence', seededPrompt: false },
        ],
        passes: [
          { chunkIdx: 0, pass: 1, temperature: 0.0, promptUsed: true, rawChars: 46, score: 95.0, dropped: 0, hadMetrics: true, reason: 'clean',
            segments: [{ text: 'Local transcription on your machine', start: 0.0, end: 2.0, noSpeechProb: 0.1, avgLogprob: -0.5, flags: [], bad: false }] },
        ],
        finalText: 'Local transcription on your machine — fast, private, no cloud required.',
      }]);
    case 'get_debug_audio_slice':
      return Promise.resolve([]);
    case 'get_onboarding_status':
      return Promise.resolve({
        microphone: true,
        accessibility: true,
        model: false,
        active_model: '',
        exe_path: '/Applications/SpeakType.app',
        is_dev: false,
        is_bundled: true,
        responsible_app: 'SpeakType',
        responsible_app_path: '',
      });
    case 'get_server_status_poll':
      return Promise.resolve(null);
    case 'update_hotkey':
    case 'request_accessibility':
    case 'request_microphone_access':
    case 'open_about':
    case 'open_system_pane':
    case 'open_models_folder':
    case 'open_recordings_folder':
    case 'reset_widget_position':
    case 'stop_mic_test':
    case 'clear_history':
    case 'delete_history_entry':
    case 'delete_history_audio':
    case 'clear_debug_sessions':
    case 'delete_debug_session':
    case 'log_frontend':
      return Promise.resolve(null);
    default:
      return Promise.resolve(null);
  }
};

window.__TAURI_INTERNALS__ = {
  invoke: (cmd, args, options) => window.__stub.invoke(cmd, args),
  transformCallback: (callback, once) => {
    window.__stub.callbacks = window.__stub.callbacks || {};
    const id = (window.__stub._cbId = (window.__stub._cbId || 0) + 1);
    window.__stub.callbacks[id] = callback;
    return id;
  },
  unregisterCallback: (id) => { delete (window.__stub.callbacks || {})[id]; },
  unregisterListener: () => {},
  convertFileSrc: (filePath, protocol) => protocol + '://localhost/' + filePath,
  metadata: { currentWindow: { label: 'main' } },
};
`;

// ---------------------------------------------------------------------------
// Static file server for the webui dist
// ---------------------------------------------------------------------------
function serve() {
  return createServer((req, res) => {
    const urlPath = decodeURIComponent((req.url || '/').split('?')[0]);
    const rel = urlPath === '/' ? '/settings.html' : urlPath;
    const filePath = join(DIST, rel);
    if (!filePath.startsWith(DIST)) { res.writeHead(403); res.end(); return; }
    try {
      const body = readFileSync(filePath);
      res.writeHead(200, { 'content-type': MIME[extname(filePath)] || 'application/octet-stream' });
      res.end(body);
    } catch {
      res.writeHead(404);
      res.end('not found');
    }
  });
}

// ---------------------------------------------------------------------------
// headless Chrome scaffolding (same style as mic-test-harness)
// ---------------------------------------------------------------------------
const chrome = spawn(CHROME, [
  '--headless=new',
  `--remote-debugging-port=${DEVTOOLS_PORT}`,
  '--no-first-run',
  '--disable-gpu',
  '--hide-scrollbars',
  `--user-data-dir=${resolve(ROOT, '.screenshot-chrome-profile')}`,
  'about:blank',
], { stdio: 'ignore' });

let ws;
let nextId = 1;
const pending = new Map();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let curViewport = { w: 880, h: 640 };

function send(method, params = {}) {
  const id = nextId++;
  ws.send(JSON.stringify({ id, method, params }));
  return new Promise((res, rej) => pending.set(id, { res, rej }));
}

async function evaluate(expression) {
  const result = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (result.exceptionDetails) {
    throw new Error(result.exceptionDetails.exception?.description || 'evaluate failed');
  }
  return result.result.value;
}

async function shot(name) {
  // Capture exactly the window viewport (real window bounds), not the
  // document's full scroll height — matches what an actual window shows.
  const { data } = await send('Page.captureScreenshot', {
    format: 'png',
    captureBeyondViewport: false,
    clip: { x: 0, y: 0, width: curViewport.w, height: curViewport.h, scale: 1 },
  });
  const file = resolve(OUT, name);
  writeFileSync(file, Buffer.from(data, 'base64'));
  console.log(`Wrote ${file.replace(ROOT, '.')}`);
}

async function connect() {
  for (let i = 0; i < 60; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${DEVTOOLS_PORT}/json/list`)).json();
      const page = list.find((t) => t.type === 'page');
      if (page) return page.webSocketDebuggerUrl;
    } catch { /* not up yet */ }
    await sleep(250);
  }
  throw new Error('Chrome did not expose a page target');
}

async function navigate(page, theme, width, height) {
  curViewport = { w: width, h: height };
  await send('Page.navigate', { url: 'about:blank' });
  await sleep(120);
  await send('Page.addScriptToEvaluateOnNewDocument', { source: `${STUB}\nwindow.__stub.theme = ${JSON.stringify(theme)};` });
  await send('Emulation.setDeviceMetricsOverride', {
    width, height, deviceScaleFactor: 2, mobile: false,
  });
  await send('Page.navigate', { url: `http://127.0.0.1:${SERVE_PORT}/${page}` });
  await sleep(1200);
  await evaluate(`document.documentElement.setAttribute('data-theme', ${JSON.stringify(theme)});`);
  await sleep(400);
}

async function switchTab(tab) {
  const ok = await evaluate(`(() => {
    const b = document.querySelector('.tab[data-tab=${JSON.stringify(tab)}]');
    if (!b) return false;
    b.click();
    return true;
  })()`);
  if (!ok) throw new Error(`Tab not found: ${tab}`);
  await flushRaf();
  await sleep(700);
}

async function flushRaf() {
  await evaluate(`new Promise((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  })`);
}

async function run() {
  const wsUrl = await connect();
  ws = new WebSocket(wsUrl);
  ws.addEventListener('message', (event) => {
    const msg = JSON.parse(event.data);
    if (msg.id && pending.has(msg.id)) {
      const { res, rej } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) rej(new Error(msg.error.message));
      else res(msg.result);
    }
  });
  await new Promise((r) => ws.addEventListener('open', r));
  await send('Page.enable');
  await send('Runtime.enable');

  const theme = 'dark';
  const W = 880;
  const H = 640;
  const name_map = {
    hotkey: '04-hotkey',
    server: '02-server',
    general: '01-general',
    models: '03-models',
    history: '05-history',
    debug: '08-debug',
    about: '06-about',
    onboarding: '07-onboarding',
  };

  // Settings tabs
  await navigate('settings.html', theme, W, H);
  await switchTab('hotkey');
  await shot(`screenshot-${name_map.hotkey}.png`);

  await switchTab('server');
  await shot(`screenshot-${name_map.server}.png`);

  await switchTab('general');
  await shot(`screenshot-${name_map.general}.png`);

  await switchTab('models');
  console.log('MODELS rendered:', await evaluate(`document.querySelectorAll('.model-card-row').length`));
  await shot(`screenshot-${name_map.models}.png`);

  await switchTab('history');
  console.log('HISTORY rendered:', await evaluate(`document.querySelectorAll('.history-entry').length`));
  await shot(`screenshot-${name_map.history}.png`);

  await switchTab('debug');
  console.log('DEBUG rendered:', await evaluate(`document.querySelectorAll('.debug-session').length`));
  await shot(`screenshot-${name_map.debug}.png`);

  // About
  await navigate('about.html', theme, 320, 380);
  await shot(`screenshot-${name_map.about}.png`);

  // Onboarding (two-column redesign)
  await navigate('onboarding.html', theme, 880, 640);
  console.log('ONBOARDING MODELS rendered:', await evaluate(`document.querySelectorAll('.model-card-row').length`));
  await shot(`screenshot-${name_map.onboarding}.png`);

  console.log(`Done — screenshots in assets/source/screenshots/`);
}

// ---------------------------------------------------------------------------

async function main() {
  const server = serve();
  await new Promise((r) => server.listen(SERVE_PORT, r));
  try {
    await run();
  } finally {
    server.close();
    chrome.kill();
  }
}

main().catch((e) => {
  console.error(e);
  try { chrome.kill(); } catch {}
  process.exit(1);
});
