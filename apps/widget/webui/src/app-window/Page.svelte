<script lang="ts">
  import type { Snippet } from 'svelte';

  let { title, lede, children }: { title: string; lede?: string; children: Snippet } = $props();
</script>

<div class="page">
  <h1>{title}</h1>
  {#if lede}<p class="lede">{lede}</p>{/if}
  {@render children()}
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
