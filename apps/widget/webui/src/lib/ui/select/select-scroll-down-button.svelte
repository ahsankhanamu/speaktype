<script lang="ts">
  import { Select as SelectPrimitive, type SelectScrollDownButtonProps } from 'bits-ui';
  import { ChevronDown } from '@lucide/svelte';

  type Props = SelectScrollDownButtonProps;

  let { ...restProps }: Props = $props();
</script>

<SelectPrimitive.ScrollDownButton data-slot="select-scroll-down-button" class="st-select-scroll-btn" {...restProps}>
  <ChevronDown size={12} />
</SelectPrimitive.ScrollDownButton>

<style>
  :global(.st-select-scroll-btn) {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: center;
    padding: 2px;
    color: var(--text-dim, #888);
  }
</style>