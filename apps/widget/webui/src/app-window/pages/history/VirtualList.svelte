<script lang="ts" generics="T extends { key: string }">
  import type { Snippet } from 'svelte';

  const OVERSCAN_PX = 320;

  let { items, height, row, label }: { items: T[]; height: (item: T) => number; row: Snippet<[T]>; label: string } = $props();

  let viewport = $state(600);
  let top = $state(0);

  const offsets = $derived.by(() => {
    const result = [0];
    for (const item of items) result.push(result[result.length - 1] + height(item));
    return result;
  });
  const total = $derived(offsets[offsets.length - 1]);

  /** First index whose end edge is below `edge`. */
  function indexAt(edge: number) {
    let [lo, hi] = [0, items.length];
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (offsets[mid + 1] > edge) hi = mid;
      else lo = mid + 1;
    }
    return lo;
  }

  const first = $derived(indexAt(top - OVERSCAN_PX));
  const last = $derived(Math.min(items.length, indexAt(top + viewport + OVERSCAN_PX) + 1));
</script>

<div class="scroller" role="list" aria-label={label} bind:clientHeight={viewport} onscroll={(event) => (top = event.currentTarget.scrollTop)}>
  <div class="space" style:height="{total}px">
    {#each { length: last - first } as _, k (items[first + k].key)}
      {@const item = items[first + k]}
      <div class="slot" role="listitem" style:transform="translateY({offsets[first + k]}px)" style:height="{height(item)}px">
        {@render row(item)}
      </div>
    {/each}
  </div>
</div>

<style>
  .scroller {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .space {
    position: relative;
  }
  .slot {
    position: absolute;
    top: 0;
    right: 0;
    left: 0;
  }
</style>
