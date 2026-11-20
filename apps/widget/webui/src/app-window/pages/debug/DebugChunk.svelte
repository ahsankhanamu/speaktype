<script lang="ts">
  import { ChevronRight, Play } from '@lucide/svelte';
  import type { ChunkEvent, PassEvent } from '../../../lib/bindings';
  import { formatDuration } from '../../../lib/format';
  import Waveform from './Waveform.svelte';
  import { player } from './audio.svelte';

  let {
    sessionId,
    chunk,
    passes,
    open = $bindable(false),
  }: { sessionId: number; chunk: ChunkEvent; passes: PassEvent[]; open?: boolean } = $props();

  const STATUS: Record<string, { label: string; tone: 'ok' | 'bad' | 'quiet' }> = {
    accepted: { label: 'Accepted', tone: 'ok' },
    accepted_cleaned: { label: 'Accepted, cleaned', tone: 'ok' },
    rejected: { label: 'Rejected', tone: 'bad' },
    failed: { label: 'Failed', tone: 'bad' },
    silence: { label: 'Silence', tone: 'quiet' },
    pipelined: { label: 'Queued', tone: 'quiet' },
  };

  const start = $derived(chunk.startSecs ?? 0);
  const end = $derived(chunk.endSecs ?? start + (chunk.durationSecs ?? 0));
  const status = $derived(STATUS[chunk.status ?? ''] ?? { label: chunk.status ?? 'Unknown', tone: 'quiet' as const });
  const span = (a: number, b: number) => `${formatDuration(Math.floor(a))}–${formatDuration(Math.ceil(b))}`;
</script>

<div class="chunk">
  <button type="button" class="head" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="chevron" class:open><ChevronRight size={12} aria-hidden="true" /></span>
    <span class="name">Chunk {(chunk.idx ?? 0) + 1}</span>
    <span class="time">{span(start, end)}</span>
    <span class="badge {status.tone}">{status.label}</span>
    <span class="meta">{chunk.chars ?? 0} chars{chunk.reason ? ` · ${chunk.reason}` : ''}</span>
  </button>
  {#if open}
    <div class="body">
      <Waveform clip={{ sessionId, start, end }} label="chunk {(chunk.idx ?? 0) + 1}" />
      {#each passes as pass (pass.pass)}
        <div class="pass">
          <p class="pass-head">
            Pass {pass.pass ?? 1}{pass.temperature != null ? ` · temperature ${pass.temperature}` : ''}{pass.score != null ? ` · score ${pass.score.toFixed(2)}` : ''}{pass.dropped ? ` · ${pass.dropped} dropped` : ''}{pass.reason ? ` · ${pass.reason}` : ''}
          </p>
          {#each pass.segments ?? [] as segment}
            {@const from = start + (segment.start ?? 0)}
            {@const to = start + (segment.end ?? segment.start ?? 0)}
            {@const clip = { sessionId, start: from, end: to }}
            <button type="button" class="segment" class:bad={segment.bad} class:playing={player.isPlaying(clip)} onclick={() => player.toggle(clip)}>
              <Play size={10} aria-hidden="true" />
              <span class="seg-time">{from.toFixed(1)}s</span>
              <span class="seg-text">{segment.text?.trim() || '(no words)'}</span>
              {#if segment.flags?.length}<span class="flags">{segment.flags.join(', ')}</span>{/if}
            </button>
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .chunk {
    border-top: 0.5px solid var(--st-separator);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 4px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--st-text);
    font: inherit;
    font-size: var(--st-font-info);
    text-align: left;
  }
  .head:hover {
    background: var(--st-hover);
  }
  .head:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .chevron {
    display: inline-flex;
    color: var(--st-text-tertiary);
    transition: rotate var(--st-dur-fast) var(--st-ease-out);
  }
  .chevron.open {
    rotate: 90deg;
  }
  .name {
    font-weight: 500;
  }
  .time,
  .meta {
    color: var(--st-text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .badge {
    padding: 1px 7px;
    border-radius: 999px;
    font-size: var(--st-font-caption);
    font-weight: 500;
  }
  .badge.ok {
    background: color-mix(in srgb, var(--st-success) 16%, transparent);
    color: var(--st-success-text);
  }
  .badge.bad {
    background: color-mix(in srgb, var(--st-danger) 16%, transparent);
    color: var(--st-danger-text);
  }
  .badge.quiet {
    background: var(--st-fill);
    color: var(--st-text-secondary);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 4px 4px 12px 24px;
  }
  .pass-head {
    margin: 0 0 2px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
  .segment {
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    padding: 3px 6px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--st-text);
    font: inherit;
    font-size: var(--st-font-info);
    text-align: left;
  }
  .segment:hover {
    background: var(--st-hover);
  }
  .segment.playing {
    background: color-mix(in srgb, var(--st-accent) 14%, transparent);
  }
  .segment :global(svg) {
    flex: none;
    color: var(--st-text-tertiary);
  }
  .seg-time {
    flex: none;
    color: var(--st-text-tertiary);
    font-variant-numeric: tabular-nums;
  }
  .seg-text {
    flex: 1;
    user-select: text;
  }
  .segment.bad .seg-text {
    color: var(--st-danger-text);
    text-decoration: line-through;
  }
  .flags {
    flex: none;
    color: var(--st-text-tertiary);
    font-size: var(--st-font-caption);
  }
</style>
