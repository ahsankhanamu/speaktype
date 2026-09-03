<script lang="ts">
  import { Select as SelectPrimitive, type SelectPortalProps } from 'bits-ui';

  type Props = SelectPortalProps;

  let { children, ...restProps }: Props = $props();
</script>

<SelectPrimitive.Portal {...restProps}>
  {@render children?.()}
</SelectPrimitive.Portal>