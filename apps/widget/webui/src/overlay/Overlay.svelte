<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { liveQuery, useEvent } from '../lib/data';
  import { commands, getSettings } from '../lib/ipc';
  import { FADE_MS, HOLD_MS, grow, inkColor, mid, strokeWidth, type Bounds } from './ink';

  type Point = { x: number; y: number };

  const settings = liveQuery('settings', getSettings, { invalidateOn: ['settings-changed'] });

  let canvas = $state<HTMLCanvasElement>();
  let colorProbe = $state<HTMLElement>();
  let drawing = $state(false);
  let fading = $state(false);
  let stroke: Point[] | null = null;
  let clearTimer: ReturnType<typeof setTimeout> | undefined;
  /** Everything drawn since the last mark, in window points. */
  let ink: Bounds | null = null;
  // The overlay's label is `overlay-<display index>`.
  const display = Number(getCurrentWindow().label.split('-')[1] ?? 0);

  useEvent<{ active: boolean }>('overlay:draw', ({ active }) => {
    drawing = active;
    clearTimeout(clearTimer);
    if (active) {
      fading = false;
      return;
    }
    stroke = null;
    // The mark is done 900 ms after the key comes up: it is captured into the
    // session with the ink still on screen, then the ink fades away.
    clearTimer = setTimeout(async () => {
      const mark = ink;
      ink = null;
      if (mark) await commands.overlayMarkComplete(display, mark).catch((error) => console.error('[overlay] mark', error));
      if (drawing) return;
      fading = true;
      clearTimer = setTimeout(clear, FADE_MS);
    }, HOLD_MS);
  });

  function clear() {
    const ctx = canvas?.getContext('2d');
    if (canvas && ctx) ctx.clearRect(0, 0, canvas.width, canvas.height);
    fading = false;
  }

  // The canvas matches the display in device pixels, so ink is sharp on Retina.
  $effect(() => {
    if (!canvas) return;
    const fit = () => {
      const ratio = window.devicePixelRatio || 1;
      canvas!.width = Math.round(innerWidth * ratio);
      canvas!.height = Math.round(innerHeight * ratio);
      canvas!.getContext('2d')?.setTransform(ratio, 0, 0, ratio, 0, 0);
    };
    fit();
    window.addEventListener('resize', fit);
    return () => window.removeEventListener('resize', fit);
  });
  $effect(() => () => clearTimeout(clearTimer));

  function pen() {
    const ctx = canvas!.getContext('2d')!;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.lineWidth = strokeWidth(settings.data?.ink_thickness);
    ctx.strokeStyle = colorProbe ? getComputedStyle(colorProbe).color : '#ef643c';
    return ctx;
  }

  function onPointerDown(event: PointerEvent) {
    if (!drawing || event.button !== 0) return;
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
    stroke = [{ x: event.clientX, y: event.clientY }];
    // The first stroke starts a session (a no-op when one is running).
    if (!ink) commands.sessionStart(null).catch((error) => console.error('[overlay] session', error));
    ink = grow(ink, stroke[0]);
    const ctx = pen();
    ctx.beginPath();
    ctx.arc(event.clientX, event.clientY, ctx.lineWidth / 2, 0, Math.PI * 2);
    ctx.fillStyle = ctx.strokeStyle;
    ctx.fill();
  }

  function onPointerMove(event: PointerEvent) {
    if (!stroke) return;
    const points = event.getCoalescedEvents?.().length ? event.getCoalescedEvents() : [event];
    const ctx = pen();
    for (const p of points) {
      const next = { x: p.clientX, y: p.clientY };
      const last = stroke[stroke.length - 1];
      const before = stroke[stroke.length - 2] ?? last;
      ctx.beginPath();
      const from = mid(before, last);
      const to = mid(last, next);
      ctx.moveTo(from.x, from.y);
      ctx.quadraticCurveTo(last.x, last.y, to.x, to.y);
      ctx.stroke();
      stroke.push(next);
      ink = grow(ink, next);
    }
  }

  const endStroke = () => (stroke = null);
</script>

<span class="probe" bind:this={colorProbe} style:color={inkColor(settings.data?.ink_color)}></span>
<canvas
  bind:this={canvas}
  class:drawing
  class:fading
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={endStroke}
  onpointercancel={endStroke}
></canvas>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    overflow: hidden;
    background: transparent;
    user-select: none;
    -webkit-user-select: none;
  }
  canvas {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    transition: opacity var(--st-dur-ink) var(--st-ease-out);
    /* Its own compositing layer: WebKit otherwise never puts a canvas drawn on
       a fully transparent page on screen. */
    transform: translateZ(0);
  }
  canvas.drawing {
    cursor: crosshair;
  }
  canvas.fading {
    opacity: 0;
  }
  .probe {
    display: none;
  }
</style>
