<script lang="ts">
  import { AudioLines, ChevronsUpDown, Download, Pause, X } from '@lucide/svelte';
  import { Button, DropdownMenu, IconButton, MenuItem, ProgressRing, Row, Spinner, StatusDot } from '@speaktype/ui';
  import { DropdownMenu as Bits } from 'bits-ui';
  import { plainModel } from './models';
  import { runModelAction, type ModelRow } from './models.svelte';

  const TILES: Record<string, string> = {
    tiny: 'linear-gradient(180deg, #8e8e93, #636366)',
    base: 'linear-gradient(180deg, #64d2ff, #0a84ff)',
    small: 'linear-gradient(180deg, #5ac8fa, #007aff)',
    medium: 'linear-gradient(180deg, #bf5af2, #5e5ce6)',
    'large-v3-turbo': 'linear-gradient(180deg, #ff9f0a, #ff6a3d)',
    'large-v3': 'linear-gradient(180deg, #ff6961, #ff375f)',
  };

  let { row, onremove }: { row: ModelRow; onremove: () => void } = $props();

  const id = $derived(row.model.id);
  const { state } = $derived(row);
  const plain = $derived(plainModel(id));
  const active = $derived(state.tone === 'success');
</script>

<div class="model" class:active>
  <Row
    kind="icon"
    label={row.name}
    caption="{plain.detail}{plain.recommended ? ' · Recommended' : ''} · {plain.size}"
    icon={AudioLines}
    tileColor={TILES[id.replace(/\.en$/, '')]}
  >
    {#snippet trailing()}
      {#if state.action === 'pause'}
        <ProgressRing value={(state.percent ?? 0) / 100} size={20} label="Downloading {row.name}" />
        <span class="state">{state.detail}</span>
        <IconButton label="Pause download" icon={Pause} size={28} onclick={() => runModelAction('pause', id)} />
      {:else if state.action === 'download'}
        <Button size={28} onclick={() => runModelAction('download', id)}><Download size={12} strokeWidth={2.4} />Download</Button>
      {:else}
        {#if state.tone === 'accent'}
          <Spinner />
        {:else if state.tone !== 'neutral'}
          <StatusDot tone={state.tone} label={state.label} />
        {/if}
        <span class="state" class:strong={active}>{state.label}</span>
        {#if state.action === 'use'}
          <DropdownMenu.Root>
            <DropdownMenu.Trigger class="model-options" aria-label="{row.name} model options">
              <ChevronsUpDown size={13} strokeWidth={2} aria-hidden="true" />
            </DropdownMenu.Trigger>
            <DropdownMenu.Content>
              <MenuItem label="Use" onSelect={() => runModelAction('use', id)} />
              <Bits.Item class="st-menu-item danger" onSelect={onremove}>Delete</Bits.Item>
            </DropdownMenu.Content>
          </DropdownMenu.Root>
        {:else if state.action === 'resume'}
          <Button size={28} onclick={() => runModelAction('resume', id)}>Resume</Button>
        {:else if state.action === 'retry'}
          <Button size={28} onclick={() => runModelAction('retry', id)}>Try again</Button>
        {:else if state.action === 'cancel'}
          <IconButton label="Cancel download" icon={X} size={28} onclick={() => runModelAction('cancel', id)} />
        {/if}
      {/if}
    {/snippet}
  </Row>
</div>

<style>
  .model.active {
    margin: 0 -8px;
    padding: 0 8px;
    border-radius: 14px;
    background: color-mix(in srgb, var(--st-accent) 10%, transparent);
  }
  .state {
    color: var(--st-text-secondary);
    font-size: var(--st-font-body);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .state.strong {
    color: var(--st-text);
  }
  :global(.model-options) {
    display: inline-grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--st-fill);
    color: var(--st-text);
  }
  :global(.st-menu-item.danger:not([data-highlighted])) {
    color: var(--st-danger-text);
  }
  :global(.model-options:focus-visible) {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
</style>
