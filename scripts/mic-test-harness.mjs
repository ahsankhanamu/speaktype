// Drives the real settings.html/css/js in headless Chrome against a stubbed
// Tauri IPC layer, so the meter's rendering, envelope and verdict logic can be
// exercised with synthetic `mictest:level` events.
//
// Usage: node scripts/mic-test-harness.mjs
// Writes screenshots + a throwaway Chrome profile under .mic-test-verify/
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..');
const WORK = resolve(ROOT, '.mic-test-verify');
const OUT = resolve(WORK, 'shots');
const PORT = 9333;
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';

mkdirSync(OUT, { recursive: true });

const STUB = `
window.__stub = {
  listeners: {},
  calls: [],
  theme: 'dark',
  emit(name, payload) { (this.listeners[name] || []).forEach(cb => cb({ payload })); },
  invoke(cmd, args) {
    this.calls.push({ cmd, args });
    switch (cmd) {
      case 'get_settings':
        return Promise.resolve({
          hotkey: 'F9', api_url: 'http://127.0.0.1:8002/inference', model: 'small.en',
          language: 'auto', paste_mode: 'original', post_paste_keys: '',
          save_recordings: true, hallucination_guard: true, theme: this.theme,
        });
      case 'check_permissions':
        return Promise.resolve({ microphone: true, accessibility: true });
      case 'get_audio_input_info':
        return Promise.resolve({ device: 'MacBook Pro Microphone', inputs: 2 });
      case 'start_mic_test':
        return Promise.resolve({
          device: 'MacBook Pro Microphone', sample_rate: 48000,
          channels: 1, inputs: 2, max_ms: 60000,
        });
      case 'get_models':
        return Promise.resolve({ available: [], downloading: [], waiting: [], progress: [] });
      case 'get_model_progress':
        return Promise.resolve({ downloading: [], waiting: [], progress: [] });
      case 'get_history':
        return Promise.resolve({ entries: [] });
      case 'get_server_status':
        return Promise.resolve({ embedded: false, running: false, port: null });
      default:
        return Promise.resolve(null);
    }
  },
};
window.__TAURI__ = {
  core: { invoke: (cmd, args) => window.__stub.invoke(cmd, args) },
  event: {
    listen: (name, cb) => {
      (window.__stub.listeners[name] = window.__stub.listeners[name] || []).push(cb);
      return Promise.resolve(() => {});
    },
    emit: () => Promise.resolve(),
  },
};
window.__feed = (make, ms) => new Promise(done => {
  const t0 = performance.now();
  const id = setInterval(() => {
    const t = performance.now() - t0;
    if (t >= ms) { clearInterval(id); done(); return; }
    window.__stub.emit('mictest:level', make(t));
  }, 33);
});
window.__sampleLevels = (frames) => new Promise(done => {
  const out = [];
  const tick = () => {
    out.push(Number(window.MicTest.meter.level.toFixed(5)));
    if (out.length >= frames) { done(out); return; }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
});
`;

const sleep = (ms) => new Promise(r => setTimeout(r, ms));

const chrome = spawn(CHROME, [
  '--headless=new',
  `--remote-debugging-port=${PORT}`,
  `--user-data-dir=${resolve(WORK, 'chrome-profile')}`,
  '--no-first-run',
  '--disable-gpu',
  '--hide-scrollbars',
  '--allow-file-access-from-files',
  'about:blank',
], { stdio: 'ignore' });

let ws;
let nextId = 1;
const pending = new Map();

function send(method, params = {}) {
  const id = nextId++;
  ws.send(JSON.stringify({ id, method, params }));
  return new Promise((res, rej) => pending.set(id, { res, rej }));
}

async function evaluate(expression) {
  const result = await send('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails) {
    throw new Error(result.exceptionDetails.exception?.description || 'evaluate failed');
  }
  return result.result.value;
}

async function shot(name, selector) {
  const params = { format: 'png', captureBeyondViewport: true };
  if (selector) {
    const box = await evaluate(`(() => {
      const r = document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect();
      return { x: r.x - 8, y: r.y - 8, width: r.width + 16, height: r.height + 16 };
    })()`);
    params.clip = { ...box, scale: 2 };
  }
  const { data } = await send('Page.captureScreenshot', params);
  const file = resolve(OUT, `${name}.png`);
  writeFileSync(file, Buffer.from(data, 'base64'));
  return file;
}

