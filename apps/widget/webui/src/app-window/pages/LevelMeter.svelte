<script lang="ts">
  const BARS = 5;

  /** `level` is 0..1. */
  let { level }: { level: number } = $props();

  const lit = $derived(Math.round(Math.min(1, level) * BARS));
</script>

<span class="meter" role="meter" aria-label="Input level" aria-valuemin="0" aria-valuemax="1" aria-valuenow={level}>
  {#each { length: BARS } as _, bar}
    <span class="bar" class:on={bar < lit} style:height="{6 + bar * 2.5}px"></span>
  {/each}
</span>

<style>
  .meter {
    display: inline-flex;
    align-items: flex-end;
    gap: 2px;
    height: 16px;
  }
  .bar {
    width: 3px;
    border-radius: 1.5px;
    background: var(--st-track);
  }
  .bar.on {
    background: var(--st-success);
  }
</style>
