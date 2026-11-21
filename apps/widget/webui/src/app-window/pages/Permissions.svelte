<script lang="ts">
  import { Check, Minus } from '@lucide/svelte';
  import { tick } from 'svelte';
  import { Button, Group, Row, TextArea, confirmAlert, toast } from '@speaktype/ui';
  import { liveQuery, useEvent } from '../../lib/data';
  import {
    checkPermissions, openOnboarding, openSystemPane, startMicTest, stopMicTest,
  } from '../../lib/ipc';
  import Page from '../Page.svelte';
  import { commands } from '../../lib/bindings';
  const { testTyping } = commands;
  import Section from '../Section.svelte';
  import LevelMeter from './LevelMeter.svelte';

  const TYPING_TEXT = 'SpeakType test';
  const SETTINGS = 'com.apple.preference.security?Privacy_';

  const permissions = liveQuery('permissions', checkPermissions, { invalidateOn: ['sidecar:permissions'], pollMs: 3000 });

  let running = $state<'mic' | 'typing' | null>(null);
  let device = $state('');
  let level = $state(0);
  let typed = $state('');
  let typingResult = $state<{ ok: boolean; message: string } | null>(null);

  useEvent<{ rms: number }>('mictest:level', ({ rms }) => (level = Math.min(1, Math.sqrt(rms * 3))));
  useEvent<unknown>('mictest:stopped', () => {
    if (running === 'mic') running = null;
  });
  $effect(() => () => {
    if (running === 'mic') void stopMicTest();
  });

  async function startTest(kind: 'mic' | 'typing') {
    if (running === 'mic') await stopMicTest();
    running = kind;
    if (kind === 'mic') {
      try {
        device = (await startMicTest()).device ?? 'the default input';
      } catch (error) {
        running = null;
        toast.show({ message: `Couldn't start the microphone test. ${(error as { message?: string })?.message ?? error}` });
      }
      return;
    }
    typed = '';
    typingResult = null;
    await tick();
    try {
      await testTyping();
      typingResult = { ok: true, message: 'Typed successfully' };
    } catch (error) {
      typingResult = { ok: false, message: (error as { message?: string })?.message ?? `${error}` };
    }
  }

  async function stopTest() {
    if (running === 'mic') await stopMicTest();
    running = null;
  }

  async function reset() {
    const confirmed = await confirmAlert({
      title: 'Walk through permissions again?',
      message: 'SpeakType will open setup so you can allow the microphone and accessibility again.',
      confirmLabel: 'Continue',
    });
    if (confirmed) await openOnboarding();
  }

  const allowed = (granted: boolean | undefined) => (granted ? 'Allowed' : 'Not allowed');
</script>

<Page title="Permissions" lede="What SpeakType is allowed to do on this Mac. Test any of them to be sure.">
  <Section>
    <Group>
      <Row
        kind="icon"
        label="Microphone"
        caption={running === 'mic'
          ? `Say something… hearing you on ${device}`
          : `${allowed(permissions.data?.microphone)} · hears you when you dictate or narrate a session`}
        icon={permissions.data?.microphone ? Check : Minus}
        tileColor={permissions.data?.microphone ? 'var(--st-success)' : 'var(--st-warning)'}
      >
        {#snippet trailing()}
          {#if running === 'mic'}
            <LevelMeter {level} />
            <Button size={28} onclick={stopTest}>Stop test</Button>
          {:else if permissions.data?.microphone}
            <Button size={28} onclick={() => startTest('mic')}>Test</Button>
          {:else}
            <Button size={28} onclick={() => openSystemPane(`${SETTINGS}Microphone`)}>Open System Settings…</Button>
          {/if}
        {/snippet}
      </Row>
      <Row
        kind="icon"
        label="Accessibility"
        caption="{allowed(permissions.data?.accessibility)} · lets SpeakType type your words into other apps"
        icon={permissions.data?.accessibility ? Check : Minus}
        tileColor={permissions.data?.accessibility ? 'var(--st-success)' : 'var(--st-warning)'}
      >
        {#snippet trailing()}
          {#if running === 'typing'}
            <span class="typing">
              <TextArea aria-label="Typing test" rows={2} bind:value={typed} placeholder={TYPING_TEXT} autofocus />
              {#if typingResult}
                <span class="result" class:fail={!typingResult.ok}>{typingResult.message}</span>
              {/if}
            </span>
            <Button size={28} onclick={stopTest}>Done</Button>
          {:else if permissions.data?.accessibility}
            <Button size={28} onclick={() => startTest('typing')}>Test typing</Button>
          {:else}
            <Button size={28} onclick={() => openSystemPane(`${SETTINGS}Accessibility`)}>Open System Settings…</Button>
          {/if}
        {/snippet}
      </Row>
      <Row
        kind="icon"
        label="Screen Recording"
        caption="Needed to capture the moment you draw. Never video. Sessions still work with voice and copied text."
        icon={Minus}
        tileColor="var(--st-warning)"
      >
        {#snippet trailing()}
          <Button size={28} onclick={() => openSystemPane(`${SETTINGS}ScreenCapture`)}>Open System Settings…</Button>
        {/snippet}
      </Row>
    </Group>
  </Section>

  <Section>
    <Group>
      <Row kind="control" label="Something not working after an update?" caption="macOS sometimes forgets permissions when an app updates. Reset them and SpeakType walks you through again.">
        {#snippet trailing()}<Button size={28} onclick={reset}>Reset permissions…</Button>{/snippet}
      </Row>
    </Group>
  </Section>
</Page>

<style>
  .typing {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 280px;
  }
  .result {
    color: var(--st-success);
    font-size: var(--st-font-caption);
  }
  .result.fail {
    color: var(--st-danger-text);
  }
</style>
