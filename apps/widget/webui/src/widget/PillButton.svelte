<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Tooltip } from '@speaktype/ui';

  let {
    label,
    shortcut,
    tone = 'neutral',
    onclick,
    icon,
  }: { label: string; shortcut?: string[]; tone?: 'neutral' | 'danger'; onclick: () => void; icon: Snippet } = $props();
</script>

<Tooltip {label} {shortcut}>
  {#snippet children(tooltipProps)}
    <button type="button" {...tooltipProps} class="pill-btn {tone}" aria-label={label} data-action {onclick}>
      {@render icon()}
    </button>
  {/snippet}
</Tooltip>

<style>
  .pill-btn {
    display: grid;
    flex: none;
    place-items: center;
    box-sizing: border-box;
    width: calc(26px * var(--k));
    height: calc(26px * var(--k));
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: var(--st-fill);
    color: rgba(255, 255, 255, 0.7);
    transition: background-color var(--st-dur-fast) var(--st-ease-out);
  }
  .pill-btn:hover {
    background: var(--st-fill-hover);
  }
  .pill-btn:active {
    background: var(--st-fill-pressed);
  }
  .pill-btn:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .danger {
    background: rgb(255 69 58 / 0.2);
    color: var(--st-danger);
  }
  .danger:hover {
    background: rgb(255 69 58 / 0.3);
  }
  .danger:active {
    background: rgb(255 69 58 / 0.38);
  }
</style>
