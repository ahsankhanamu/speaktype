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
    /* macOS Tahoe: a floating panel inset from the window, with its own edge. */
    margin: 8px 0 8px 8px;
    padding: 10px;
    border-radius: 16px;
    background: var(--st-sidebar);
    box-shadow: 0 0 0 1px var(--st-sidebar-edge), 0 1px 3px var(--st-shadow);
    overflow-y: auto;
    font-family: var(--st-font-family);
  }
</style>
