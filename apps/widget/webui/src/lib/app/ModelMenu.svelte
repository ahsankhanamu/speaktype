<script lang="ts">
  import { Check, Download } from '@lucide/svelte';
  import { Select } from '@speaktype/ui';
  import { Select as Bits } from 'bits-ui';
  import { plainModel } from './models';
  import { runModelAction, useModels } from './models.svelte';
  import SpeechModelsSheet from './SpeechModelsSheet.svelte';

  let { trigger, onopenproviders }: { trigger: 'row' | 'compact'; onopenproviders: () => void } = $props();

  const models = useModels();
  let sheetOpen = $state(false);

  const active = $derived(models.rows.find((row) => row.model.id === models.activeId));
  const listed = $derived(
    models.rows.filter((row) => plainModel(row.model.id).tier !== 'more' || row.state.action !== 'download'),
  );
  const moreNames = $derived(
    models.rows
      .filter((row) => plainModel(row.model.id).tier === 'more' && row.state.action === 'download')
      .map((row) => row.name)
      .join(', '),
  );
  const triggerLabel = $derived(
    !active ? 'Speech model' : trigger === 'row' ? `${active.name} · ${active.state.label}` : active.name,
  );

  const openers: Record<string, () => void> = {
    more: () => (sheetOpen = true),
    server: () => onopenproviders(),
    cloud: () => onopenproviders(),
  };

  function pick(value: string) {
    if (value in openers) openers[value]();
    else if (value !== models.activeId) runModelAction('use', value);
  }
</script>

<Select.Root bind:value={() => models.activeId, pick}>
  <Select.Trigger class="model-trigger {trigger}" aria-label="Speech model">{triggerLabel}</Select.Trigger>
  <Select.Content>
    <div class="body">
      <p class="heading">On this Mac</p>
      {#each listed as row (row.model.id)}
        {@const plain = plainModel(row.model.id)}
        <Bits.Item class="st-menu-item" value={row.model.id} label={row.name}>
          <span class="check">
            {#if row.model.id === models.activeId}<Check size={13} strokeWidth={2.2} aria-hidden="true" />{/if}
          </span>
          <span class="name" data-st-label>{row.name}</span>
          {#if plain.recommended}<span class="state tag">Recommended</span>{/if}
          <span class="state">
            {#if row.state.action === 'download'}
              <Download size={11} strokeWidth={2.4} aria-hidden="true" />{plain.size.replace('~', '')}
            {:else}
              {row.state.label}
            {/if}
          </span>
        </Bits.Item>
      {/each}
      <Bits.Item class="st-menu-item" value="more" label="More Whisper models…">
        <span class="check"></span>
        <span class="name">More Whisper models…</span>
        {#if moreNames}<span class="state">{moreNames}</span>{/if}
      </Bits.Item>
      <Select.Separator />
      <p class="heading">Your server</p>
      <Bits.Item class="st-menu-item" value="server" label="Connect a whisper.cpp server…">
        <span class="check"></span>
        <span class="name">Connect a whisper.cpp server…</span>
        <span class="state">by URL</span>
      </Bits.Item>
      <Select.Separator />
      <p class="heading">Cloud</p>
      <Bits.Item class="st-menu-item" value="cloud" label="Add a cloud service…">
        <span class="check"></span>
        <span class="name">Add a cloud service…</span>
        <span class="state">OpenAI, Groq, Deepgram</span>
      </Bits.Item>
    </div>
  </Select.Content>
</Select.Root>

<SpeechModelsSheet bind:open={sheetOpen} />

<style>
  :global(.model-trigger.row) {
    min-width: 260px;
  }
  :global(.model-trigger.compact) {
    min-width: 0;
    height: var(--st-h-row-control);
    border-radius: 14px;
  }
  .body {
    width: 360px;
  }
  .heading {
    margin: 0;
    padding: 8px 10px 4px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
    font-weight: 600;
  }
  .check {
    display: grid;
    flex: none;
    place-items: center;
    width: 14px;
  }
  .name {
    flex: none;
    white-space: nowrap;
  }
  .tag {
    margin-left: -4px;
  }
  .state:last-child {
    margin-left: auto;
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
    white-space: nowrap;
  }
  :global(.st-menu-item[data-highlighted]) .state {
    color: inherit;
  }
</style>
