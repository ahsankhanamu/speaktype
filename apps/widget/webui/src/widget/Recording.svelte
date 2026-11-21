<script lang="ts">
  import { X } from '@lucide/svelte';
  import { useEvent } from '../lib/data';
  import { formatDuration } from '../lib/format';
  import PillButton from './PillButton.svelte';

  let { onstop, oncancel }: { onstop: () => void; oncancel: () => void } = $props();

  const BARS = 9;
  const STEP_MS = 60;
  const GAIN = 15;
  const SMOOTHING = 0.35;

  let heights = $state<number[]>(Array(BARS).fill(0));
  let elapsed = $state(0);
  let level = 0;

  useEvent<{ level: number }>('sidecar:audio_level', (event) => {
    level = Math.min(event.level * GAIN, 1);
  });

  // Runs only while this component exists, which is only while recording.
  $effect(() => {
    const begin = performance.now();
    let last = begin;
    let smooth = 0;
    let frame = 0;
    const tick = (now: number) => {
      elapsed = Math.floor((now - begin) / 1000);
      smooth += (level - smooth) * SMOOTHING;
      if (now - last >= STEP_MS) {
        last = now;
        heights = [...heights.slice(1), smooth];
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  });
</script>

<PillButton label="Cancel" shortcut={['Escape']} onclick={oncancel}>
  {#snippet icon()}<X size={10} strokeWidth={2.4} aria-hidden="true" />{/snippet}
</PillButton>
<span class="waveform" data-waveform aria-hidden="true">
  {#each heights as height}
    <span class="bar" style:height="calc((4px + {height * 14}px) * var(--k))"></span>
  {/each}
</span>
<span class="timer">{formatDuration(elapsed)}</span>
<PillButton label="Stop" tone="danger" onclick={onstop}>
  {#snippet icon()}<span class="square"></span>{/snippet}
</PillButton>

<style>
  .waveform {
    display: flex;
    align-items: center;
    gap: 2px;
    height: calc(18px * var(--k));
  }
  .bar {
    width: 2px;
    border-radius: 1px;
    background: rgba(255, 255, 255, 0.85);
  }
  .timer {
    color: rgba(255, 255, 255, 0.6);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .square {
    width: calc(8px * var(--k));
    height: calc(8px * var(--k));
    border-radius: 2px;
    background: var(--st-danger);
  }
</style>
