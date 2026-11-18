<script lang="ts">
  import { AlertDialog } from 'bits-ui';
  import { fade } from 'svelte/transition';
  import Button from '../controls/Button.svelte';
  import { duration, sheet } from '../motion';
  import { alertState, settleAlert } from './confirm.svelte';

  const options = $derived(alertState.options);
  let actions = $state<HTMLElement | null>(null);
</script>

<AlertDialog.Root open={alertState.open} onOpenChange={(next) => !next && settleAlert(false)}>
  <AlertDialog.Portal>
    <AlertDialog.Overlay forceMount>
      {#snippet child({ props, open })}
        {#if open}<div {...props} class="scrim" transition:fade={{ duration: duration.base }}></div>{/if}
      {/snippet}
    </AlertDialog.Overlay>
    <AlertDialog.Content forceMount onOpenAutoFocus={(e) => (e.preventDefault(), actions?.querySelector<HTMLElement>('[data-alert-dialog-cancel]')?.focus())}>
      {#snippet child({ props, open })}
        {#if open}
          <div {...props} class="alert" transition:sheet>
            <AlertDialog.Title>{options.title}</AlertDialog.Title>
            {#if options.message}<AlertDialog.Description>{options.message}</AlertDialog.Description>{/if}
            <div class="actions" bind:this={actions}>
              <AlertDialog.Action>
                {#snippet child({ props: action })}
                  <Button {...action} variant={options.destructive ? 'destructive' : 'secondary'} size={28} onclick={() => settleAlert(true)}>{options.confirmLabel}</Button>
                {/snippet}
              </AlertDialog.Action>
              <AlertDialog.Cancel>
                {#snippet child({ props: dismiss })}
                  <Button {...dismiss} variant="primary" size={28}>Cancel</Button>
                {/snippet}
              </AlertDialog.Cancel>
            </div>
          </div>
        {/if}
      {/snippet}
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: var(--st-z-sheet);
    background: var(--st-scrim);
  }
  .alert {
    position: fixed;
    inset: 0;
    z-index: var(--st-z-sheet);
    box-sizing: border-box;
    width: 300px;
    height: fit-content;
    margin: auto;
    padding: 20px;
    border-radius: var(--st-radius-card);
    background: var(--st-popover);
    box-shadow: var(--st-elevation-popover);
    color: var(--st-text);
    font-family: var(--st-font-family);
    text-align: center;
    outline: none;
  }
  .alert :global([data-alert-dialog-title]) {
    margin: 0;
    font-size: var(--st-font-body);
    font-weight: 600;
  }
  .alert :global([data-alert-dialog-description]) {
    margin: 6px 0 16px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
    line-height: 1.45;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .actions :global(.st-btn) {
    flex: 1;
  }
</style>
