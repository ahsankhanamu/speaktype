<script lang="ts">
  import { ChevronRight, Trash2 } from '@lucide/svelte';
  import { Button, EmptyState, Group, IconButton, Row, confirmAlert } from '@speaktype/ui';
  import { liveQuery } from '../../lib/data';
  import { formatDuration } from '../../lib/format';
  import { clearDebugSessions, deleteDebugSession, getDebugSessions } from '../../lib/ipc';
  import Section from '../Section.svelte';
  import DebugChunk from './debug/DebugChunk.svelte';
  import Waveform from './debug/Waveform.svelte';
  import { player } from './debug/audio.svelte';

  const records = liveQuery('debug-sessions', getDebugSessions, { invalidateOn: ['dictation-state'] });
  type Record = NonNullable<typeof records.data>[number];

  let open = $state<number | null>(null);
  /** Opened chunks of the open record; opening another record starts closed. */
  let openChunks = $state(new Set<number>());

  const title = (record: Record) =>
    [new Date(record.startedAt ?? '').toLocaleString(), record.model, record.durationSecs ? formatDuration(record.durationSecs) : '']
      .filter(Boolean)
      .join(' · ');



  $effect(() => () => player.stop());

  async function remove(id: number) {
    await deleteDebugSession(id);
    if (open === id) open = null;
    await records.refetch();
  }

  async function clear() {
    const confirmed = await confirmAlert({
      title: 'Clear all debug records?',
      message: 'Your History is kept. Only these decoding records are removed.',
      confirmLabel: 'Clear',
      destructive: true,
    });
    if (!confirmed) return;
    await clearDebugSessions();
    open = null;
    await records.refetch();
  }
</script>

<Section title="Recent dictation debug records" lede="Click a record to see what was heard and how each part was decoded.">
  <Group>
    {#each records.data ?? [] as record (record.id)}
      {@const expanded = open === record.id}
      {@const toggle = () => {
        open = expanded ? null : (record.id ?? null);
        openChunks = new Set();
      }}
      {@const chunkIds = (record.chunks ?? []).map((chunk) => chunk.idx ?? 0)}
      {@const allOpen = chunkIds.length > 0 && chunkIds.every((idx) => openChunks.has(idx))}
      <div class="block" class:expanded>
      <!-- The whole row opens the record; the buttons inside keep their own clicks. -->
      <div
        class="record"
        role="button"
        tabindex="0"
        aria-expanded={expanded}
        onclick={(event) => !(event.target as Element).closest('button') && toggle()}
        onkeydown={(event) => (event.key === 'Enter' || event.key === ' ') && event.target === event.currentTarget && (event.preventDefault(), toggle())}
      >
      <Row
        kind="control"
        label={title(record)}
        caption="{record.finalChars} characters · {record.chunks?.length ?? 0} chunks · {record.passes?.length ?? 0} decode passes"
      >
        {#snippet trailing()}
          <IconButton
            label={expanded ? 'Hide details' : 'Show details'}
            icon={ChevronRight}
            size={28}
            class={expanded ? 'turned' : ''}
            aria-expanded={expanded}
            onclick={toggle}
          />
          <IconButton label="Delete record" icon={Trash2} size={28} onclick={() => remove(record.id ?? 0)} />
        {/snippet}
      </Row>
      </div>
      {#if expanded}
        <div class="detail">
          <p class="text">{record.finalText || 'No text came out of this dictation.'}</p>
          {#if record.id != null}
            <Waveform clip={{ sessionId: record.id, start: 0, end: record.durationSecs ?? 0 }} label="the whole recording" />
          {/if}
          {#if record.chunks?.length}
            <div class="chunk-tools">
              <span>{record.chunks.length} {record.chunks.length === 1 ? 'chunk' : 'chunks'}</span>
              <Button size={28} onclick={() => (openChunks = allOpen ? new Set() : new Set(chunkIds))}>{allOpen ? 'Collapse all' : 'Expand all'}</Button>
            </div>
            <div class="chunks">
              {#each record.chunks as chunk (chunk.idx)}
                {@const idx = chunk.idx ?? 0}
                <DebugChunk
                  sessionId={record.id ?? 0}
                  {chunk}
                  passes={(record.passes ?? []).filter((pass) => pass.chunkIdx === chunk.idx)}
                  bind:open={() => openChunks.has(idx), (on) => (openChunks = on ? new Set([...openChunks, idx]) : new Set([...openChunks].filter((i) => i !== idx)))}
                />
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      </div>
    {:else}
      <EmptyState title="No records yet" message="The last 25 dictations show up here with how each was decoded." />
    {/each}
  </Group>
  {#if records.data?.length}
    <!-- Stays in view at the bottom while the records scroll past. -->
    <div class="footer">
      <span>{records.data.length} {records.data.length === 1 ? 'record' : 'records'}</span>
      <Button size={28} variant="destructive" onclick={clear}>Clear records…</Button>
    </div>
  {/if}
</Section>

<style>
  /* One record: its row, and when open, its detail in a tinted panel. */
  .block + .block {
    border-top: 0.5px solid var(--st-separator);
  }
  .block.expanded {
    margin: 4px 0;
    border-radius: var(--st-radius-card);
    background: var(--st-fill);
    box-shadow: inset 0 0 0 0.5px var(--st-separator);
  }
  .block.expanded + .block {
    border-top: 0;
  }
  .record {
    border-radius: var(--st-radius-card);
  }
  /* The open record's row stays in view while its chunks scroll past. */
  .block.expanded .record {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--st-group);
    border-radius: var(--st-radius-card) var(--st-radius-card) 0 0;
    box-shadow: 0 0.5px 0 var(--st-separator);
  }
  .chunk-tools {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 12px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
  .record:hover {
    background: var(--st-hover);
  }
  .record:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .footer {
    position: sticky;
    bottom: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 2px;
    border-top: 0.5px solid var(--st-separator);
    background: var(--st-window);
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
  :global(.st-icon-btn.turned svg) {
    transform: rotate(90deg);
  }
  .detail {
    padding: 4px 14px 12px;
    font-size: var(--st-font-caption);
    user-select: text;
  }
  .text {
    margin: 0 0 10px;
    color: var(--st-text);
    font-size: var(--st-font-info);
    line-height: 1.45;
  }
  .chunks {
    margin-top: 4px;
  }
</style>
