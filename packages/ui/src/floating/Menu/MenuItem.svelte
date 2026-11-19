<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Check } from '@lucide/svelte';
  import { DropdownMenu } from 'bits-ui';
  import '../menu.css';

  let {
    label,
    icon,
    value,
    disabled = false,
    destructive = false,
    onSelect,
  }: { label: string; icon?: Snippet; value?: string; disabled?: boolean; destructive?: boolean; onSelect?: () => void } = $props();
</script>

{#snippet layout(checked: boolean)}
  <span class="check">{#if checked}<Check size={12} strokeWidth={2.2} aria-hidden="true" />{/if}</span>
  {#if icon}<span class="icon">{@render icon()}</span>{/if}
  <span class="text" data-st-label>{label}</span>
{/snippet}

{#if value !== undefined}
  <DropdownMenu.RadioItem {value} {disabled} {onSelect} class="st-menu-item">
    {#snippet children({ checked })}{@render layout(checked)}{/snippet}
  </DropdownMenu.RadioItem>
{:else}
  <DropdownMenu.Item {disabled} {onSelect} class={['st-menu-item', destructive && 'destructive']}>
    {@render layout(false)}
  </DropdownMenu.Item>
{/if}

<style>
  .check {
    display: grid;
    flex: none;
    place-items: center;
    width: var(--menu-check-col, 14px);
  }
  .icon {
    display: inline-flex;
    flex: none;
    align-items: center;
    width: var(--menu-icon-col, 18px);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
</style>
