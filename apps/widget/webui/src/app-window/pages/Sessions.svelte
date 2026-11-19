<script lang="ts">
  import { Button, Group, Keycap, PopupMenu, Row, ShortcutField, Switch, formatShortcut } from '@speaktype/ui';
  import { openSystemPane } from '../../lib/ipc';
  import Page from '../Page.svelte';
  import Section from '../Section.svelte';
  import { useSettings } from '../settings.svelte';

  const settings = useSettings();
  const drawKey = settings.field('draw_key', 'right_option');
  const drawAnytime = settings.field('draw_anytime', true);
  const screenshot = settings.field('scribe_screenshot_hotkey', 'Super+Option+S');
  const clipboard = settings.field('scribe_clipboard_monitor', true);

  const screenshotKeys = {
    get: () => screenshot.value.split('+'),
    set: (keys: string[]) => keys.length && (screenshot.value = keys.join('+')),
  };
</script>

<Page title="Sessions">
  <Section>
    <Group>
      <Row kind="icon" label="Screen Recording" caption="Needed to capture screenshots. Voice and copied text work without it.">
        {#snippet trailing()}
          <Button size={28} onclick={() => openSystemPane('com.apple.preference.security?Privacy_ScreenCapture')}>Open System Settings…</Button>
        {/snippet}
      </Row>
    </Group>
  </Section>

  <Section title="Shortcuts">
    <Group>
      <Row kind="control" label="Start or stop a session">
        {#snippet trailing()}
          <span class="keys">{#each formatShortcut(['Option', 'Super', 'K']) as key}<Keycap {key} />{/each}</span>
        {/snippet}
      </Row>
      <Row kind="control" label="Draw while holding" caption="Let go to click and scroll normally">
        {#snippet trailing()}
          <PopupMenu
            label="Draw while holding"
            bind:value={drawKey.value}
            items={[
              { value: 'right_option', label: 'Right Option' },
              { value: 'right_command', label: 'Right Command' },
              { value: 'fn', label: 'Globe / Fn' },
            ]}
          />
        {/snippet}
      </Row>
      <Row kind="control" label="Draw anytime" caption="Hold the draw key outside a session to start one">
        {#snippet trailing()}<Switch label="Draw anytime" bind:checked={drawAnytime.value} />{/snippet}
      </Row>
      <Row kind="control" label="Screenshot without drawing">
        {#snippet trailing()}<ShortcutField bind:value={screenshotKeys.get, screenshotKeys.set} />{/snippet}
      </Row>
    </Group>
  </Section>

  <Section title="Capture">
    <Group>
      <Row kind="control" label="Include text I copy">
        {#snippet trailing()}<Switch label="Include text I copy" bind:checked={clipboard.value} />{/snippet}
      </Row>
    </Group>
  </Section>
</Page>

<style>
  .keys {
    display: inline-flex;
    gap: 3px;
  }
</style>
