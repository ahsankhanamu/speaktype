<script lang="ts">
  import type { Snippet } from 'svelte';

  let { tone, children }: { tone: 'recording' | 'active' | 'warning' | 'success' | null; children: Snippet } = $props();
</script>

<div class="halo" style:box-shadow={tone ? `var(--st-halo-${tone})` : undefined}>{@render children()}</div>

<style>
  .halo {
    display: inline-flex;
    border-radius: 999px;
    transition: box-shadow var(--st-dur-slow) var(--st-ease-out);
  }
</style>
