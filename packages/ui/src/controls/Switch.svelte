<script lang="ts">
  import { Switch } from 'bits-ui';
  import type { PassThrough } from '../attrs';

  let {
    checked = $bindable(false),
    disabled = false,
    label,
    ...rest
  }: { checked?: boolean; disabled?: boolean; label: string } & PassThrough = $props();
</script>

<Switch.Root bind:checked {disabled} aria-label={label} class="st-switch" {...rest}>
  <Switch.Thumb class="st-switch-thumb" />
</Switch.Root>

<style>
  :global(.st-switch) {
    position: relative;
    flex: none;
    box-sizing: border-box;
    width: 38px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: 11px;
    background: var(--st-track);
    transition: background-color var(--st-dur-base) var(--st-ease-out);
  }
  :global(.st-switch[data-state='checked']) {
    background: var(--st-accent);
  }
  :global(.st-switch:focus-visible) {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  :global(.st-switch:disabled) {
    opacity: 0.4;
  }
  :global(.st-switch-thumb) {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.35);
    transition: transform var(--st-dur-base) var(--st-ease-out);
  }
  :global(.st-switch-thumb[data-state='checked']) {
    transform: translateX(16px);
  }
</style>
