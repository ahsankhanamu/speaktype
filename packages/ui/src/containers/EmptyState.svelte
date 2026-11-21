<script lang="ts">
  import type { Component, Snippet } from 'svelte';

  let {
    title,
    message,
    icon: Icon,
    actions,
  }: { title: string; message?: string; icon?: Component<{ size?: number; strokeWidth?: number }>; actions?: Snippet } = $props();
</script>

<div class="st-empty">
  {#if Icon}<Icon size={32} strokeWidth={1.6} />{/if}
  <h3>{title}</h3>
  {#if message}<p>{message}</p>{/if}
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</div>

<style>
  .st-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 32px 24px;
    color: var(--st-text-secondary);
    font-family: var(--st-font-family);
    text-align: center;
  }
  h3 {
    margin: 4px 0 0;
    color: var(--st-text);
    font-size: var(--st-font-section);
    font-weight: 600;
  }
  p {
    max-width: 320px;
    margin: 0;
    font-size: var(--st-font-info);
    line-height: 1.45;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
</style>
