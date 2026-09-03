<script lang="ts">
  import { Select as SelectPrimitive, type SelectSingleRootProps } from 'bits-ui';

  type RootProps = Omit<SelectSingleRootProps, 'type'> & { type?: 'single' };

  let {
    type = 'single',
    value = $bindable(),
    children,
    ...restProps
  }: RootProps & { type?: 'single' } = $props();
</script>

<SelectPrimitive.Root bind:value={value} {type} {...restProps}>
  {@render children?.()}
</SelectPrimitive.Root>