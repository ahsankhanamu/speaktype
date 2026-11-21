<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Dialog } from 'bits-ui';
  import { fade } from 'svelte/transition';
  import { duration, sheet as sheetTransition } from '../motion';

  let {
    open = $bindable(false),
    title,
    onclose,
    footer,
    children,
  }: { open?: boolean; title: string; onclose?: () => void; footer?: Snippet; children: Snippet } = $props();
</script>

<Dialog.Root bind:open onOpenChange={(next) => !next && onclose?.()}>
  <Dialog.Portal>
    <Dialog.Overlay forceMount>
      {#snippet child({ props, open: shown })}
        {#if shown}<div {...props} class="scrim" transition:fade={{ duration: duration.base }}></div>{/if}
      {/snippet}
    </Dialog.Overlay>
    <Dialog.Content forceMount>
      {#snippet child({ props, open: shown })}
        {#if shown}
          <div {...props} class="sheet" transition:sheetTransition>
            <header><Dialog.Title>{title}</Dialog.Title></header>
            <div class="body">{@render children()}</div>
            {#if footer}<footer>{@render footer()}</footer>{/if}
          </div>
        {/if}
      {/snippet}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: var(--st-z-sheet);
    background: var(--st-scrim);
  }
  .sheet {
    --inset: 28px;
    /* Centred in the window, both ways. */
    position: fixed;
    inset: 0;
    z-index: var(--st-z-sheet);
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    width: min(780px, 100% - 48px);
    height: fit-content;
    max-height: calc(100vh - 48px);
    margin: auto;
    overflow: hidden;
    border-radius: var(--st-radius-sheet);
    background: var(--st-group);
    box-shadow: var(--st-elevation-sheet);
    color: var(--st-text);
    font-family: var(--st-font-family);
    font-size: var(--st-font-body);
    outline: none;
  }
  header {
    padding: 18px var(--inset) 8px;
  }
  header :global([data-dialog-title]) {
    margin: 0;
    font-size: 17px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .body {
    flex: 1 1 auto;
    min-height: 0;
    padding: 0 var(--inset) 16px;
    overflow-y: auto;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px var(--inset) 14px;
    box-shadow: inset 0 0.5px 0 var(--st-separator);
  }
</style>
