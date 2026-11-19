<script lang="ts">
  import type { Snippet } from 'svelte';

  let { title, children }: { title?: string; children: Snippet } = $props();
</script>

<section>
  {#if title}<h2>{title}</h2>{/if}
  {@render children()}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h2 {
    margin: 0;
    font-size: var(--st-font-section);
    font-weight: 600;
  }
</style>
