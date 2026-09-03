<script lang="ts">
  import { Select as SelectPrimitive, type SelectContentProps } from 'bits-ui';

  type Props = SelectContentProps;

  let { children, ...restProps }: Props = $props();
</script>

<SelectPrimitive.Portal>
  <SelectPrimitive.Content
    data-slot="select-content"
    class="st-select-content"
    sideOffset={4}
    {...restProps}
  >
    <div class="select-scroll">
      {@render children?.()}
    </div>
  </SelectPrimitive.Content>
</SelectPrimitive.Portal>

<style>
  :global(.st-select-content) {
    z-index: 50;
    max-height: min(320px, var(--bits-select-content-available-height, 320px));
    width: var(--bits-select-anchor-width);
    min-width: var(--bits-select-anchor-width, 12rem);
    overflow-y: auto;
    overscroll-behavior: contain;
    background: var(--bg-elevated, #18181b);
    border: 1px solid var(--border, #333);
    border-radius: 10px;
    padding: 4px;
    box-shadow: var(--elevation-2, 0 4px 14px rgba(0, 0, 0, 0.42));
    color: var(--text, #e0e0e0);
    font-size: 13px;
    font-family: inherit;
    box-sizing: border-box;
  }
  .select-scroll {
    width: 100%;
  }
</style>