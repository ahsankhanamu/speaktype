<script lang="ts">
  import type { Snippet } from 'svelte';

  /** `lede` introduces the section under its title; `note` explains the group under it, as macOS does. */
  let { title, lede, note, children }: { title?: string; lede?: string; note?: string; children: Snippet } = $props();
</script>

<section>
  {#if title}<h2>{title}</h2>{/if}
  {#if lede}<p class="lede">{lede}</p>{/if}
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
  .lede {
    margin: -4px 0 0;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
  .note {
    margin: 0 12px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
</style>
