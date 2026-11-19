<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();
</script>

<div class="paper">{@render children()}</div>

<style>
  .paper {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 14px;
    background: #fbfbf9;
    box-shadow: 0 0 0 0.5px rgba(0, 0, 0, 0.18), 0 8px 24px rgba(0, 0, 0, 0.2);
    color: #1d1d1f;
  }
</style>
