<script lang="ts">
  import { Pause, Play } from '@lucide/svelte';
  import { IconButton } from '@speaktype/ui';
  import { decodeWav, loadClip, peaks, player, type Clip } from './audio.svelte';

  let { clip, label }: { clip: Clip; label: string } = $props();

  const BARS = 120;
  let bars = $state<number[]>([]);
  let failed = $state(false);

  $effect(() => {
    let live = true;
    loadClip(clip)
      .then((bytes) => live && (bars = peaks(decodeWav(bytes).samples, BARS)))
      .catch(() => live && (failed = true));
    return () => {
      live = false;
    };
  });
  $effect(() => () => player.isPlaying(clip) && player.stop());

  const playing = $derived(player.isPlaying(clip));
  const loudest = $derived(Math.max(0.05, ...bars));

  function seek(event: MouseEvent) {
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    void player.toggle(clip, Math.min(1, Math.max(0, (event.clientX - box.left) / box.width)));
  }
</script>

<div class="wave">
  <IconButton label={playing ? `Pause ${label}` : `Play ${label}`} icon={playing ? Pause : Play} size={28} onclick={() => player.toggle(clip)} />
  <button type="button" class="bars" aria-label="Play {label} from here" onclick={seek} disabled={failed || !bars.length}>
    {#if failed}
      <span class="note">No audio kept for this</span>
    {:else}
      {#each bars as peak, i}
        <span class="bar" class:played={playing && i / BARS < player.progress} style:height="{Math.max(6, (peak / loudest) * 100)}%"></span>
      {/each}
    {/if}
  </button>
</div>

<style>
  .wave {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .bars {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 1px;
    height: 36px;
    padding: 0;
    border: 0;
    background: none;
  }
  .bar {
    flex: 1;
    min-width: 1px;
    border-radius: 1px;
    background: var(--st-text-tertiary);
  }
  .bar.played {
    background: var(--st-accent);
  }
  .note {
    color: var(--st-text-tertiary);
    font-size: var(--st-font-caption);
  }
</style>