async function connect() {
  for (let i = 0; i < 60; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
      const page = list.find(t => t.type === 'page');
      if (page) return page.webSocketDebuggerUrl;
    } catch (_) { /* not up yet */ }
    await sleep(250);
  }
  throw new Error('Chrome did not expose a page target');
}

async function openSettings(theme) {
  await send('Page.navigate', { url: 'about:blank' });
  await sleep(150);
  await send('Page.addScriptToEvaluateOnNewDocument', {
    source: STUB + `\nwindow.__stub.theme = ${JSON.stringify(theme)};`,
  });
  await send('Emulation.setDeviceMetricsOverride', {
    width: 880, height: 720, deviceScaleFactor: 2, mobile: false,
  });
  await send('Page.navigate', { url: `file://${ROOT}/packages/widget-ui/settings.html` });
  await sleep(1200);
  await evaluate(`switchTab('general'); document.documentElement.setAttribute('data-theme', ${JSON.stringify(theme)});`);
  await sleep(400);
}

const results = {};
const screenshots = {};

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
  await new Promise(r => ws.addEventListener('open', r));
  await send('Page.enable');
  await send('Runtime.enable');

  for (const theme of ['dark', 'light']) {
    await openSettings(theme);

    results[`${theme}_card_visible`] = await evaluate(
      `!document.getElementById('mic-test-card').hidden`
    );
    results[`${theme}_idle_status`] = await evaluate(
      `document.getElementById('mic-test-status').textContent`
    );
    screenshots[`${theme}-idle`] = await shot(`${theme}-idle`, '#mic-test-card');

    await evaluate(`document.getElementById('mic-test-btn').click()`);
    await sleep(200);
    results[`${theme}_started`] = await evaluate(`window.MicTest.running === true`);

    // Speech-like: peaks around -10dBFS with syllabic gaps.
    await evaluate(`window.__feed(t => {
      const env = 0.35 * (0.55 + 0.45 * Math.sin(t / 260));
      return { peak: env, rms: env * 0.3, clipped: false };
    }, 2500)`);
    results[`${theme}_good_status`] = await evaluate(
      `document.getElementById('mic-test-status').textContent`
    );
    results[`${theme}_good_state`] = await evaluate(
      `document.getElementById('mic-test-status').dataset.state`
    );
    await evaluate(`window.__stub.emit('mictest:level', { peak: 0.42, rms: 0.12, clipped: false })`);
    await sleep(90);
    screenshots[`${theme}-good`] = await shot(`${theme}-good`, '#mic-test-card');

    await evaluate(`window.__feed(() => ({ peak: 0.995, rms: 0.4, clipped: true }), 900)`);
    results[`${theme}_clip_status`] = await evaluate(
      `document.getElementById('mic-test-status').textContent`
    );
    results[`${theme}_clip_state`] = await evaluate(
      `document.getElementById('mic-test-status').dataset.state`
    );
    screenshots[`${theme}-clipping`] = await shot(`${theme}-clipping`, '#mic-test-card');

    await evaluate(`document.getElementById('mic-test-btn').click()`);
    await sleep(150);
  }

  // Behavioural checks (theme-independent), run on the light session already open.
  await openSettings('dark');
  await evaluate(`document.getElementById('mic-test-btn').click()`);
  await sleep(150);

  await evaluate(`window.__feed(() => ({ peak: 0.0004, rms: 0.0002, clipped: false }), 3000)`);
  results.silent_status = await evaluate(`document.getElementById('mic-test-status').textContent`);
  results.silent_state = await evaluate(`document.getElementById('mic-test-status').dataset.state`);
  results.silent_detail = await evaluate(`document.getElementById('mic-test-detail').textContent`);
  results.silent_sound_btn_visible = await evaluate(
    `!document.getElementById('mic-test-sound-btn').hidden`
  );
  screenshots['dark-silent'] = await shot('dark-silent', '#mic-test-card');

  await evaluate(`document.getElementById('mic-test-btn').click()`);
  await sleep(150);
  await evaluate(`document.getElementById('mic-test-btn').click()`);
  await sleep(150);
  await evaluate(`window.__feed(() => ({ peak: 0.03, rms: 0.004, clipped: false }), 1200)`);
  results.quiet_status = await evaluate(`document.getElementById('mic-test-status').textContent`);
  results.quiet_state = await evaluate(`document.getElementById('mic-test-status').dataset.state`);
  screenshots['dark-quiet'] = await shot('dark-quiet', '#mic-test-card');

  // Interpolation: one event, then sample the envelope every frame. A stepped
  // meter would show a single jump; an interpolated one ramps across frames.
  await evaluate(`window.MicTest.meter.level = 0; window.MicTest.meter.target = 0;`);
  await evaluate(`window.__stub.emit('mictest:level', { peak: 0.5, rms: 0.15, clipped: false })`);
  const attack = await evaluate(`window.__sampleLevels(8)`);
  results.attack_samples = attack;
  results.attack_is_monotonic = attack.every((v, i) => i === 0 || v >= attack[i - 1]);
  results.attack_distinct_values = new Set(attack).size;

  await evaluate(`window.__stub.emit('mictest:level', { peak: 0.0001, rms: 0.0001, clipped: false })`);
  const decay = await evaluate(`window.__sampleLevels(12)`);
  results.decay_samples = decay;
  results.decay_is_monotonic = decay.every((v, i) => i === 0 || v <= decay[i - 1]);
  // Per-frame fraction of the remaining distance travelled: attack should close
  // most of the gap in one frame, decay only a few percent.
  const attackRate = (attack[1] - attack[0]) / (attack[attack.length - 1] - attack[0]);
  const decayRate = (decay[0] - decay[1]) / decay[0];
  results.attack_frame_rate = Number(attackRate.toFixed(3));
  results.decay_frame_rate = Number(decayRate.toFixed(3));
  results.decay_slower_than_attack = decayRate < attackRate;

  // Lifecycle: blur, tab switch, and backend-initiated stop must all release.
  results.stop_calls_before = await evaluate(
    `window.__stub.calls.filter(c => c.cmd === 'stop_mic_test').length`
  );
  await evaluate(`window.dispatchEvent(new Event('blur'))`);
  await sleep(150);
  results.blur_stops = await evaluate(`window.MicTest.running === false`);
  results.blur_raf_cancelled = await evaluate(`window.MicTest.meter.rafId === 0`);
  results.stop_calls_after_blur = await evaluate(
    `window.__stub.calls.filter(c => c.cmd === 'stop_mic_test').length`
  );

  await evaluate(`document.getElementById('mic-test-btn').click()`);
  await sleep(150);
  await evaluate(`switchTab('models')`);
  await sleep(150);
  results.tab_switch_stops = await evaluate(`window.MicTest.running === false`);
  results.stop_calls_after_tab = await evaluate(
    `window.__stub.calls.filter(c => c.cmd === 'stop_mic_test').length`
  );

  await evaluate(`switchTab('general')`);
  await sleep(150);
  await evaluate(`document.getElementById('mic-test-btn').click()`);
  await sleep(150);
  await evaluate(`window.__stub.emit('mictest:stopped', { reason: 'recording' })`);
  await sleep(150);
  results.backend_stop_stops = await evaluate(`window.MicTest.running === false`);
  results.backend_stop_note = await evaluate(
    `document.getElementById('mic-test-detail').textContent`
  );
  results.backend_stop_button = await evaluate(
    `document.getElementById('mic-test-btn').textContent`
  );
  screenshots['dark-after-dictation'] = await shot('dark-after-dictation', '#mic-test-card');

  results.permission_hides_card = await evaluate(`
    window.MicTest.setPermission(false);
    document.getElementById('mic-test-card').hidden;
  `);

  console.log(JSON.stringify({ results, screenshots }, null, 2));
}

run()
  .catch(err => { console.error('HARNESS FAILED:', err); process.exitCode = 1; })
  .finally(async () => {
    try { ws?.close(); } catch (_) { /* ignore */ }
    chrome.kill();
  });
