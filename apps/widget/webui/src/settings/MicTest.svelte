<script lang="ts">
  import { onMount } from 'svelte';
  import {
    startMicTest,
    stopMicTest,
    getAudioInputInfo,
    openSystemPane,
    listenEvent,
  } from '../lib/ipc';

  let {
    permitted = null,
    sectionVisible = true,
    inputDeviceValue = '',
  }: {
    permitted?: boolean | null;
    sectionVisible?: boolean;
    inputDeviceValue?: string;
  } = $props();

  const MIN_DB = -60;
  const SEG_W = 6;
  const SEG_GAP = 3;
  const SEG_MIN = 14;
  const SEG_RADIUS = 1.5;
  const ATTACK_TAU_MS = 18;
  const DECAY_TAU_MS = 320;
  const HOLD_MS = 900;
  const HOLD_FALL_PER_MS = 0.0009;
  const MAX_FRAME_MS = 250;
  const HOT_FROM = 0.9;
  const CLIP_FROM = 0.975;
  const CLIP_PEAK = 0.98;
  const CLIP_STICKY_MS = 2000;
  const GOOD_RMS = 0.012;
  const SIGNAL_RMS = 0.0015;
  const SILENT_AFTER_MS = 2200;
  const FALLBACK_COLORS = {
    track: 'rgba(128, 128, 128, 0.25)',
    normal: '#3b82f6',
    hot: '#f59e0b',
    clip: '#ef4444',
  };
  const hasRoundRect =
    typeof CanvasRenderingContext2D !== 'undefined' &&
    typeof CanvasRenderingContext2D.prototype.roundRect === 'function';

  const now = () => (typeof performance !== 'undefined' ? performance.now() : Date.now());

  let canvas = $state<HTMLCanvasElement | null>(null);
  let ctx: CanvasRenderingContext2D | null = null;
  let running = $state(false);
  let starting = $state(false);
  let device = $state('Checking input…');
  let statusText = $state('Not tested yet.');
  let statusState = $state('idle');
  let detail = $state('');
  let soundShown = $state(false);

  // Meter state
  let level = 0;
  let target = 0;
  let hold = 0;
  let holdUntil = 0;
  let startedAt = 0;
  let bestRms = 0;
  let lastClipAt = 0;
  let lastTs = 0;
  let rafId = 0;
  let colors = { ...FALLBACK_COLORS };
  let runningRef = false;

  const VERDICTS: Record<string, { state: string; text: string; detail?: string; showSound?: boolean }> = {
    idle: { state: 'idle', text: 'Not tested yet.' },
    listening: { state: 'idle', text: 'Listening — say something.' },
    silent: {
      state: 'bad',
      text: 'No sound is reaching SpeakType.',
      detail: 'Check that the right input is selected and unmuted in Sound settings.',
      showSound: true,
    },
    quiet: {
      state: 'warn',
      text: 'Very quiet — dictation may miss words.',
      detail: 'Move closer to the microphone or raise its input volume.',
      showSound: true,
    },
    good: { state: 'ok', text: 'Microphone is working.' },
    clipping: {
      state: 'warn',
      text: 'Too loud — the signal is clipping.',
      detail: 'Lower the input volume so the meter stays out of the red.',
      showSound: true,
    },
  };
  const STOP_NOTES: Record<string, string> = {
    recording: 'Test stopped — dictation started.',
    timeout: 'Test stopped automatically after a minute.',
    window_closed: 'Test stopped.',
  };

  function normalizeDb(peak: number): number {
    if (!(peak > 0)) return 0;
    const db = 20 * Math.log10(Math.min(1, peak));
    if (db <= MIN_DB) return 0;
    return Math.min(1, (db - MIN_DB) / -MIN_DB);
  }

  function syncSurface() {
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth || 0;
    const h = canvas.clientHeight || 0;
    if (w <= 0 || h <= 0) return;
    canvas.width = Math.max(1, Math.round(w * dpr));
    canvas.height = Math.max(1, Math.round(h * dpr));
    ctx = canvas.getContext('2d');
    if (ctx) ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  function refreshColors() {
    if (!canvas) return;
    let style: CSSStyleDeclaration | null = null;
    try {
      style = window.getComputedStyle(canvas);
    } catch (_) {
      style = null;
    }
    const read = (name: string, fallback: string) => {
      if (!style) return fallback;
      const v = style.getPropertyValue(name).trim();
      return v || fallback;
    };
    colors = {
      track: read('--surface-4', FALLBACK_COLORS.track),
      normal: read('--accent', FALLBACK_COLORS.normal),
      hot: read('--warning-alt', FALLBACK_COLORS.hot),
      clip: read('--danger', FALLBACK_COLORS.clip),
    };
  }

  function zoneColor(index: number, count: number) {
    const position = count > 1 ? index / (count - 1) : 0;
    if (position >= CLIP_FROM) return colors.clip;
    if (position >= HOT_FROM) return colors.hot;
    return colors.normal;
  }

  function draw() {
    if (!canvas || !ctx) return;
    const w = canvas.clientWidth || 0;
    const h = canvas.clientHeight || 0;
    if (w <= 0 || h <= 0) return;
    ctx.clearRect(0, 0, w, h);
    const count = Math.max(SEG_MIN, Math.floor((w + SEG_GAP) / (SEG_W + SEG_GAP)));
    const pitch = (w + SEG_GAP) / count;
    const segW = Math.max(1, pitch - SEG_GAP);
    const litCount = Math.round(level * count);
    const holdIndex =
      hold > 0 ? Math.min(count - 1, Math.max(0, Math.ceil(hold * count) - 1)) : -1;
    for (let i = 0; i < count; i++) {
      const lit = i < litCount || i === holdIndex;
      ctx.fillStyle = lit ? zoneColor(i, count) : colors.track;
      const x = i * pitch;
      if (hasRoundRect) {
        ctx.beginPath();
        (ctx as CanvasRenderingContext2D & { roundRect?: (a: number, b: number, c: number, d: number, e: number) => void }).roundRect?.(x, 0, segW, h, SEG_RADIUS);
        ctx.fill();
      } else {
        ctx.fillRect(x, 0, segW, h);
      }
    }
  }

  function updateVerdict(t: number) {
    let verdict: string;
    if (t - lastClipAt < CLIP_STICKY_MS) verdict = 'clipping';
    else if (bestRms >= GOOD_RMS) verdict = 'good';
    else if (bestRms >= SIGNAL_RMS) verdict = 'quiet';
    else if (t - startedAt < SILENT_AFTER_MS) verdict = 'listening';
    else verdict = 'silent';
    if (verdict !== statusState) renderVerdict(verdict);
  }

  function frame(ts: number) {
    if (!runningRef) return;
    if (canvas) {
      const dpr = window.devicePixelRatio || 1;
      const w = canvas.clientWidth || 0;
      const h = canvas.clientHeight || 0;
      if (Math.abs(canvas.width - w * dpr) > 1 || Math.abs(canvas.height - h * dpr) > 1) syncSurface();
    }
    let dt = lastTs ? ts - lastTs : 16;
    lastTs = ts;
    if (dt < 0) dt = 0;
    else if (dt > MAX_FRAME_MS) dt = MAX_FRAME_MS;
    const tau = target > level ? ATTACK_TAU_MS : DECAY_TAU_MS;
    level += (target - level) * (1 - Math.exp(-dt / tau));
    const t = now();
    if (level >= hold) {
      hold = level;
      holdUntil = t + HOLD_MS;
    } else if (t > holdUntil) {
      hold = Math.max(level, hold - dt * HOLD_FALL_PER_MS);
    }
    updateVerdict(t);
    draw();
    rafId = requestAnimationFrame(frame);
  }

  function resetMeter() {
    if (rafId) cancelAnimationFrame(rafId);
    rafId = 0;
    lastTs = 0;
    level = 0;
    target = 0;
    hold = 0;
    holdUntil = 0;
    bestRms = 0;
    lastClipAt = 0;
  }

  function renderVerdict(verdict: string) {
    const copy = VERDICTS[verdict] || VERDICTS.idle;
    statusText = copy.text;
    statusState = copy.state;
    detail = copy.detail || '';
    soundShown = !!copy.showSound;
    syncButton();
  }

  function renderError(error: unknown) {
    const message =
      error instanceof Error ? error.message : String((error as { message?: string })?.message || error || 'Unknown error');
    statusText = message;
    statusState = 'bad';
    detail = '';
    soundShown = false;
    syncButton();
  }

  function syncButton() {
    // running state drives button label via reactive binding; no-op here
  }

  function settle(reason: string) {
    runningRef = false;
    running = false;
    if (rafId) cancelAnimationFrame(rafId);
    rafId = 0;
    level = 0;
    target = 0;
    hold = 0;
    holdUntil = 0;
    bestRms = 0;
    lastClipAt = 0;
    draw();
    if (statusState === 'listening' || statusState === 'idle') renderVerdict('idle');
    const note = STOP_NOTES[reason];
    if (note) {
      detail = note;
    }
  }

  function stop(reason: string) {
    if (!runningRef) return;
    settle(reason);
    stopMicTest().catch((e) => console.error('Failed to stop mic test:', e));
  }

  async function start() {
    if (runningRef || starting) return;
    starting = true;
    try {
      const info = await startMicTest();
      runningRef = true;
      running = true;
      resetMeter();
      startedAt = now();
      rafId = requestAnimationFrame(frame);
      if (info && info.device) {
        const rate = info.sample_rate ? ` · ${Math.round(info.sample_rate / 1000)} kHz` : '';
        device = `${info.device}${rate}`;
      }
      renderVerdict('listening');
    } catch (e) {
      renderError(e);
    } finally {
      starting = false;
    }
  }

  function toggle() {
    if (runningRef) stop('user');
    else start();
  }

  async function refreshDevice() {
    if (runningRef) return;
    try {
      const info = (await getAudioInputInfo()) as { device?: string | null } | null;
      const selected = inputDeviceValue;
      if (selected) {
        device = selected;
      } else if (info && info.device) {
        device = info.device;
      } else {
        device = 'No input device';
      }
    } catch (e) {
      console.error('Failed to read input device:', e);
    }
  }

  function handleBackendStop(reason: string) {
    if (!runningRef) return;
    settle(reason);
  }

  $effect(() => {
    if (sectionVisible) return;
    stop('navigated');
  });

  $effect(() => {
    if (permitted === null) return;
    if (!permitted) stop('permission');
  });

  onMount(() => {
    if (!canvas) return;
    syncSurface();
    refreshColors();
    draw();
    lastTs = 0;

    const unlistenLevel = listenEvent<unknown>('mictest:level', (event) => {
      if (!runningRef) return;
      const payload =
        typeof event === 'string' ? JSON.parse(event) : (event as Record<string, unknown>);
      const peak = Math.max(0, Math.min(1, Number((payload as { peak?: number }).peak) || 0));
      const rms = Math.max(0, Number((payload as { rms?: number }).rms) || 0);
      target = normalizeDb(peak);
      if (rms > bestRms) bestRms = rms;
      if ((payload as { clipped?: boolean }).clipped || peak >= CLIP_PEAK) lastClipAt = now();
    });

    const unlistenStopped = listenEvent<string | Record<string, unknown>>('mictest:stopped', (event) => {
      const payload = typeof event === 'string' ? JSON.parse(event) : event;
      handleBackendStop(((payload as Record<string, unknown>).reason as string) || '');
    });

    const unlistenDevice = listenEvent<void>('sidecar:device_changed', () => refreshDevice());

    const cleanups: (() => void)[] = [];
    for (const p of [unlistenLevel, unlistenStopped, unlistenDevice]) p.then((fn) => cleanups.push(fn));

    const onBlur = () => stop('blur');
    const onVisChange = () => {
      if (document.hidden) stop('hidden');
    };
    window.addEventListener('blur', onBlur);
    document.addEventListener('visibilitychange', onVisChange);

    renderVerdict('idle');
    refreshDevice();

    return () => {
      if (rafId) cancelAnimationFrame(rafId);
      if (runningRef) stopMicTest().catch(() => {});
      runningRef = false;
      cleanups.forEach((fn) => fn());
      window.removeEventListener('blur', onBlur);
      document.removeEventListener('visibilitychange', onVisChange);
    };
  });
</script>

<div class="mic-test" id="mic-test">
  <div class="mic-test-head">
    <span class="mic-test-device" id="mic-test-device">{device}</span>
    <button
      class="btn secondary btn-small"
      id="mic-test-btn"
      type="button"
      aria-pressed={running || starting}
      aria-label={running ? 'Stop the microphone test' : 'Start the microphone test'}
      onclick={toggle}
      disabled={starting || permitted === false || permitted === null}
    >
      {running ? 'Stop Test' : 'Test Microphone'}
    </button>
  </div>
  <canvas
    bind:this={canvas}
    class="mic-test-meter"
    id="mic-test-meter"
    height="34"
    aria-hidden="true"
  ></canvas>
  <div class="mic-test-scale" aria-hidden="true">
    <span>Quiet</span>
    <span>Too loud</span>
  </div>
  <p class="mic-test-status" id="mic-test-status" data-state={statusState} role="status" aria-live="polite">
    {statusText}
  </p>
  <p class="mic-test-detail" id="mic-test-detail" hidden={!detail}>{detail}</p>
  <button
    class="btn secondary btn-small mic-test-sound"
    id="mic-test-sound-btn"
    type="button"
    hidden={!soundShown}
    onclick={() => openSystemPane('com.apple.preference.sound')}
  >
    Open Sound Settings
  </button>
</div>