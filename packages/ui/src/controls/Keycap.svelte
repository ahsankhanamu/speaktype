<script lang="ts" module>
  const GLYPHS: Record<string, string> = {
    return: 'M13 3.5V8a1.5 1.5 0 0 1-1.5 1.5H3.5M6.5 6.5l-3 3 3 3',
    delete: 'M6 3.5h6.5A1.5 1.5 0 0 1 14 5v6a1.5 1.5 0 0 1-1.5 1.5H6L2 8zM8.5 6.2l3 3.6M11.5 6.2l-3 3.6',
    tab: 'M2 8h8.5M7.5 5l3 3-3 3M13.5 3.5v9',
  };
</script>

<script lang="ts">
  let { key }: { key: string } = $props();
  const glyph = $derived(GLYPHS[key]);
  // Mac keyboards label Escape "esc"; the ⎋ glyph reads as "reload".
  const label = $derived(key === 'escape' ? 'esc' : key);
</script>

<kbd class="st-keycap" aria-label={glyph ? key : undefined}>
  {#if glyph}
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={glyph} /></svg>
  {:else}
    {label}
  {/if}
</kbd>

<style>
  .st-keycap {
    --size: var(--keycap-size, 24px);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    min-width: var(--size);
    height: var(--size);
    padding: 0 calc(var(--size) / 4);
    border-radius: var(--st-radius-tile);
    background: var(--st-keycap);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.14), 0 1px 0 rgba(0, 0, 0, 0.65), 0 1px 3px rgba(0, 0, 0, 0.3);
    color: var(--st-text);
    font: inherit;
    font-size: calc(var(--size) * 0.54);
    font-weight: 500;
    line-height: 1;
  }
</style>
