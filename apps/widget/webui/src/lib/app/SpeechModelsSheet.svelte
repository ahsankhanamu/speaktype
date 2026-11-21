<script lang="ts">
  import { Button, Sheet } from '@speaktype/ui';
  import { formatByteSize } from '../format';
  import ModelRow from './ModelRow.svelte';
  import ModelSlider from './ModelSlider.svelte';
  import { runModelAction, useModels } from './models.svelte';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  const models = useModels();
  const activeIndex = $derived(models.rows.findIndex((row) => row.model.id === models.activeId));

  let index = $state(0);
  $effect(() => {
    if (activeIndex >= 0) index = activeIndex;
  });
</script>

<Sheet bind:open title="Speech models">
  <p class="lead">
    Slide toward Accurate for better results with names, accents and noisy rooms. Faster models download quicker and use less memory.
  </p>
  {#if models.loaded}
    <ModelSlider
      rows={models.rows}
      bind:index
      activeId={models.activeId}
      oncommit={(row) => runModelAction('use', row.model.id)}
    />
    <div class="list" role="list" aria-label="All speech models">
      {#each models.rows as row (row.model.id)}
        <div role="listitem"><ModelRow {row} onremove={() => models.remove(row.model.id)} /></div>
      {/each}
    </div>
  {/if}
  {#snippet footer()}
    <span class="meta">
      Using {formatByteSize(models.bytesOnDisk) || '0 MB'} · English or multilingual follows your Language setting
    </span>
    <span class="spacer"></span>
    <Button variant="primary" onclick={() => (open = false)}>Done</Button>
  {/snippet}
</Sheet>

<style>
  .lead {
    margin: 0 0 14px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-body);
    line-height: 1.45;
  }
  .list {
    padding-top: 10px;
  }
  .list :global([role='listitem']) {
    position: relative;
  }
  .list :global([role='listitem'] + [role='listitem'])::before {
    content: '';
    position: absolute;
    top: 0;
    right: 0;
    left: 52px;
    height: 0.5px;
    background: var(--st-separator);
  }
  .meta {
    color: var(--st-text-tertiary);
    font-size: 13px;
  }
  .spacer {
    flex: 1;
  }
</style>
