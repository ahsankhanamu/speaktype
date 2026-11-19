<script lang="ts">
  import { Clock } from '@lucide/svelte';
  import { Button, EmptyState, Keycap, SearchField, Segmented, Skeleton, formatShortcut } from '@speaktype/ui';
  import { liveQuery } from '../../lib/data';
  import { getHistory, sessionList } from '../../lib/ipc';
  import { useSettings } from '../settings.svelte';
  import HistoryDetail from './history/HistoryDetail.svelte';
  import { buildRows, type Entry, type Filter } from './history/history';
  import HistoryRow from './history/HistoryRow.svelte';
  import VirtualList from './history/VirtualList.svelte';

  const HEADER_PX = 32;
  const ROW_PX = 66;

  const settings = useSettings();
  const history = liveQuery('history', getHistory, { invalidateOn: ['dictation-state', 'settings-changed'] });
  const sessions = liveQuery('sessions', () => sessionList(null), { invalidateOn: ['sessions-changed'] });

  let filter = $state<Filter>('all');
  let query = $state('');
  let selectedKey = $state('');

  const items = $derived<Entry[]>([
    ...(history.data?.entries ?? []).map((entry, index): Entry => ({ kind: 'dictation', key: `d${index}`, index, at: new Date(entry.timestamp), entry })),
    ...(sessions.data ?? []).map((session): Entry => ({ kind: 'session', key: `s${session.id}`, at: new Date(session.startedAtMs), session })),
  ]);
  const rows = $derived(buildRows(items, filter, query, new Date()));
  const entries = $derived(rows.filter((row): row is Entry => row.kind !== 'header'));
  const selected = $derived(entries.find((entry) => entry.key === selectedKey));
  const loaded = $derived(history.data !== undefined && sessions.data !== undefined);

  $effect(() => {
    if (!selected && entries.length) selectedKey = entries[0].key;
  });

  const hotkey = $derived(formatShortcut((settings.data?.shortcuts?.dictation ?? 'Super+Control').split('+')));
</script>

{#if loaded && !items.length}
  <div class="empty">
    <EmptyState title="Nothing here yet" message="Everything you dictate or capture in a session shows up here, so you can find it later." icon={Clock}>
      {#snippet actions()}
        <span class="hint">
          Dictate with {#each hotkey as key}<Keycap {key} />{/each} · Start a session with
          {#each formatShortcut(['Option', 'Super', 'K']) as key}<Keycap {key} />{/each}
        </span>
      {/snippet}
    </EmptyState>
  </div>
{:else if history.error || sessions.error}
  <div class="empty">
    <EmptyState title="Couldn’t load your history" message="Something went wrong reading it from this Mac.">
      {#snippet actions()}<Button onclick={() => (history.refetch(), sessions.refetch())}>Try again</Button>{/snippet}
    </EmptyState>
  </div>
{:else}
  <div class="history">
    <section class="list" aria-label="Results">
      <div class="tools">
        <SearchField bind:value={query} placeholder="Search everything you said" />
        <Segmented
          label="Show"
          bind:value={filter}
          items={[{ value: 'all', label: 'All' }, { value: 'sessions', label: 'Sessions' }, { value: 'dictations', label: 'Dictations' }]}
        />
      </div>
      {#if !loaded}
        <div class="loading"><Skeleton lines={5} height="44px" /></div>
      {:else if !entries.length}
        <EmptyState title="No matches" message="Nothing in your history matches that." />
      {:else}
        <VirtualList items={rows} label="History" height={(row) => (row.kind === 'header' ? HEADER_PX : ROW_PX)}>
          {#snippet row(item)}
            {#if item.kind === 'header'}
              <h3>{item.label}</h3>
            {:else}
              <div class="pad"><HistoryRow {item} selected={item.key === selectedKey} onselect={() => (selectedKey = item.key)} /></div>
            {/if}
          {/snippet}
        </VirtualList>
      {/if}
    </section>
    <section class="detail">
      {#if selected}
        {#key selected.key}
          <HistoryDetail item={selected} newest={selected.kind === 'dictation' && selected.index === 0} onchange={() => history.refetch()} />
        {/key}
      {/if}
    </section>
  </div>
{/if}

<style>
  .history {
    display: flex;
    height: 100%;
  }
  .list {
    display: flex;
    flex: none;
    flex-direction: column;
    box-sizing: border-box;
    width: 300px;
    border-right: 0.5px solid var(--st-separator);
    background: var(--st-sidebar);
  }
  .tools {
    display: flex;
    flex-direction: column;
    gap: 10px;
    /* Clear the window's overlay title bar. */
    padding: calc(var(--titlebar-h, 44px) + 4px) 12px 8px;
  }
  .loading {
    padding: 12px;
  }
  h3 {
    margin: 0;
    padding: 10px 20px 0;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
    font-weight: 600;
  }
  .pad {
    box-sizing: border-box;
    height: 100%;
    padding: 0 8px;
  }
  .detail {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
  }
  .empty {
    display: grid;
    place-items: center;
    height: 100%;
  }
  .hint {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-info);
  }
</style>
