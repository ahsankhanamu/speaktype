<script lang="ts">
  import { ChevronDown } from '@lucide/svelte';
  import {
    Button, DropdownMenu, Group, Keycap, MenuItem, MenuSeparator, PopupMenu, Row, ShortcutField, confirmAlert, formatShortcut, toast,
  } from '@speaktype/ui';
  import Page from '../Page.svelte';
  import Section from '../Section.svelte';
  import { shortcutSet, shortcutsReset, type ShortcutAction } from '../shortcuts';
  import { useShortcuts } from '../shortcuts.svelte';

  const FIXED: { label: string; caption: string; keys: string[] }[] = [
    { label: 'Cancel recording', caption: 'Only while recording', keys: ['Escape'] },
    { label: 'Undo last stroke', caption: 'While drawing', keys: ['Super', 'Z'] },
    { label: 'Copy prompt', caption: 'In quick review and Result', keys: ['Super', 'Enter'] },
    { label: 'Open History', caption: 'From the menu bar menu', keys: ['Super', 'Y'] },
    { label: 'Open Settings', caption: 'From the menu bar menu', keys: ['Super', ','] },
    { label: 'Quit', caption: 'From the menu bar menu', keys: ['Super', 'Q'] },
  ];
  const REFUSALS = ['conflict', 'systemShortcut', 'registerFailed', 'invalid'];

  type Refusal = { kind: string; message: string; keys: string };

  const shortcuts = useShortcuts();
  let refusals = $state<Partial<Record<ShortcutAction, Refusal>>>({});
  let recording = $state(false);

  async function save(action: ShortcutAction, keys: string, force = false) {
    try {
      await shortcutSet(action, keys, force);
      delete refusals[action];
    } catch (error) {
      const { kind, message } = error as { kind?: string; message?: string };
      if (kind && REFUSALS.includes(kind)) refusals[action] = { kind, message: message ?? '', keys };
      else toast.show({ message: `Couldn't change that shortcut. ${message ?? error}` });
    }
  }

  const field = (action: ShortcutAction) => ({
    get: () => shortcuts.keys(action).split('+').filter(Boolean),
    set(keys: string[]) {
      if (keys.length) void save(action, keys.join('+'));
    },
  });
  const dictation = field('dictation');
  const session = field('session');
  const screenshot = field('screenshot');

  async function restore() {
    const confirmed = await confirmAlert({
      title: 'Restore default shortcuts?',
      message: 'Dictation goes back to ⌃⌘, sessions to ⌥⌘K, screenshots to ⌥⌘S and drawing to Right ⌥.',
      confirmLabel: 'Restore',
    });
    if (!confirmed) return;
    try {
      await shortcutsReset();
      refusals = {};
    } catch (error) {
      toast.show({ message: `Couldn't restore the shortcuts. ${(error as { message?: string })?.message ?? error}` });
    }
  }
</script>

{#snippet useAnyway(action: ShortcutAction)}
  {@const refusal = refusals[action]}
  {#if refusal?.kind === 'systemShortcut'}
    <Button size={28} onclick={() => save(action, refusal.keys, true)}>Use anyway</Button>
  {/if}
{/snippet}

<Page title="Keyboard">
  <Section title="Shortcuts">
    <Group>
      <Row kind="control" label="Start / stop dictation" caption="Press once to start, again to stop">
        {#snippet trailing()}
          {@render useAnyway('dictation')}
          <ShortcutField bind:value={dictation.get, dictation.set} bind:recording conflict={refusals.dictation?.message} />
          <DropdownMenu.Root>
            <DropdownMenu.Trigger>
              {#snippet child({ props })}
                <Button {...props} size={28} aria-label="Shortcut presets"><ChevronDown size={13} aria-hidden="true" /></Button>
              {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content>
              <DropdownMenu.RadioGroup value={shortcuts.keys('dictation')} onValueChange={(next) => save('dictation', next)}>
                <MenuItem value="Super+Control" label="⌃⌘ (default)" />
                <MenuItem value="F9" label="F9" />
                <MenuItem value="F8" label="F8" />
              </DropdownMenu.RadioGroup>
              <MenuSeparator />
              <MenuItem label="Record a shortcut…" onSelect={() => (recording = true)} />
            </DropdownMenu.Content>
          </DropdownMenu.Root>
        {/snippet}
      </Row>
      <Row kind="control" label="Start / stop a session">
        {#snippet trailing()}
          {@render useAnyway('session')}
          <ShortcutField bind:value={session.get, session.set} conflict={refusals.session?.message} />
        {/snippet}
      </Row>
      <Row kind="control" label="Take a screenshot" caption="Joins the current session or starts one">
        {#snippet trailing()}
          {@render useAnyway('screenshot')}
          <ShortcutField bind:value={screenshot.get, screenshot.set} conflict={refusals.screenshot?.message} />
        {/snippet}
      </Row>
      <Row kind="control" label="Draw (hold)" caption="Let go to click and scroll normally">
        {#snippet trailing()}
          {#if refusals.drawKey}<span class="note">{refusals.drawKey.message}</span>{/if}
          <PopupMenu
            label="Draw (hold)"
            bind:value={() => shortcuts.keys('drawKey'), (next) => save('drawKey', next)}
            items={[
              { value: 'RightOption', label: 'Right ⌥' },
              { value: 'RightCommand', label: 'Right ⌘' },
              { value: 'Fn', label: 'Fn' },
            ]}
          />
        {/snippet}
      </Row>
    </Group>
  </Section>

  <Section title="In SpeakType">
    <Group>
      {#each FIXED as { label, caption, keys } (label)}
        <Row kind="control" label={label} caption={caption}>
          {#snippet trailing()}
            <span class="keys">{#each formatShortcut(keys) as key}<Keycap {key} />{/each}</span>
          {/snippet}
        </Row>
      {/each}
    </Group>
  </Section>

  <div class="restore"><Button size={28} onclick={restore}>Restore defaults</Button></div>
</Page>

<style>
  .keys {
    display: inline-flex;
    gap: 3px;
  }
  .note {
    color: var(--st-warning);
    font-size: var(--st-font-caption);
  }
  .restore {
    display: flex;
  }
</style>
