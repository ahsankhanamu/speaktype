<script lang="ts">
  import { Button } from '@speaktype/ui';
  import ModelSlider from '../lib/app/ModelSlider.svelte';
  import SpeechModelsSheet from '../lib/app/SpeechModelsSheet.svelte';
  import { plainModel } from '../lib/app/models';
  import type { OnboardingModels } from './downloads.svelte';
  import Page from './Page.svelte';

  let { models }: { models: OnboardingModels } = $props();

  let index = $state(-1);
  let sheetOpen = $state(false);

  const recommended = $derived(models.stops.find((row) => plainModel(row.model.id).recommended));

  $effect(() => {
    if (index >= 0 || !models.stops.length) return;
    const inUse = models.stops.findIndex((row) => row.model.id === models.activeId && row.model.downloaded);
    index = inUse > 0 ? inUse : Math.max(0, models.stops.findIndex((row) => row === recommended));
    models.choose(models.stops[index]);
  });
</script>

<Page label="Step 3 of 4" title="Pick how SpeakType listens.">
  <p>Faster models start quicker. More accurate ones catch names, accents and noisy rooms. You can change this anytime.</p>
  {#if recommended}
    <div class="recommended">
      <p class="caption">Recommended for your Mac</p>
      <p class="pick">{recommended.name} · {plainModel(recommended.model.id).size}</p>
    </div>
  {/if}
  {#snippet panel()}
    {#if index >= 0}
      <ModelSlider rows={models.stops} bind:index activeId={models.activeId} oncommit={models.choose} />
      <Button variant="plain" onclick={() => (sheetOpen = true)}>See all models…</Button>
    {/if}
  {/snippet}
</Page>
<SpeechModelsSheet bind:open={sheetOpen} />

<style>
  .recommended {
    margin-top: 12px;
  }
  .recommended p {
    margin: 0;
  }
  .recommended .pick {
    color: var(--st-text);
    font-size: var(--st-font-body);
  }
</style>
