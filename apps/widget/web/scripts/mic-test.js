/**
 * Microphone check for the settings General section.
 *
 * A segmented column meter driven by `mictest:level`, plus a verdict that
 * answers "is my microphone working" so the bars never have to be interpreted.
 * The meter interpolates between the 33ms level events in the render loop with
 * a fast-attack / slow-decay envelope, which is what makes an audio meter read
 * as continuous rather than stepped.
 */
(function () {
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

  /**
   * Ladder position where the meter turns amber, then red: -6dBFS and -1.5dBFS.
   * Comfortable speech peaks around -18dBFS, so a normal voice never leaves the
   * plain band — amber means genuinely loud, not merely audible.
   */
  const HOT_FROM = 0.9;
  const CLIP_FROM = 0.975;
  const CLIP_PEAK = 0.98;
  const CLIP_STICKY_MS = 2000;

  /**
   * Dictation drops audio whose 50ms segment RMS never exceeds 0.01, so a
   * "working" verdict is pinned just above that gate: anything this loud will
   * survive the speech check that runs on a real recording.
   */
  const GOOD_RMS = 0.012;
  const SIGNAL_RMS = 0.0015;
  const SILENT_AFTER_MS = 2200;

  const FALLBACK_COLORS = {
    track: 'rgba(128, 128, 128, 0.25)',
    normal: '#3b82f6',
    hot: '#f59e0b',
    clip: '#ef4444',
  };

  const hasRoundRect = typeof CanvasRenderingContext2D !== 'undefined'
    && typeof CanvasRenderingContext2D.prototype.roundRect === 'function';

  const now = () => (typeof performance !== 'undefined' ? performance.now() : Date.now());

  function normalizeDb(peak) {
    if (!(peak > 0)) return 0;
    const db = 20 * Math.log10(Math.min(1, peak));
    if (db <= MIN_DB) return 0;
    return Math.min(1, (db - MIN_DB) / -MIN_DB);
  }

  class MicMeter {
    constructor(canvas, options) {
      this.canvas = canvas;
      this.ctx = canvas.getContext('2d');
      this.onVerdict = (options && options.onVerdict) || null;
      this.frameBound = this.frame.bind(this);

      this.dpr = 0;
      this.cssW = 0;
      this.cssH = 0;
      this.colors = Object.assign({}, FALLBACK_COLORS);

      this.running = false;
      this.rafId = 0;
      this.lastTs = 0;
      this.level = 0;
      this.target = 0;
      this.hold = 0;
      this.holdUntil = 0;
      this.startedAt = 0;
      this.bestRms = 0;
      this.lastClipAt = 0;
      this.verdict = 'idle';

      this.refreshColors();
      this.watchTheme();
      this.watchSize();
      this.syncSurface(true);
      this.draw();
    }

    syncSurface(force) {
      const dpr = window.devicePixelRatio || 1;
      const w = this.canvas.clientWidth || this.canvas.width || 0;
      const h = this.canvas.clientHeight || this.canvas.height || 0;
      if (!force && dpr === this.dpr && w === this.cssW && h === this.cssH) return;
      this.dpr = dpr;
      this.cssW = w;
      this.cssH = h;
      this.canvas.width = Math.max(1, Math.round(w * dpr));
      this.canvas.height = Math.max(1, Math.round(h * dpr));
      this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }

    refreshColors() {
      let style = null;
      try {
        style = getComputedStyle(this.canvas);
      } catch (e) {
        style = null;
      }
      const read = (name, fallback) => {
        if (!style) return fallback;
        const value = style.getPropertyValue(name).trim();
        return value || fallback;
      };
      this.colors = {
        track: read('--surface-4', FALLBACK_COLORS.track),
        normal: read('--accent', FALLBACK_COLORS.normal),
        hot: read('--warning-alt', FALLBACK_COLORS.hot),
        clip: read('--danger', FALLBACK_COLORS.clip),
      };
    }

    watchTheme() {
      if (typeof MutationObserver === 'undefined') return;
      const observer = new MutationObserver(() => {
        this.refreshColors();
        this.draw();
      });
      observer.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ['data-theme'],
      });
    }

    watchSize() {
      if (typeof ResizeObserver === 'undefined') return;
      const observer = new ResizeObserver(() => {
        this.syncSurface();
        this.draw();
      });
      observer.observe(this.canvas);
    }

    start() {
      this.reset();
      this.running = true;
      this.startedAt = now();
      this.rafId = requestAnimationFrame(this.frameBound);
    }

    stop() {
      this.running = false;
      if (this.rafId) cancelAnimationFrame(this.rafId);
      this.rafId = 0;
      this.level = 0;
      this.target = 0;
      this.hold = 0;
      this.draw();
    }

    reset() {
      if (this.rafId) cancelAnimationFrame(this.rafId);
      this.rafId = 0;
      this.lastTs = 0;
      this.level = 0;
      this.target = 0;
      this.hold = 0;
      this.holdUntil = 0;
      this.bestRms = 0;
      this.lastClipAt = 0;
      this.verdict = 'idle';
    }

    push(sample) {
      if (!sample) return;
      const peak = Math.max(0, Math.min(1, Number(sample.peak) || 0));
      const rms = Math.max(0, Number(sample.rms) || 0);
      this.target = normalizeDb(peak);
      if (rms > this.bestRms) this.bestRms = rms;
      if (sample.clipped || peak >= CLIP_PEAK) this.lastClipAt = now();
    }

    frame(ts) {
      if (!this.running) return;
      this.syncSurface();

      let dt = this.lastTs ? ts - this.lastTs : 16;
      this.lastTs = ts;
      if (dt < 0) dt = 0;
      else if (dt > MAX_FRAME_MS) dt = MAX_FRAME_MS;

      const tau = this.target > this.level ? ATTACK_TAU_MS : DECAY_TAU_MS;
      this.level += (this.target - this.level) * (1 - Math.exp(-dt / tau));

      const t = now();
      if (this.level >= this.hold) {
        this.hold = this.level;
        this.holdUntil = t + HOLD_MS;
      } else if (t > this.holdUntil) {
        this.hold = Math.max(this.level, this.hold - dt * HOLD_FALL_PER_MS);
      }

      this.updateVerdict(t);
      this.draw();
      this.rafId = requestAnimationFrame(this.frameBound);
    }

    updateVerdict(t) {
      let verdict;
      if (t - this.lastClipAt < CLIP_STICKY_MS) verdict = 'clipping';
      else if (this.bestRms >= GOOD_RMS) verdict = 'good';
      else if (this.bestRms >= SIGNAL_RMS) verdict = 'quiet';
      else if (t - this.startedAt < SILENT_AFTER_MS) verdict = 'listening';
      else verdict = 'silent';

      if (verdict === this.verdict) return;
      this.verdict = verdict;
      if (this.onVerdict) this.onVerdict(verdict);
    }

    zoneColor(index, count) {
      const position = count > 1 ? index / (count - 1) : 0;
      if (position >= CLIP_FROM) return this.colors.clip;
      if (position >= HOT_FROM) return this.colors.hot;
      return this.colors.normal;
    }

    draw() {
      const ctx = this.ctx;
      const w = this.cssW;
      const h = this.cssH;
      if (!ctx || w <= 0 || h <= 0) return;

      ctx.clearRect(0, 0, w, h);

      const count = Math.max(SEG_MIN, Math.floor((w + SEG_GAP) / (SEG_W + SEG_GAP)));
      const pitch = (w + SEG_GAP) / count;
      const segW = Math.max(1, pitch - SEG_GAP);
      const litCount = Math.round(this.level * count);
      const holdIndex = this.hold > 0
        ? Math.min(count - 1, Math.max(0, Math.ceil(this.hold * count) - 1))
        : -1;

      for (let i = 0; i < count; i++) {
        const lit = i < litCount || i === holdIndex;
        ctx.fillStyle = lit ? this.zoneColor(i, count) : this.colors.track;
        const x = i * pitch;
        if (hasRoundRect) {
          ctx.beginPath();
          ctx.roundRect(x, 0, segW, h, SEG_RADIUS);
          ctx.fill();
        } else {
          ctx.fillRect(x, 0, segW, h);
        }
      }
    }
  }

  const VERDICTS = {
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

  const STOP_NOTES = {
    recording: 'Test stopped — dictation started.',
    timeout: 'Test stopped automatically after a minute.',
    window_closed: 'Test stopped.',
  };

  const MicTest = {
    els: null,
    meter: null,
    running: false,
    starting: false,
    permitted: null,
    sectionVisible: true,

    mount() {
      const root = document.getElementById('mic-test');
      if (!root) return;
      this.els = {
        root,
        card: document.getElementById('mic-test-card'),
        canvas: document.getElementById('mic-test-meter'),
        button: document.getElementById('mic-test-btn'),
        device: document.getElementById('mic-test-device'),
        status: document.getElementById('mic-test-status'),
        detail: document.getElementById('mic-test-detail'),
        sound: document.getElementById('mic-test-sound-btn'),
      };

      this.meter = new MicMeter(this.els.canvas, {
        onVerdict: (verdict) => this.renderVerdict(verdict),
      });

      this.els.button.addEventListener('click', () => this.toggle());
      this.els.sound.addEventListener('click', () => {
        ttipc.openSystemPane('com.apple.preference.sound');
      });

      window.addEventListener('blur', () => this.stop('blur'));
      window.addEventListener('pagehide', () => this.stop('unload'));
      window.addEventListener('beforeunload', () => this.stop('unload'));
      document.addEventListener('visibilitychange', () => {
        if (document.hidden) this.stop('hidden');
      });

      ttipc.listen('mictest:level', (event) => {
        if (!this.running) return;
        const payload = typeof event.payload === 'string'
          ? JSON.parse(event.payload)
          : event.payload;
        this.meter.push(payload);
      });

      ttipc.listen('mictest:stopped', (event) => {
        const payload = typeof event.payload === 'string'
          ? JSON.parse(event.payload)
          : event.payload;
        this.handleBackendStop((payload && payload.reason) || '');
      });

      ttipc.listen('sidecar:device_changed', () => this.refreshDevice());

      this.renderVerdict('idle');
      this.refreshDevice();
    },

    setPermission(granted) {
      if (this.permitted === granted) return;
      this.permitted = granted;
      if (!this.els) return;
      this.els.card.hidden = !granted;
      if (!granted) this.stop('permission');
    },

    setSectionVisible(visible) {
      this.sectionVisible = visible;
      if (!visible) this.stop('navigated');
    },

    async refreshDevice() {
      if (!this.els || this.running) return;
      try {
        const info = await ttipc.getAudioInputInfo();
        const select = document.getElementById('input-device-select');
        const selected = select && select.value;
        if (selected) {
          const opt = select.selectedOptions && select.selectedOptions[0];
          this.els.device.textContent = (opt && opt.textContent.replace(/ — current default.*$/, '')) || selected;
        } else if (info && info.device) {
          this.els.device.textContent = info.device || 'No input device';
        } else {
          this.els.device.textContent = 'No input device';
        }
      } catch (e) {
        console.error('Failed to read input device:', e);
      }
    },

    toggle() {
      if (this.running) this.stop('user');
      else this.start();
    },

    async start() {
      if (this.running || this.starting || !this.els) return;
      this.starting = true;
      this.els.button.disabled = true;
      try {
        const info = await ttipc.startMicTest();
        this.running = true;
        this.meter.start();
        if (info && info.device) {
          const rate = info.sample_rate ? ` · ${Math.round(info.sample_rate / 1000)} kHz` : '';
          this.els.device.textContent = `${info.device}${rate}`;
        }
        this.renderVerdict('listening');
      } catch (e) {
        this.renderError(e);
      } finally {
        this.starting = false;
        this.els.button.disabled = false;
        this.syncButton();
      }
    },

    stop(reason) {
      if (!this.running || !this.els) return;
      this.settle(reason);
      ttipc.stopMicTest().catch(e => console.error('Failed to stop mic test:', e));
    },

    handleBackendStop(reason) {
      if (!this.running || !this.els) return;
      this.settle(reason);
    },

    settle(reason) {
      const verdict = this.meter.verdict;
      this.running = false;
      this.meter.stop();
      // A test that ended before any sound arrived has nothing to report, so
      // the transient "Listening" prompt must not survive the stop.
      if (verdict === 'listening' || verdict === 'idle') this.renderVerdict('idle');
      this.syncButton();
      const note = STOP_NOTES[reason];
      if (note) {
        this.els.detail.textContent = note;
        this.els.detail.hidden = false;
      }
    },

    syncButton() {
      if (!this.els) return;
      const label = this.running ? 'Stop Test' : 'Test Microphone';
      this.els.button.textContent = label;
      this.els.button.setAttribute('aria-pressed', this.running ? 'true' : 'false');
      this.els.button.setAttribute(
        'aria-label',
        this.running ? 'Stop the microphone test' : 'Start the microphone test'
      );
    },

    renderVerdict(verdict) {
      if (!this.els) return;
      const copy = VERDICTS[verdict] || VERDICTS.idle;
      this.els.status.textContent = copy.text;
      this.els.status.dataset.state = copy.state;
      this.els.detail.textContent = copy.detail || '';
      this.els.detail.hidden = !copy.detail;
      this.els.sound.hidden = !copy.showSound;
      this.syncButton();
    },

    renderError(error) {
      if (!this.els) return;
      const message = error && error.message ? error.message : String(error || 'Unknown error');
      this.els.status.textContent = message;
      this.els.status.dataset.state = 'bad';
      this.els.detail.hidden = true;
      this.els.sound.hidden = true;
    },
  };

  window.MicMeter = MicMeter;
  window.MicTest = MicTest;

  if (document.getElementById('mic-test')) MicTest.mount();
})();
