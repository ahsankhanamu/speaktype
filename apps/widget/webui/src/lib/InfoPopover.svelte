<script lang="ts">
  import type { Snippet } from 'svelte';
  // Reusable info-icon button that toggles an associated popover panel.
  // Renders the label (opt-in) + a circular info button with aria-expanded/aria-controls,
  // plus a popover (<role="status">) shown when open.
  let {
    label = '',
    title = 'More info',
    align = 'left',
    children,
  }: {
    label?: string;
    title?: string;
    align?: 'left' | 'right';
    children?: Snippet;
  } = $props();

  let open = $state(false);
  let id = $state('');
  let popoverId = $state('');

  $effect(() => {
    if (!id) id = `info-${Math.floor(Math.random() * 1e9).toString(36)}`;
    popoverId = `${id}-popover`;
  });
</script>

<div class="info-control" class:info-control-right={align === 'right'}>
  {#if label}<span class="info-label">{label}</span>{/if}
  <button
    type="button"
    class="info-icon"
    id={id}
    aria-label={title}
    aria-expanded={open ? 'true' : 'false'}
    aria-controls={popoverId}
    onclick={() => (open = !open)}
  >
    <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
      <circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>
    </svg>
  </button>
</div>
{#if open}
  <div class="info-popover" id={popoverId} role="status">
    {@render children?.()}
  </div>
{/if}