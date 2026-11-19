<script lang="ts">
  import { LayerRoot } from '@speaktype/ui';
  import MicPicker from '../../../src/lib/app/MicPicker.svelte';
  import ModelMenu from '../../../src/lib/app/ModelMenu.svelte';
  import SpeechModelsSheet from '../../../src/lib/app/SpeechModelsSheet.svelte';

  let { scene }: { scene: string } = $props();
</script>

<LayerRoot>
  {#if scene === 'menu'}
    <div class="panel menu-scene">
      <ModelMenu trigger="row" onopenproviders={() => {}} />
      <ModelMenu trigger="compact" onopenproviders={() => {}} />
    </div>
  {:else if scene === 'sheet'}
    <div class="panel"></div>
    <SpeechModelsSheet open />
  {:else if scene === 'mic'}
    <div class="panel mic-scene"><MicPicker /></div>
  {/if}
</LayerRoot>

<style>
  .menu-scene {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 16px;
  }
  .mic-scene {
    width: 560px;
  }
</style>
