<script lang="ts">
  import { AudioLines, ChevronsUpDown } from '@lucide/svelte';
  import { Button, Combobox, Group, Keycap, Popover, PopupMenu, Progress, Row, Switch, TextField, formatShortcut } from '@speaktype/ui';
  import MicPicker from '../../lib/app/MicPicker.svelte';
  import { plainModel } from '../../lib/app/models';
  import { useModels } from '../../lib/app/models.svelte';
  import SpeechModelsSheet from '../../lib/app/SpeechModelsSheet.svelte';
  import { LANGUAGES } from '../languages';
  import Page from '../Page.svelte';
  import Section from '../Section.svelte';
  import { navigate } from '../router.svelte';
  import { useSettings } from '../settings.svelte';
  import { useShortcuts } from '../shortcuts.svelte';

  const PASTE_KEYS = ['', 'enter', 'tab', 'enter+enter', 'space'];

  const settings = useSettings();
  const models = useModels();
  const shortcuts = useShortcuts();
  const language = settings.field('language', 'auto');
  const pasteMode = settings.field('paste_mode', 'active');
  const saveAudio = settings.field('save_recordings', false);
  const device = settings.field('input_device', '');
  const postPaste = settings.field('post_paste_keys', '');

  let sheetOpen = $state(false);
  let micOpen = $state(false);
  let customKeys = $state(false);

  const pasteChoice = $derived(customKeys || !PASTE_KEYS.includes(postPaste.value) ? 'custom' : postPaste.value);
  const hotkey = $derived(shortcuts.keys('dictation').split('+').filter(Boolean));

  const active = $derived(plainModel(models.activeId));
  const activeRow = $derived(models.rows.find((row) => row.model.id === models.activeId));
  const downloading = $derived(models.rows.find((row) => row.state.action === 'pause'));

  function choosePasteKeys(value: string) {
    customKeys = value === 'custom';
    if (!customKeys) postPaste.value = value || null;
  }
</script>

<Page title="Dictation">
  <Section>
    <Group>
      <Row kind="control" label="Start / stop dictation" caption="Press once to start, again to stop">
        {#snippet trailing()}
          <span class="keys">{#each formatShortcut(hotkey) as key}<Keycap {key} />{/each}</span>
          <Button size={28} aria-label="Change in Keyboard" onclick={() => navigate('keyboard')}>Change in Keyboard…</Button>
        {/snippet}
      </Row>
      <Row kind="control" label="Language">
        {#snippet trailing()}<Combobox label="Language" items={LANGUAGES} bind:value={language.value} />{/snippet}
      </Row>
      <Row kind="control" label="Microphone">
        {#snippet trailing()}
          <Popover bind:open={micOpen}>
            {#snippet trigger(props)}
              <Button {...props} size={28}>{device.value || 'System default'}<ChevronsUpDown size={13} aria-hidden="true" /></Button>
            {/snippet}
            <div class="mics"><MicPicker /></div>
          </Popover>
        {/snippet}
      </Row>
      <Row kind="control" label="Paste into" caption="Where your words go if you switch apps while it's transcribing">
        {#snippet trailing()}
          <PopupMenu
            label="Paste into"
            bind:value={pasteMode.value}
            items={[
              { value: 'active', label: "The app you're in now" },
              { value: 'original', label: 'The app you started in' },
            ]}
          />
        {/snippet}
      </Row>
      <Row kind="control" label="After pasting, press" caption="Handy for sending chat messages straight away">
        {#snippet trailing()}
          <PopupMenu
            label="After pasting, press"
            bind:value={() => pasteChoice, choosePasteKeys}
            items={[
              { value: '', label: 'Nothing' },
              { value: 'enter', label: 'Return' },
              { value: 'tab', label: 'Tab' },
              { value: 'enter+enter', label: 'Return twice' },
              { value: 'space', label: 'Space' },
              { value: 'custom', label: 'Custom…', separatorBefore: true },
            ]}
          />
        {/snippet}
      </Row>
      {#if pasteChoice === 'custom'}
        <Row kind="control" label="Custom keys" caption="Join keys with +, for example tab+enter">
          {#snippet trailing()}
            <TextField
              aria-label="Custom keys"
              placeholder="tab+enter"
              value={postPaste.value ?? ''}
              onchange={(event) => (postPaste.value = event.currentTarget.value.trim() || null)}
            />
          {/snippet}
        </Row>
      {/if}
      <Row kind="control" label="Save recording audio" caption="Keep each dictation's audio so you can replay it">
        {#snippet trailing()}<Switch label="Save recording audio" bind:checked={saveAudio.value} />{/snippet}
      </Row>
    </Group>
  </Section>

  <Section title="Speech model">
    <Group>
      <Row
        kind="icon"
        label={active.name}
        caption={[activeRow?.state.label, active.detail, active.size].filter(Boolean).join(' · ')}
        icon={AudioLines}
        tileColor="linear-gradient(180deg, #2a9bff, #0a6fe0)"
      >
        {#snippet trailing()}<Button size={28} onclick={() => (sheetOpen = true)}>Manage models…</Button>{/snippet}
      </Row>
      {#if downloading}
        <Row kind="info" label="{downloading.name} is downloading">
          {#snippet trailing()}
            <span class="bar"><Progress value={(downloading.state.percent ?? 0) / 100} label="{downloading.name} download" /></span>
            <span class="detail">{downloading.state.detail} · switches over when ready</span>
          {/snippet}
        </Row>
      {/if}
    </Group>
  </Section>
</Page>

<SpeechModelsSheet bind:open={sheetOpen} />

<style>
  .keys {
    display: inline-flex;
    gap: 3px;
  }
  .mics {
    width: 340px;
  }
  .bar {
    width: 120px;
  }
  .detail {
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
</style>
