<script lang="ts">
  import { ChevronLeft, CircleCheck, Mic, Zap } from '@lucide/svelte';
  import { Button, Group, Row, Spinner, Switch } from '@speaktype/ui';
  import { openSystemPane, quitApp, type OnboardingStatus } from '../lib/ipc';
  import Page from './Page.svelte';

  let { status }: { status: OnboardingStatus } = $props();

  const HELP_AFTER_MS = 15_000;

  const granted = $derived(status.accessibility);
  let waited = $state(false);
  let opened = $state(false);
  $effect(() => {
    if (granted) return;
    const timer = setTimeout(() => (waited = true), HELP_AFTER_MS);
    return () => clearTimeout(timer);
  });

  const expanded = $derived(status.is_dev || opened);
  const unbundled = $derived(status.is_dev && !status.is_bundled);
  const owner = $derived(status.responsible_app || 'the terminal app that launched it');
</script>

<Page label="Step 2 of 4" title="Let SpeakType type for you.">
  <p>macOS calls this Accessibility. It lets SpeakType put your words into the app you're using.</p>
  {#if status.accessibility}
    <div class="ok">
      <CircleCheck size={16} color="var(--st-success)" aria-hidden="true" />
      <p class="done">Accessibility allowed.</p>
    </div>
  {:else}
    <div class="actions">
      <Button onclick={() => openSystemPane('com.apple.preference.security?Privacy_Accessibility')}>Open System Settings</Button>
    </div>
    <p class="caption waiting"><Spinner />Waiting for you to switch it on. We'll continue automatically.</p>
    {#if expanded}
      {#if unbundled}
        <p class="caption">
          This is a bare executable, so macOS assigns Accessibility to {owner} rather than to SpeakType. Run "make dev" instead — it launches a real SpeakType Dev app that holds its own grant.
        </p>
      {:else if status.is_dev}
        <p class="caption">
          Dev build: enable "{status.responsible_app || 'SpeakType Dev'}" in the Accessibility list, then Quit &amp; Reopen. Remove any older entry with the same name first.
        </p>
      {:else}
        <p class="caption">
          System Settings can show SpeakType ON for an old or debug copy while this app is still blocked. Remove SpeakType from the Accessibility list, quit every SpeakType process, open only /Applications/SpeakType.app, turn it ON, then use Quit &amp; Reopen once.
        </p>
      {/if}
      {#if status.exe_path}
        <p class="caption path">{status.responsible_app_path ? `Grant to: ${status.responsible_app_path}` : status.exe_path}</p>
      {/if}
      <div class="actions"><Button variant="plain" class="flush" onclick={quitApp}>Quit &amp; Reopen SpeakType</Button></div>
    {:else if waited}
      <div class="actions"><Button variant="plain" class="flush" onclick={() => (opened = true)}>Not working?</Button></div>
    {/if}
  {/if}
  {#snippet panel()}
    <div class="mock" inert aria-hidden="true">
      <div class="bar">
        <span class="lights"><i></i><i></i><i></i></span>
        <span class="back"><ChevronLeft size={12} strokeWidth={2.4} />Accessibility</span>
      </div>
      <p class="caption">Allow the apps below to control your computer.</p>
      <Group>
        <Row kind="control" icon={Zap} tileColor="#6b5ce7" label="Raycast">
          {#snippet trailing()}<Switch label="Raycast" checked />{/snippet}
        </Row>
        <div class="target">
          <Row kind="control" icon={Mic} tileColor="#48484a" label="SpeakType">
            {#snippet trailing()}<Switch label="SpeakType" checked={status.accessibility} />{/snippet}
          </Row>
        </div>
      </Group>
      <p class="caption">Turn on <b>SpeakType</b>. You may need your Mac password.</p>
    </div>
  {/snippet}
</Page>

<style>
  .ok {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-top: 8px;
  }
  .ok p {
    margin: 0;
    color: var(--st-text);
    font-size: var(--st-font-body);
  }
  .actions {
    display: flex;
    gap: 10px;
  }
  .waiting {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .path {
    word-break: break-all;
  }
  .actions :global(.flush) {
    margin-left: -10px;
  }
  .mock {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 0 0 16px;
    margin: 22px 0 auto;
    border-radius: 12px;
    background: var(--st-popover);
    box-shadow: var(--st-elevation-popover);
    overflow: hidden;
  }
  .mock .caption {
    margin: 0 16px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
  .mock .caption b {
    color: var(--st-text);
    font-weight: 600;
  }
  .mock :global(.st-group) {
    margin: 0 16px;
    background: var(--st-field);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 16px;
    background: var(--st-field);
    color: var(--st-text);
    font-size: var(--st-font-caption);
    font-weight: 500;
  }
  .lights {
    display: flex;
    gap: 6px;
  }
  .lights i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--st-track);
  }
  .back {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .target {
    border-radius: var(--st-radius-card);
    box-shadow: inset 0 0 0 1px var(--st-accent);
  }
</style>
