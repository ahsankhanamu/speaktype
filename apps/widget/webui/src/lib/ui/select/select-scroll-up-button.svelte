<script lang="ts">
  import { Select as SelectPrimitive, type SelectScrollUpButtonProps } from 'bits-ui';
  import { ChevronUp } from '@lucide/svelte';

  type Props = SelectScrollUpButtonProps;

  let { ...restProps }: Props = $props();
</script>

<SelectPrimitive.ScrollUpButton data-slot="select-scroll-up-button" class="st-select-scroll-btn" {...restProps}>
  <ChevronUp size={12} />
</SelectPrimitive.ScrollUpButton>

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