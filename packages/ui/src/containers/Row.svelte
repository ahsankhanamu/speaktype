<script lang="ts">
  import type { Component, Snippet } from 'svelte';

  let {
    kind,
    label,
    caption,
    icon: Icon,
    tileColor,
    trailing,
  }: {
    kind: 'info' | 'control' | 'icon';
    label: string;
    caption?: string;
    icon?: Component<{ size?: number; strokeWidth?: number }>;
    tileColor?: string;
    trailing?: Snippet;
  } = $props();

  const height = { info: 'var(--st-row-info)', control: 'var(--st-row-control)', icon: 'var(--st-row-icon)' };
</script>

<div class="st-row {kind}" style:min-height={height[kind]}>
  {#if Icon}
    <span class="tile" style:background={tileColor}><Icon size={16} strokeWidth={2.2} /></span>
  {/if}
  <div class="text">
    <span class="label">{label}</span>
    {#if caption}<span class="caption">{caption}</span>{/if}
  </div>
  {#if trailing}<div class="trailing">{@render trailing()}</div>{/if}
</div>

<style>
  .st-row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    box-sizing: border-box;
    padding: 8px 14px 8px 16px;
    font-family: var(--st-font-family);
  }
  .icon {
    gap: 14px;
  }
  .tile {
    display: grid;
    flex: none;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    color: #fff;
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .label {
    color: var(--st-text);
    font-size: var(--st-font-body);
  }
  .info .label {
    font-size: var(--st-font-info);
  }
  .caption {
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
  .trailing {
    display: flex;
    flex: none;
    align-items: center;
    gap: 10px;
  }
</style>
