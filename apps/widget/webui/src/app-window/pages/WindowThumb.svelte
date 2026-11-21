<script lang="ts">
  let { mode }: { mode: 'auto' | 'light' | 'dark' } = $props();
</script>

{#snippet half(tone: 'l' | 'd')}
  <span class="half {tone}"><i class="side"></i><i class="line"></i><i class="line short"></i></span>
{/snippet}

<span class="thumb">
  {#if mode !== 'dark'}{@render half('l')}{/if}
  {#if mode !== 'light'}{@render half('d')}{/if}
</span>

<style>
  .thumb {
    display: flex;
    width: 100%;
    height: 100%;
  }
  .half {
    position: relative;
    flex: 1;
    overflow: hidden;
  }
  .l {
    background: #f2f2f4;
    --block: #d9d9de;
  }
  .d {
    background: #1e1e1e;
    --block: #3a3a3c;
  }
  i {
    position: absolute;
    border-radius: 4px;
    background: var(--block);
  }
  .side {
    top: 12px;
    bottom: 10px;
    left: 8px;
    width: 22px;
  }
  .line {
    top: 12px;
    right: 8px;
    left: 38px;
    height: 7px;
  }
  .short {
    top: 24px;
    right: 24px;
  }
</style>
