<script lang="ts">
  import { MicOff, Pencil } from '@lucide/svelte';
  import PillButton from './PillButton.svelte';
  import type { useSession } from './session.svelte';

  let { session }: { session: ReturnType<typeof useSession> } = $props();

  const BARS = 5;
  const report = (error: unknown) => console.error('[widget] session', error);
</script>

<span class="live" class:muted={session.muted} aria-hidden="true"></span>
<span class="timer" aria-label="Session time">{session.elapsed}</span>
<button
  type="button"
  class="level"
  data-action
  aria-label={session.muted ? 'Unmute' : 'Mute'}
  aria-pressed={session.muted}
  onclick={() => session.toggleMute().catch(report)}
>
  {#if session.muted}
    <MicOff size={13} strokeWidth={2} aria-hidden="true" />
  {:else}
    {#each { length: BARS } as _, bar}
      <span class="bar" class:on={bar < Math.round(session.level * BARS)} style:height="calc({4 + bar * 2}px * var(--k))"></span>
    {/each}
  {/if}
</button>
<span class="marks" aria-label="{session.marks} {session.marks === 1 ? 'mark' : 'marks'}">
  <Pencil size={12} strokeWidth={2} aria-hidden="true" />{session.marks}
</span>
<PillButton label="Stop session" tone="danger" onclick={() => session.stop().catch(report)}>
  {#snippet icon()}<span class="square"></span>{/snippet}
</PillButton>

<style>
  .live {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--st-danger);
    animation: breathe 1.6s ease-in-out infinite;
  }
  .live.muted {
    background: rgba(255, 255, 255, 0.35);
    animation: none;
  }
  @keyframes breathe {
    50% {
      opacity: 0.45;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .live {
      animation: none;
    }
  }
  .timer {
    color: rgba(255, 255, 255, 0.75);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .level {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: calc(24px * var(--k));
    padding: 0 6px;
    border: 0;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.8);
  }
  .level:hover {
    background: rgba(255, 255, 255, 0.14);
  }
  .level:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .bar {
    width: 2px;
    border-radius: 1px;
    background: rgba(255, 255, 255, 0.3);
  }
  .bar.on {
    background: rgba(255, 255, 255, 0.9);
  }
  .marks {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: rgba(255, 255, 255, 0.75);
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
