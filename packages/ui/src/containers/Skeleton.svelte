<script lang="ts">
  let { width = '100%', height = '14px', lines = 1 }: { width?: string; height?: string; lines?: number } = $props();
</script>

<div class="st-skeleton" aria-hidden="true">
  {#each { length: lines } as _, i}
    <span style:width={lines > 1 && i === lines - 1 ? '60%' : width} style:height></span>
  {/each}
</div>

<style>
  .st-skeleton {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  span {
    display: block;
    max-width: 100%;
    border-radius: 6px;
    background: linear-gradient(90deg, var(--st-fill) 25%, var(--st-fill-hover) 50%, var(--st-fill) 75%) 0 0 / 200% 100%;
    animation: shimmer 1.4s linear infinite;
  }
  @keyframes shimmer {
    to {
      background-position: -200% 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    span {
      animation: none;
      background: var(--st-fill);
    }
  }
</style>
