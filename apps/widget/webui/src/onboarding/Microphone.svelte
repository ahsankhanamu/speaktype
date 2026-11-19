<script lang="ts">
  import { CircleCheck } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { Button } from '@speaktype/ui';
  import MicPicker from '../lib/app/MicPicker.svelte';
  import { openSystemPane, quitApp, requestMicrophoneAccess, type OnboardingStatus } from '../lib/ipc';
  import Page from './Page.svelte';

  let { status }: { status: OnboardingStatus } = $props();

  let settingsOpened = $state(false);
  onMount(() => {
    if (!status.microphone) requestMicrophoneAccess();
  });

  function openSettings() {
    settingsOpened = true;
    openSystemPane('com.apple.preference.security?Privacy_Microphone');
  }
</script>

<Page label="Step 1 of 4" title="Let SpeakType hear you.">
  <p>Your voice turns into text right here on this Mac. Nothing is recorded or sent anywhere.</p>
  {#if status.microphone}
    <div class="ok">
      <CircleCheck size={16} color="var(--st-success)" aria-hidden="true" />
      <div>
        <p class="done">Microphone allowed. Say something.</p>
        <p class="caption">Pick the one you'll use on the right. You can change it anytime from the widget.</p>
      </div>
    </div>
  {:else}
    <p class="caption">
      Allow microphone access when prompted, or enable SpeakType under Privacy &amp; Security → Microphone in System Settings.
    </p>
    <div class="actions">
      <Button onclick={openSettings}>Open System Settings</Button>
      {#if settingsOpened}<Button onclick={quitApp}>Quit SpeakType</Button>{/if}
    </div>
    {#if settingsOpened}
      <p class="caption">
        Microphone access isn't active yet. If you just enabled it in System Settings, quit and reopen SpeakType once.
      </p>
    {/if}
  {/if}
  {#snippet panel()}
    {#if status.microphone}
      <MicPicker />
      <p class="caption hint">Talk for a second. The bars move on the microphone that hears you.</p>
    {:else}
      <p class="caption waiting">Waiting for microphone access…</p>
    {/if}
  {/snippet}
</Page>

<style>
  .ok {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    margin-top: 8px;
  }
  .ok :global(svg) {
    margin-top: 2px;
  }
  .ok p {
    margin: 0;
  }
  .ok .done {
    color: var(--st-text);
    font-size: var(--st-font-body);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .hint {
    margin: 14px 0 0 4px;
    font-size: var(--st-font-caption);
    color: var(--st-text-secondary);
  }
  .waiting {
    align-self: center;
    font-size: var(--st-font-caption);
    color: var(--st-text-tertiary);
  }
</style>
