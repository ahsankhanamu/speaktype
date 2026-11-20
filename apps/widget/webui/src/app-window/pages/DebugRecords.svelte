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

<Section title="Recent dictation debug records" note="Click a record to see what was heard and how each part was decoded.">
  <Group>
    {#each records.data ?? [] as record (record.id)}
      {@const expanded = open === record.id}
      {@const toggle = () => (open = expanded ? null : (record.id ?? null))}
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
            <Waveform clip={{ sessionId: record.id, start: null, end: null }} label="the whole recording" />
          {/if}
          {#if record.chunks?.length}
            <div class="chunks">
              {#each record.chunks as chunk (chunk.idx)}
                <DebugChunk sessionId={record.id ?? 0} {chunk} passes={(record.passes ?? []).filter((pass) => pass.chunkIdx === chunk.idx)} />
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    {:else}
      <EmptyState title="No records yet" message="The last 25 dictations show up here with how each was decoded." />
    {/each}
  </Group>
  {#if records.data?.length}
    <div class="clear"><Button size={28} variant="destructive" onclick={clear}>Clear records…</Button></div>
  {/if}
</Section>

<style>
  .record {
    border-radius: var(--st-radius-card);
  }
  .record:hover {
    background: var(--st-hover);
  }
  .record:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .clear {
    display: flex;
    justify-content: flex-end;
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
    margin-top: 10px;
  }
</style>
