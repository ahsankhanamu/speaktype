<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

<div class="st-group">{@render children()}</div>

<style>
  .st-group {
    border-radius: var(--st-radius-card);
    background: var(--st-group);
    box-shadow: inset 0 0 0 0.5px var(--st-separator);
  }
  .st-group > :global(.st-row + .st-row)::before {
    content: '';
    position: absolute;
    top: 0;
    right: 0;
    left: 14px;
    height: 0.5px;
    background: var(--st-separator);
  }
</style>
