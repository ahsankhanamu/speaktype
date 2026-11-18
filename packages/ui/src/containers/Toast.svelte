<script lang="ts" module>
  function autoDismiss(node: HTMLElement, params: { ms: number; paused: boolean; dismiss: () => void }) {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const arm = ({ ms, paused, dismiss }: typeof params) => {
      clearTimeout(timer);
      if (!paused) timer = setTimeout(dismiss, ms);
    };
    arm(params);
    return { update: arm, destroy: () => clearTimeout(timer) };
  }
</script>

<script lang="ts">
  import { CircleCheck } from '@lucide/svelte';
  import Button from '../controls/Button.svelte';
  import { pop } from '../motion';
  import { dismiss, MAX_VISIBLE, toasts } from './toasts.svelte';

  let paused = $state(false);
  const visible = $derived(toasts.slice(0, MAX_VISIBLE));
</script>

<div class="stack" role="presentation" onmouseenter={() => (paused = true)} onmouseleave={() => (paused = false)}>
  {#each visible as item (item.id)}
    <div class="toast" role="status" transition:pop use:autoDismiss={{ ms: item.durationMs, paused, dismiss: () => dismiss(item.id) }}>
      <CircleCheck size={14} aria-hidden="true" />
      {item.message}
      {#if item.action}
        {@const action = item.action}
        <Button
          variant="plain"
          size={28}
          onclick={() => {
            action.run();
            dismiss(item.id);
          }}>{action.label}</Button
        >
      {/if}
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    bottom: 20px;
    left: 50%;
    z-index: var(--st-z-toast);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    transform: translateX(-50%);
    pointer-events: none;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 14px;
    border-radius: 14px;
    background: var(--st-popover);
    box-shadow: var(--st-elevation-popover);
    color: var(--st-text);
    font-family: var(--st-font-family);
    font-size: var(--st-font-info);
    white-space: nowrap;
    pointer-events: auto;
  }
  .toast > :global(svg) {
    color: var(--st-success);
  }
</style>
