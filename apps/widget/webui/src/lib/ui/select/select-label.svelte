<script lang="ts">
  import { Select as SelectPrimitive, type SelectGroupHeadingProps } from 'bits-ui';

  type Props = SelectGroupHeadingProps;

  let { children, ...restProps }: Props = $props();
</script>

<SelectPrimitive.GroupHeading data-slot="select-label" class="st-select-label" {...restProps}>
  {@render children?.()}
</SelectPrimitive.GroupHeading>

<style>
  :global(.st-select-label) {
    padding: 5px 10px 3px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-label, #999);
  }
</style>