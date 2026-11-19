<script lang="ts">
  import { X } from '@lucide/svelte';
  import Keycap from './Keycap.svelte';
  import { formatShortcut } from './shortcut';

  const MODIFIER_NAMES: Record<string, string> = { Control: 'Control', Alt: 'Option', Shift: 'Shift', Meta: 'Super' };
  const MODIFIER_ORDER = ['Control', 'Option', 'Shift', 'Super'];

  let {
    value = $bindable([]),
    recording = $bindable(false),
    conflict,
  }: { value?: string[]; recording?: boolean; conflict?: string } = $props();

  let root = $state<HTMLElement | null>(null);

  function keyName(e: KeyboardEvent) {
    if (e.code.startsWith('Key')) return e.code.slice(3);
    if (e.code.startsWith('Digit')) return e.code.slice(5);
    if (e.code === 'Space') return 'Space';
    return e.key.length === 1 ? e.key.toUpperCase() : e.key;
  }

  /** A shortcut is either modifiers plus a key, or two or more modifiers pressed and released together. */
  $effect(() => {
    if (!recording) return;
    const held = new Set<string>();
    const peak = new Set<string>();

    const capture = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const modifier = MODIFIER_NAMES[e.key];
      if (modifier) {
        held.add(modifier);
        peak.add(modifier);
        return;
      }
      recording = false;
      if (e.key === 'Escape') return;
      const modifiers = [e.ctrlKey && 'Control', e.altKey && 'Option', e.shiftKey && 'Shift', e.metaKey && 'Super'];
      value = [...modifiers.filter((m): m is string => !!m), keyName(e)];
    };

    const release = (e: KeyboardEvent) => {
      const modifier = MODIFIER_NAMES[e.key];
      if (!modifier) return;
      held.delete(modifier);
      if (held.size) return;
      if (peak.size >= 2) {
        value = MODIFIER_ORDER.filter((name) => peak.has(name));
        recording = false;
      }
      peak.clear();
    };

    const clickAway = (e: PointerEvent) => {
      if (!root?.contains(e.target as Node)) recording = false;
    };

    window.addEventListener('keydown', capture, true);
    window.addEventListener('keyup', release, true);
    window.addEventListener('pointerdown', clickAway, true);
    return () => {
      window.removeEventListener('keydown', capture, true);
      window.removeEventListener('keyup', release, true);
      window.removeEventListener('pointerdown', clickAway, true);
    };
  });
</script>

<div class="st-shortcut" bind:this={root}>
  <div class="field" class:recording data-conflict={conflict ? '' : undefined}>
    <button
      type="button"
      class="record"
      aria-label={recording ? 'Press a shortcut' : 'Record shortcut'}
      onclick={() => (recording = true)}
    >
      {#if recording}
        Press a shortcut…
      {:else}
        {#each formatShortcut(value) as key}<Keycap {key} />{/each}
      {/if}
    </button>
    {#if value.length && !recording}
      <button type="button" class="clear" aria-label="Clear shortcut" onclick={() => (value = [])}>
        <X size={12} aria-hidden="true" />
      </button>
    {/if}
  </div>
  {#if conflict}<p class="conflict">{conflict}</p>{/if}
</div>

<style>
  .st-shortcut {
    display: inline-flex;
    flex-direction: column;
    gap: 4px;
  }
  .field {
    display: inline-flex;
    align-items: center;
    box-sizing: border-box;
    height: var(--st-h-panel-control);
    padding: 0 6px;
    border-radius: 16px;
    background: var(--st-inset);
    box-shadow: inset 0 0 0 0.5px var(--st-edge);
  }
  .field:has(:focus-visible) {
    box-shadow: var(--st-focus-ring);
  }
  .field.recording {
    box-shadow: var(--st-focus-ring);
    padding: 0 12px;
  }
  .field[data-conflict] {
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--st-warning) 80%, transparent);
  }
  .record {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    min-width: 32px;
    height: 100%;
    padding: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--st-text-secondary);
    font: inherit;
    font-size: var(--st-font-info);
    cursor: default;
  }
  .clear {
    display: none;
    place-items: center;
    width: 18px;
    height: 18px;
    margin-left: 4px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--st-fill);
    color: var(--st-text-secondary);
    cursor: default;
  }
  .field:hover .clear,
  .field:focus-within .clear {
    display: grid;
  }
  .clear:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .conflict {
    margin: 0;
    color: var(--st-warning);
    font-size: var(--st-font-caption);
  }
</style>
