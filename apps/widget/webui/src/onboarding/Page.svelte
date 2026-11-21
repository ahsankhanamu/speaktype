<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    label,
    title,
    eyebrow,
    children,
    panel,
  }: { label?: string; title: string; eyebrow?: Snippet; children: Snippet; panel: Snippet } = $props();
</script>

<section class="page">
  <div class="copy">
    {#if eyebrow}{@render eyebrow()}{/if}
    {#if label}<p class="label">{label}</p>{/if}
    <h1>{title}</h1>
    {@render children()}
  </div>
  <div class="panel">{@render panel()}</div>
</section>

<style>
  .page {
    display: grid;
    grid-template-columns: 400px 1fr;
    height: 100%;
    min-height: 0;
  }
  .copy {
    display: flex;
    flex-direction: column;
    gap: 18px;
    box-sizing: border-box;
    padding: 74px 32px 24px 56px;
    min-height: 0;
  }
  .label {
    margin: 0 0 -6px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-body);
  }
  h1 {
    margin: 0;
    font-size: 28px;
    font-weight: 600;
    line-height: 1.12;
    letter-spacing: -0.02em;
  }
  :global(:where(.copy p)) {
    margin: 0;
    color: var(--st-text-secondary);
    font-size: 16px;
    line-height: 1.42;
  }
  .copy :global(.caption) {
    font-size: var(--st-font-caption);
    line-height: 1.4;
  }
  .panel {
    display: flex;
    flex-direction: column;
    justify-content: center;
    box-sizing: border-box;
    min-width: 0;
    padding: 28px;
    border-radius: var(--st-radius-card-lg);
    background: var(--st-inset);
  }
</style>
