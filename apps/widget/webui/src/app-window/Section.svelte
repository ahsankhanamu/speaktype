<script lang="ts">
  import type { Snippet } from 'svelte';

  /** `note` sits under the group in secondary text, as macOS explains a group. */
  let { title, note, children }: { title?: string; note?: string; children: Snippet } = $props();
</script>

<section>
  {#if title}<h2>{title}</h2>{/if}
  {@render children()}
  {#if note}<p class="note">{note}</p>{/if}
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
  .note {
    margin: 0 12px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
</style>
