<script lang="ts">
  import { Mic } from '@lucide/svelte';

  let { px }: { px: number } = $props();
</script>

<span class="desk">
  <span class="mic" style:width="{px}px" style:height="{px}px"><Mic size={px * 0.42} strokeWidth={2} aria-hidden="true" /></span>
</span>

<style>
  .desk {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    background: linear-gradient(160deg, #5a6a90, #2f3a58);
  }
  .mic {
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: #1c1c1e;
    color: #fff;
  }
</style>
