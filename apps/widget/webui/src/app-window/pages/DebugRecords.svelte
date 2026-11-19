<script lang="ts">
  import { Trash2 } from '@lucide/svelte';
  import { Button, EmptyState, Group, IconButton, Row } from '@speaktype/ui';
  import { liveQuery } from '../../lib/data';
  import { formatDuration } from '../../lib/format';
  import { clearDebugSessions, deleteDebugSession, getDebugSessions } from '../../lib/ipc';
  import Section from '../Section.svelte';

  const records = liveQuery('debug-sessions', getDebugSessions, { invalidateOn: ['dictation-state', 'sidecar:pasted'] });

  const title = (record: NonNullable<typeof records.data>[number]) =>
    [new Date(record.startedAt ?? '').toLocaleString(), record.model, record.durationSecs ? formatDuration(record.durationSecs) : '']
      .filter(Boolean)
      .join(' · ');

  async function remove(id: number) {
    await deleteDebugSession(id);
    await records.refetch();
  }

  async function clear() {
    await clearDebugSessions();
    await records.refetch();
  }
</script>

<Section title="Recent dictation debug records">
  <Group>
    {#each records.data ?? [] as record (record.id)}
      <Row
        kind="control"
        label={title(record)}
        caption="{record.finalChars} characters · {record.chunks?.length ?? 0} chunks · {record.passes?.length ?? 0} decode passes"
      >
        {#snippet trailing()}<IconButton label="Delete record" icon={Trash2} size={28} onclick={() => remove(record.id)} />{/snippet}
      </Row>
    {:else}
      <EmptyState title="No records yet" message="The last 25 dictations show up here with how each was decoded." />
    {/each}
  </Group>
  {#if records.data?.length}
    <div class="clear"><Button size={28} variant="destructive" onclick={clear}>Clear records</Button></div>
  {/if}
</Section>

<style>
  .clear {
    display: flex;
    justify-content: flex-end;
  }
</style>
