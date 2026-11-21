<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Keycap from './Keycap.svelte';
  import Spinner from './Spinner.svelte';
  import { formatShortcut } from './shortcut';

  let {
    variant = 'secondary',
    size = 32,
    busy = false,
    disabled = false,
    shortcut,
    children,
    class: className = '',
    onclick,
    ...rest
  }: {
    variant?: 'primary' | 'secondary' | 'plain' | 'destructive';
    size?: 28 | 32;
    busy?: boolean;
    disabled?: boolean;
    shortcut?: string[];
    children: Snippet;
  } & HTMLButtonAttributes = $props();
</script>

<button
  type="button"
  class="st-btn {variant} s{size} {className}"
  class:busy
  disabled={disabled || busy}
  aria-disabled={disabled || busy ? 'true' : undefined}
  aria-busy={busy ? 'true' : undefined}
  onclick={busy || disabled ? undefined : onclick}
  {...rest}
>
  <span class="label">
    {@render children()}
    {#each shortcut ? formatShortcut(shortcut) : [] as key}<Keycap {key} />{/each}
  </span>
  {#if busy}<span class="spinner"><Spinner /></span>{/if}
</button>

<style>
  .st-btn {
    --fill: var(--st-fill);
    --fill-hover: var(--st-fill-hover);
    --fill-pressed: var(--st-fill-pressed);
    --edge: inset 0 0 0 0.5px var(--st-edge);
    --focus: var(--st-focus-ring);
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    border: 0;
    margin: 0;
    background: var(--fill);
    box-shadow: var(--edge);
    color: var(--st-text);
    font: inherit;
    font-size: var(--st-font-info);
    white-space: nowrap;
    transition:
      background-color var(--st-dur-fast) var(--st-ease-out),
      box-shadow var(--st-dur-fast) var(--st-ease-out);
  }
  .s32 {
    height: var(--st-h-panel-control);
    padding: 0 18px;
    border-radius: 16px;
  }
  .s28 {
    height: var(--st-h-row-control);
    padding: 0 14px;
    border-radius: 14px;
  }
  .primary {
    --fill: var(--st-accent);
    --fill-hover: var(--st-accent-hover);
    --fill-pressed: var(--st-accent-pressed);
    --edge: 0 0 #0000;
    --focus: inset 0 0 0 1px rgba(255, 255, 255, 0.7), var(--st-focus-ring);
    color: #fff;
    font-weight: 600;
  }
  .plain {
    --fill: transparent;
    --fill-hover: color-mix(in srgb, var(--st-accent) 12%, transparent);
    --fill-pressed: color-mix(in srgb, var(--st-accent) 20%, transparent);
    --edge: 0 0 #0000;
    padding-inline: 10px;
    color: var(--st-link);
  }
  .destructive {
    color: var(--st-danger-text);
  }
  .st-btn:hover:not(:disabled) {
    background: var(--fill-hover);
  }
  .st-btn:active:not(:disabled) {
    background: var(--fill-pressed);
  }
  .st-btn:focus-visible {
    outline: none;
    box-shadow: var(--edge), var(--focus);
  }
  .st-btn:disabled:not(.busy) {
    opacity: 0.4;
  }
  .label {
    --keycap-size: 20px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .busy .label {
    visibility: hidden;
  }
  .spinner {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
  }
</style>
