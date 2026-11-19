<script lang="ts">
  import type { Snippet } from 'svelte';

  let { label, children }: { label: string; children: Snippet } = $props();
</script>

<nav class="st-sidebar" aria-label={label}>{@render children()}</nav>

<style>
  .st-sidebar {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: 8px;
    box-sizing: border-box;
    width: 220px;
    padding: 10px;
    background: var(--st-sidebar);
    font-family: var(--st-font-family);
  }
</style>
