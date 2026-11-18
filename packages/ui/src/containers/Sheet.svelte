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
    position: fixed;
    top: 72px;
    inset-inline: 0;
    z-index: var(--st-z-sheet);
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    width: min(780px, 100% - 48px);
    max-height: calc(100vh - 96px);
    margin-inline: auto;
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
    padding: var(--inset) var(--inset) 16px;
  }
  header :global([data-dialog-title]) {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .body {
    flex: 1;
    min-height: 0;
    padding: 0 var(--inset) 20px;
    overflow-y: auto;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 18px var(--inset) 24px;
    box-shadow: inset 0 0.5px 0 var(--st-separator);
  }
</style>
