<script lang="ts">
  import { Select as SelectPrimitive, type SelectGroupProps } from 'bits-ui';

  type Props = SelectGroupProps;

  let { children, ...restProps }: Props = $props();
</script>

<SelectPrimitive.Group data-slot="select-group" {...restProps}>
  {@render children?.()}
</SelectPrimitive.Group>