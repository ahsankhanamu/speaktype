<script lang="ts">
  import { Button, LayerRoot, Progress, Toast, toast } from '@speaktype/ui';
  import { step as slide } from '@speaktype/ui/motion';
  import { liveQuery } from '../lib/data';
  import { finishOnboarding, getOnboardingStatus, getSettings, saveSettings, type Settings } from '../lib/ipc';
  import Accessibility from './Accessibility.svelte';
  import { useOnboardingModels } from './downloads.svelte';
  import Microphone from './Microphone.svelte';
  import SpeechModel from './SpeechModel.svelte';
  import TryIt from './TryIt.svelte';
  import Welcome from './Welcome.svelte';

  const LAST = 4;
  const POLL_MS = 2000;

  const settings = liveQuery('settings', getSettings, { invalidateOn: ['settings-changed'] });
  const status = liveQuery('onboarding-status', getOnboardingStatus, { invalidateOn: ['sidecar:permissions'], pollMs: POLL_MS });
  const models = useOnboardingModels();

  let current = $state<number>();
  let direction = $state<1 | -1>(1);
  let finishing = $state(false);

  $effect(() => {
    if (current === undefined && settings.data) {
      current = Math.min(LAST, settings.data.onboarding_step ?? 0);
    }
  });

  async function go(to: number) {
    direction = to > current! ? 1 : -1;
    current = to;
    try {
      await saveSettings({ ...(await getSettings()), onboarding_step: to } as Settings);
    } catch (error) {
      toast.show({ message: String(error) });
    }
  }

  async function finish() {
    finishing = true;
    try {
      await finishOnboarding();
    } catch (error) {
      toast.show({ message: String(error) });
      finishing = false;
    }
  }

  let started = false;
  $effect(() => {
    if (!current || started || !models.loaded) return;
    started = true;
    if (!models.stops.some((row) => row.model.downloaded)) models.choose(models.stops[0]);
  });

  let wasGranted = true;
  $effect(() => {
    const granted = status.data?.accessibility ?? true;
    if (current === 2 && wasGranted === false && granted) go(3);
    wasGranted = granted;
  });

  const ready = $derived(
    current !== undefined &&
      status.data !== undefined &&
      [true, status.data.microphone, status.data.accessibility, true, status.data.model && !finishing][current],
  );
</script>

<LayerRoot>
  <main>
    <div class="stage">
      {#if current !== undefined && status.data}
        {#key current}
          <div class="slot" in:slide={{ direction }} out:slide={{ direction: -direction as 1 | -1 }}>
            {#if current === 0}
              <Welcome />
            {:else if current === 1}
              <Microphone status={status.data} />
            {:else if current === 2}
              <Accessibility status={status.data} />
            {:else if current === 3}
              <SpeechModel {models} />
            {:else}
              <TryIt hotkey={settings.data?.shortcuts?.dictation ?? ''} />
            {/if}
          </div>
        {/key}
      {/if}
    </div>
    <footer>
      <div class="note">
        {#if current && models.footer}
          {#if models.percent !== undefined}<span class="bar"><Progress value={models.percent / 100} label="Download" /></span>{/if}
          <span>{models.footer}</span>
        {/if}
      </div>
      {#if current}<Button size={32} onclick={() => go(current! - 1)}>Back</Button>{/if}
      <Button variant="primary" size={32} disabled={!ready} onclick={current === LAST ? finish : () => go(current! + 1)}>
        {current === 0 ? 'Get started' : current === LAST ? 'Done' : 'Continue'}
      </Button>
    </footer>
  </main>
  <Toast />
</LayerRoot>

<style>
  main {
    display: grid;
    grid-template-rows: 1fr auto;
    height: 100vh;
    background: var(--st-window);
    color: var(--st-text);
    font-family: var(--st-font-family);
    font-size: var(--st-font-body);
    -webkit-font-smoothing: antialiased;
    user-select: none;
  }
  .stage {
    display: grid;
    overflow: hidden;
    padding: 14px 14px 0 0;
  }
  .slot {
    grid-area: 1 / 1;
    min-height: 0;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 20px 16px 56px;
  }
  .note {
    display: flex;
    flex: 1;
    align-self: stretch;
    align-items: center;
    gap: 12px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-info);
  }
  .bar {
    width: 60px;
  }
</style>
