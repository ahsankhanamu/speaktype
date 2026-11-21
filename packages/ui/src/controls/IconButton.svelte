<script lang="ts">
  import type { Component } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Tooltip from '../floating/Tooltip.svelte';

  let {
    label,
    shortcut,
    icon: Icon,
    size = 32,
    class: className = '',
    ...rest
  }: { label: string; shortcut?: string[]; icon: Component<{ size?: number }>; size?: 28 | 32 } & Omit<HTMLButtonAttributes, 'children'> = $props();
</script>

<Tooltip {label} {shortcut}>
  {#snippet children(tooltipProps)}
    <button type="button" {...tooltipProps} {...rest} class="st-icon-btn {className}" style:--size="{size}px" aria-label={label}>
      <Icon size={size / 2} />
    </button>
  {/snippet}
</Tooltip>

<style>
  .st-icon-btn {
    display: inline-grid;
    place-items: center;
    box-sizing: border-box;
    width: var(--size);
    height: var(--size);
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--st-text-secondary);
    transition:
      background-color var(--st-dur-fast) var(--st-ease-out),
      color var(--st-dur-fast) var(--st-ease-out);
  }
  .st-icon-btn:hover:not(:disabled) {
    background: var(--st-fill);
    color: var(--st-text);
  }
  .st-icon-btn:active:not(:disabled) {
    background: var(--st-fill-pressed);
  }
  .st-icon-btn:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .st-icon-btn:disabled {
    opacity: 0.4;
  }
</style>
