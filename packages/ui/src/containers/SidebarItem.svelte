<script lang="ts">
  import type { Component } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  let {
    label,
    icon: Icon,
    tileColor,
    selected,
    badge,
    ...rest
  }: {
    label: string;
    icon: Component<{ size?: number; strokeWidth?: number }>;
    tileColor: string;
    selected: boolean;
    badge?: 'dot';
  } & Omit<HTMLButtonAttributes, 'children'> = $props();
</script>

<button type="button" class="st-sidebar-item" class:selected aria-current={selected ? 'page' : undefined} {...rest}>
  <span class="tile" style:background={tileColor}><Icon size={12} strokeWidth={2.4} /></span>
  <span class="label">{label}</span>
  {#if badge}<span class="badge" role="img" aria-label="Needs attention"></span>{/if}
</button>

<style>
  .st-sidebar-item {
    display: flex;
    align-items: center;
    gap: 8px;
    box-sizing: border-box;
    width: 100%;
    height: 28px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--st-radius-selection);
    background: transparent;
    color: var(--st-text);
    font: inherit;
    font-size: var(--st-font-sidebar);
    text-align: left;
    cursor: default;
    transition: background-color var(--st-dur-fast) var(--st-ease-out);
  }
  .st-sidebar-item:hover {
    background: var(--st-hover);
  }
  .st-sidebar-item:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .selected,
  .selected:hover {
    background: var(--st-accent);
    color: #fff;
  }
  .tile {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 5px;
    color: #fff;
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--st-accent);
  }
  .selected .badge {
    background: #fff;
  }
</style>
