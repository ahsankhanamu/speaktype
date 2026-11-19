<script lang="ts">
  import { Mic, Pencil } from '@lucide/svelte';
  import { truncate } from '@speaktype/ui';
  import { metaOf, type Entry } from './history';

  let { item, selected, onselect }: { item: Entry; selected: boolean; onselect: () => void } = $props();

  const title = $derived(item.kind === 'dictation' ? item.entry.text : item.session.name);
  const snippet = $derived(item.kind === 'session' ? (item.session.snippet ?? '') : '');
</script>

<button type="button" class="row" class:selected aria-current={selected ? 'true' : undefined} onclick={onselect}>
  <span class="tile">
    {#if item.kind === 'dictation'}<Mic size={16} aria-hidden="true" />{:else}<Pencil size={16} aria-hidden="true" />{/if}
  </span>
  <span class="text">
    <span class="title" use:truncate={{ mode: 'tail', full: title }}></span>
    {#if snippet}<span class="snippet" use:truncate={{ mode: 'tail', full: snippet }}></span>{/if}
    <span class="meta">{metaOf(item)}</span>
  </span>
</button>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 0 12px;
    border: 0;
    border-radius: var(--st-radius-card);
    background: none;
    color: var(--st-text);
    font: inherit;
    text-align: left;
    cursor: default;
  }
  .row:hover:not(.selected) {
    background: var(--st-hover);
  }
  .row:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .row.selected {
    background: var(--st-accent);
    color: #fff;
  }
  .tile {
    display: grid;
    flex: none;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 9px;
    background: var(--st-control);
    color: var(--st-text-secondary);
  }
  .selected .tile {
    background: rgb(255 255 255 / 0.22);
    color: #fff;
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .title {
    font-size: var(--st-font-body);
    font-weight: 600;
  }
  .snippet,
  .meta {
    color: var(--st-text-secondary);
    font-size: var(--st-font-info);
  }
  .meta {
    color: var(--st-text-tertiary);
    font-size: var(--st-font-caption);
  }
  .selected .snippet,
  .selected .meta {
    color: rgb(255 255 255 / 0.82);
  }
</style>
