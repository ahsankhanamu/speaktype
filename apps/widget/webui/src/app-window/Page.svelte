<script lang="ts">
  import type { Snippet } from 'svelte';

  /** `footer` holds page-wide actions (restore, clear): it stays in view at the bottom while the page scrolls. */
  let { title, lede, footer, children }: { title: string; lede?: string; footer?: Snippet; children: Snippet } = $props();
</script>

<div class="page">
  <h1>{title}</h1>
  {#if lede}<p class="lede">{lede}</p>{/if}
  {@render children()}
  {#if footer}<footer>{@render footer()}</footer>{/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 24px;
    box-sizing: border-box;
    min-height: 100%;
    padding: var(--page-top, 20px) 28px 40px;
  }
  .page:has(> footer) {
    padding-bottom: 0;
  }
  footer {
    position: sticky;
    bottom: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    margin: auto -28px 0;
    padding: 10px 28px;
    border-top: 0.5px solid var(--st-separator);
    background: var(--st-window);
  }
  h1 {
    margin: 0;
    font-size: var(--st-font-title);
    font-weight: 700;
    line-height: 1.2;
  }
  .lede {
    margin: -16px 0 0;
    color: var(--st-text-secondary);
    font-size: var(--st-font-body);
    line-height: 1.4;
  }
</style>
