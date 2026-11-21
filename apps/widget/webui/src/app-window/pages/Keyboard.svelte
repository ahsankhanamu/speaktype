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
  /** The row whose shortcut is being recorded. */
  let recording = $state<ShortcutAction | null>(null);

  type Editable = { action: Exclude<ShortcutAction, 'drawKey'>; label: string; caption?: string; presets: { keys: string; label: string }[] };
  const EDITABLE: Editable[] = [
    {
      action: 'dictation',
      label: 'Start / stop dictation',
      caption: 'Press once to start, again to stop',
      presets: [
        { keys: 'Super+Control', label: '⌃⌘ (default)' },
        { keys: 'F9', label: 'F9' },
        { keys: 'F8', label: 'F8' },
      ],
    },
    { action: 'session', label: 'Start / stop a session', presets: [{ keys: 'Option+Super+K', label: '⌥⌘K (default)' }] },
    {
      action: 'screenshot',
      label: 'Take a screenshot',
      caption: 'Joins the current session or starts one',
      presets: [{ keys: 'Super+Option+S', label: '⌥⌘S (default)' }],
    },
  ];

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
      {#each EDITABLE as { action, label, caption, presets } (action)}
        {@const keys = field(action)}
        <Row kind="control" {label} {caption}>
          {#snippet trailing()}
            {@render useAnyway(action)}
            <span class="split">
              <ShortcutField
                bind:value={keys.get, keys.set}
                bind:recording={() => recording === action, (on) => (recording = on ? action : null)}
                conflict={refusals[action]?.message}
              />
              <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                  {#snippet child({ props })}
                    <button {...props} type="button" class="more" aria-label="Change {label}"><ChevronDown size={12} aria-hidden="true" /></button>
                  {/snippet}
                </DropdownMenu.Trigger>
              <DropdownMenu.Content>
                <DropdownMenu.RadioGroup value={shortcuts.keys(action)} onValueChange={(next) => save(action, next)}>
                  {#each presets as preset (preset.keys)}<MenuItem value={preset.keys} label={preset.label} />{/each}
                </DropdownMenu.RadioGroup>
                <MenuSeparator />
                  <MenuItem label="Record a shortcut…" onSelect={() => (recording = action)} />
                </DropdownMenu.Content>
              </DropdownMenu.Root>
            </span>
          {/snippet}
        </Row>
      {/each}
      <Row kind="control" label="Draw (hold)" caption="Let go to click and scroll normally">
        {#snippet trailing()}
          {#if refusals.drawKey}<span class="note">{refusals.drawKey.message}</span>{/if}
          <PopupMenu
            label="Draw (hold)"
            bind:value={() => shortcuts.keys('drawKey'), (next) => save('drawKey', next)}
            items={[
              { value: 'right_option', label: 'Right ⌥' },
              { value: 'right_command', label: 'Right ⌘' },
              { value: 'fn', label: 'Fn' },
            ]}
          />
        {/snippet}
      </Row>
    </Group>
  </Section>

  <Section title="In SpeakType" note="Standard Mac shortcuts. They work while SpeakType is in front or its menu is open, and can't be changed.">
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

  {#snippet footer()}<Button size={28} onclick={restore}>Restore defaults…</Button>{/snippet}
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
  /* The shortcut and its menu read as one control: a split button. */
  .split {
    position: relative;
    display: inline-flex;
    align-items: flex-start;
  }
  /* One edge and one focus ring around both halves, not one per half. */
  .split::after {
    content: '';
    position: absolute;
    inset: 0 0 auto;
    height: var(--st-h-panel-control);
    border-radius: 14px;
    box-shadow: inset 0 0 0 0.5px var(--st-edge);
    pointer-events: none;
  }
  .split:has(:global(.field.recording), :focus-visible, .more[data-state='open'])::after {
    box-shadow: var(--st-focus-ring);
  }
  .split:has(:global(.field[data-conflict]))::after {
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--st-warning) 80%, transparent);
  }
  .split :global(.st-shortcut .field.field) {
    border-radius: 14px 0 0 14px;
    box-shadow: none;
  }
  .more {
    display: grid;
    place-items: center;
    box-sizing: border-box;
    width: 26px;
    height: var(--st-h-panel-control);
    padding: 0 2px 0 0;
    border: 0;
    border-left: 0.5px solid var(--st-separator);
    border-radius: 0 14px 14px 0;
    background: var(--st-inset);
    color: var(--st-text-secondary);
  }
  .more:hover {
    background: var(--st-fill-hover);
  }
  .more:focus-visible {
    outline: none;
  }
</style>
