<script lang="ts">
  import { onMount } from 'svelte';
  import { WaveformRenderer } from '../lib/waveform';

  let canvas: HTMLCanvasElement;
  let renderer: WaveformRenderer | null = null;

  export function setLevel(peak: number) {
    renderer?.setLevel(peak);
  }

  export function frame(ts: number) {
    renderer?.frame(ts);
  }

  export function reset() {
    renderer?.reset();
  }

  onMount(() => {
    renderer = new WaveformRenderer(canvas);
    return () => {
      renderer = null;
    };
  });
</script>

<div class="waveform" aria-hidden="true">
  <canvas
    bind:this={canvas}
    class="wave-canvas"
    width="136"
    height="30"
  ></canvas>
</div>

<style>
  .waveform {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex: 1 1 auto;
    min-width: 0;
    height: 30px;
    position: relative;
    padding-left: 10px;
    overflow: hidden;
    pointer-events: none;
  }
  .waveform::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: rgba(239, 68, 68, 0.9);
    animation: recDot 2s ease-in-out infinite;
  }
  @keyframes recDot {
    0%,
    100% {
      opacity: 1;
      transform: translateY(-50%) scale(1);
    }
    50% {
      opacity: 0.35;
      transform: translateY(-50%) scale(0.85);
    }
  }
  .wave-canvas {
    display: block;
    flex: 0 0 auto;
    width: 136px;
    height: 30px;
    color: var(--widget-text);
  }
</style>