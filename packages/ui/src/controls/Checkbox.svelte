<script lang="ts">
  import { Check, Minus } from '@lucide/svelte';
  import { Checkbox } from 'bits-ui';
  import type { PassThrough } from '../attrs';

  let {
    checked = $bindable(false),
    disabled = false,
    label,
    ...rest
  }: { checked?: boolean | 'mixed'; disabled?: boolean; label: string } & PassThrough = $props();
</script>

<label class="st-checkbox" class:disabled>
  <Checkbox.Root
    checked={checked === true}
    indeterminate={checked === 'mixed'}
    onCheckedChange={(next) => (checked = next)}
    {disabled}
    class="st-checkbox-box"
    {...rest}
  >
    {#snippet children({ checked: on, indeterminate })}
      {#if indeterminate}<Minus size={12} strokeWidth={3} aria-hidden="true" />{:else if on}<Check size={12} strokeWidth={3} aria-hidden="true" />{/if}
    {/snippet}
  </Checkbox.Root>
  {label}
</label>

<style>
  .st-checkbox {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--st-text);
    font-size: var(--st-font-body);
  }
  .disabled {
    color: var(--st-text-tertiary);
  }
  :global(.st-checkbox-box) {
    display: grid;
    flex: none;
    place-items: center;
    box-sizing: border-box;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: 5px;
    background: transparent;
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--st-text) 34%, transparent);
    color: #fff;
    transition: background-color var(--st-dur-fast) var(--st-ease-out);
  }
  :global(.st-checkbox-box[data-state='checked']),
  :global(.st-checkbox-box[data-state='indeterminate']) {
    background: var(--st-accent);
    box-shadow: none;
  }
  :global(.st-checkbox-box:disabled) {
    opacity: 0.4;
  }
  :global(.st-checkbox-box:focus-visible) {
    outline: none;
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--st-text) 34%, transparent), var(--st-focus-ring);
  }
  :global(.st-checkbox-box[data-state='checked']:focus-visible),
  :global(.st-checkbox-box[data-state='indeterminate']:focus-visible) {
    box-shadow: var(--st-focus-ring);
  }
</style>
