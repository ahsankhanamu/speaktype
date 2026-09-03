<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Select as SelectPrimitive, type SelectItemProps } from 'bits-ui';
  import { Check } from '@lucide/svelte';

  type Props = SelectItemProps & {
    children?: Snippet | undefined;
  };

  let {
    class: className,
    children: childContent,
    value,
    label,
    disabled,
    ...restProps
  }: Props = $props();
</script>

<SelectPrimitive.Item
  data-slot="select-item"
  class="st-select-item"
  {value}
  {label}
  {disabled}
  {...restProps}
>
  {#snippet children({ selected })}
    <span
      class="select-item-check"
      class:visible={selected}
      aria-hidden="true"
    >
      {#if selected}
        <Check size={13} />
      {/if}
    </span>
    <span class="select-item-label">
      {#if childContent}
        {@render childContent()}
      {:else}
        {label}
      {/if}
    </span>
  {/snippet}
</SelectPrimitive.Item>

<style>
  :global(.st-select-item) {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    border-radius: 6px;
    color: var(--text, #e0e0e0);
    font-size: 13px;
    line-height: normal;
    cursor: pointer;
    box-sizing: border-box;
    user-select: none;
  }
  :global(.st-select-item[data-highlighted]) {
    background: var(--accent-soft, rgba(59, 130, 246, 0.15));
  }
  :global(.st-select-item[data-disabled]) {
    opacity: 0.5;
    cursor: default;
    pointer-events: none;
  }
  .select-item-check {
    width: 14px;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    visibility: hidden;
    color: var(--accent, #3b82f6);
  }
  .select-item-check.visible {
    visibility: visible;
  }
  .select-item-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>