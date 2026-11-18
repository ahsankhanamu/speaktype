<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

<div class="group">{@render children()}</div>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
</style>
