const BAR_WIDTH = 3;
const BAR_GAP = 2;
const BAR_PITCH = BAR_WIDTH + BAR_GAP;
const BAR_INTERVAL_MS = 55;
const BAR_MIN_H = 3;
const EDGE_PAD = 2;
const FADE_RATIO = 1 / 3;
const HEAD_TAU_MS = 40;
const MAX_FRAME_MS = 250;
const FALLBACK_COLOR = 'rgba(255, 255, 255, 0.9)';

const hasRoundRect =
  typeof Path2D !== 'undefined' &&
  typeof Path2D.prototype.roundRect === 'function';

function addBar(path: Path2D, x: number, y: number, h: number) {
  if (hasRoundRect) path.roundRect(x, y, BAR_WIDTH, h, BAR_WIDTH / 2);
  else path.rect(x, y, BAR_WIDTH, h);
}

export class WaveformRenderer {
  private ctx: CanvasRenderingContext2D;
  private cssW: number;
  private cssH: number;
  private dpr = 0;
  private color = FALLBACK_COLOR;
  private count: number;
  private amps: Float32Array;
  private head = 0;
  private acc = 0;
  private lastTs = 0;
  private level = 0;
  private headTarget = 0;
  private headAmp = 0;

  constructor(private canvas: HTMLCanvasElement) {
    this.ctx = canvas.getContext('2d') as CanvasRenderingContext2D;
    this.cssW = canvas.width || 136;
    this.cssH = canvas.height || 30;
    this.count = Math.max(1, Math.floor(this.cssW / BAR_PITCH));
    this.amps = new Float32Array(this.count);
    this.syncSurface();
    this.refreshColor();
    this.watchTheme();
  }

  private syncSurface() {
    const dpr = window.devicePixelRatio || 1;
    if (dpr === this.dpr) return;
    this.dpr = dpr;
    this.canvas.width = Math.round(this.cssW * dpr);
    this.canvas.height = Math.round(this.cssH * dpr);
    this.canvas.style.width = this.cssW + 'px';
    this.canvas.style.height = this.cssH + 'px';
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    this.ctx.fillStyle = this.color;
  }

  private refreshColor() {
    let color = '';
    try {
      color = getComputedStyle(this.canvas).color;
    } catch (e) {
      color = '';
    }
    this.color = color || FALLBACK_COLOR;
    this.ctx.fillStyle = this.color;
  }

  private watchTheme() {
    if (typeof MutationObserver === 'undefined') return;
    const observer = new MutationObserver(() => this.refreshColor());
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    });
  }

  setLevel(peak: number) {
    const v = peak > 1 ? 1 : peak > 0 ? peak : 0;
    this.level = v;
    if (v > this.headTarget) this.headTarget = v;
  }

  private push(amp: number) {
    this.amps[this.head] = amp;
    this.head = (this.head + 1) % this.count;
  }

  private ampAt(age: number) {
    let i = (this.head - 1 - age) % this.count;
    if (i < 0) i += this.count;
    return this.amps[i];
  }

  frame(ts: number) {
    this.syncSurface();

    let dt = this.lastTs ? ts - this.lastTs : 0;
    this.lastTs = ts;
    if (dt < 0) dt = 0;
    else if (dt > MAX_FRAME_MS) dt = MAX_FRAME_MS;

    this.headAmp += (this.headTarget - this.headAmp) * (1 - Math.exp(-dt / HEAD_TAU_MS));

    this.acc += dt;
    while (this.acc >= BAR_INTERVAL_MS) {
      this.acc -= BAR_INTERVAL_MS;
      this.push(this.headAmp);
      this.headTarget = this.level;
    }

    this.draw(this.acc / BAR_INTERVAL_MS);
  }

  private draw(progress: number) {
    const ctx = this.ctx;
    const w = this.cssW;
    const h = this.cssH;
    const cy = h / 2;
    const span = h - EDGE_PAD * 2 - BAR_MIN_H;
    const offset = progress * BAR_PITCH;
    const fadeW = w * FADE_RATIO;

    ctx.clearRect(0, 0, w, h);

    const solid = new Path2D();
    let hasSolid = false;

    for (let age = -1; age < this.count; age++) {
      const x = w - BAR_WIDTH / 2 - age * BAR_PITCH - offset;
      if (x < -BAR_WIDTH) break;
      const amp = age < 0 ? this.headAmp : this.ampAt(age);
      const barH = BAR_MIN_H + span * amp;
      const left = x - BAR_WIDTH / 2;
      const top = cy - barH / 2;
      const alpha = x >= fadeW ? 1 : x / fadeW;
      if (alpha >= 0.995) {
        addBar(solid, left, top, barH);
        hasSolid = true;
      } else if (alpha > 0.01) {
        const faded = new Path2D();
        addBar(faded, left, top, barH);
        ctx.globalAlpha = alpha;
        ctx.fill(faded);
      }
    }

    ctx.globalAlpha = 1;
    if (hasSolid) ctx.fill(solid);
  }

  reset() {
    this.amps.fill(0);
    this.head = 0;
    this.acc = 0;
    this.lastTs = 0;
    this.level = 0;
    this.headTarget = 0;
    this.headAmp = 0;
    this.syncSurface();
    this.refreshColor();
    this.draw(0);
  }
}