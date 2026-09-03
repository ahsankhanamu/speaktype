<script lang="ts">
  import { Select as SelectPrimitive, type SelectTriggerProps } from 'bits-ui';

  type Props = SelectTriggerProps & {
    child?: never;
  };

  let { class: className, children, ...restProps }: Props = $props();
</script>

<SelectPrimitive.Trigger
  data-slot="select-trigger"
  class="st-select"
  {...restProps}
>
  {@render children?.()}
  <div class="select-chevron" aria-hidden="true">
    <svg width="10" height="6" viewBox="0 0 10 6" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
      <path d="M1 1l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </div>
</SelectPrimitive.Trigger>

<style>
  :global(.st-select) {
    width: 100%;
    padding: 0 32px 0 12px;
    min-height: 36px;
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    background: var(--surface-2, #252525);
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    color: var(--text-strong, #fff);
    font-size: 13px;
    font-family: inherit;
    line-height: normal;
    cursor: pointer;
    text-align: left;
    gap: 8px;
    box-sizing: border-box;
    position: relative;
    transition: border-color 0.15s, box-shadow 0.15s, background-color 0.15s;
  }
  :global(.st-select:hover:not(:disabled)) {
    background: var(--surface-3, #2a2a2a);
  }
  :global(.st-select:focus-visible) {
    outline: none;
    border-color: var(--accent, #3b82f6);
    box-shadow: 0 0 0 3px var(--accent-ring, rgba(59, 130, 246, 0.35));
  }
  :global(.st-select:disabled) {
    cursor: default;
    opacity: 0.55;
  }
  :global(.st-select[data-placeholder]) {
    color: var(--text-dim, #888);
  }
  .select-chevron {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-dim, #888);
    display: inline-flex;
    pointer-events: none;
  }
</style>