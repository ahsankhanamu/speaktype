<script lang="ts">
  import { Tooltip } from 'bits-ui';
  import Keycap from '../controls/Keycap.svelte';
  import { formatShortcut } from '../controls/shortcut';
  import { pop } from '../motion';

  let { label, shortcut, customAnchor }: { label: string; shortcut?: string[]; customAnchor?: Element | null } = $props();
</script>

<Tooltip.Content forceMount sideOffset={6} {customAnchor}>
  {#snippet child({ props, wrapperProps, open })}
    {#if open}
      <div {...wrapperProps}>
        <div {...props} role="tooltip" class="st-tooltip" transition:pop>
          {label}
          {#each shortcut ? formatShortcut(shortcut) : [] as key}<Keycap {key} />{/each}
        </div>
      </div>
    {/if}
  {/snippet}
</Tooltip.Content>

<style>
  .st-tooltip {
    --keycap-size: 18px;
    z-index: var(--st-z-tooltip);
    display: flex;
    align-items: center;
    gap: 6px;
    max-width: 320px;
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--st-popover);
    box-shadow: var(--st-elevation-popover);
    color: var(--st-text);
    font-family: var(--st-font-family);
    font-size: var(--st-font-caption);
    overflow-wrap: anywhere;
  }
</style>
