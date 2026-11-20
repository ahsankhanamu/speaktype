<script lang="ts" generics="T extends string">
  import { RadioGroup } from 'bits-ui';

  let { items, value = $bindable(), label }: { items: { value: T; label: string }[]; value: T; label?: string } = $props();
</script>

<RadioGroup.Root bind:value orientation="horizontal" class="st-segmented" aria-label={label}>
  {#each items as item (item.value)}
    <RadioGroup.Item value={item.value} class="st-segmented-item">{item.label}</RadioGroup.Item>
  {/each}
</RadioGroup.Root>

<style>
  /* Equal segments, as in AppKit; given the full width, they share it evenly. */
  :global(.st-segmented) {
    display: inline-grid;
    grid-auto-columns: 1fr;
    grid-auto-flow: column;
    padding: 3px;
    border-radius: 17px;
    background: var(--st-fill);
    box-shadow: inset 0 0 0 0.5px var(--st-edge);
  }
  :global(.st-segmented-item) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    height: 26px;
    padding: 0 14px;
    border: 0;
    border-radius: 13px;
    background: transparent;
    color: var(--st-text-secondary);
    font: inherit;
    font-size: var(--st-font-info);
    transition:
      background-color var(--st-dur-fast) var(--st-ease-out),
      color var(--st-dur-fast) var(--st-ease-out);
  }
  :global(.st-segmented-item:hover:not([data-state='checked'])) {
    color: var(--st-text);
  }
  :global(.st-segmented-item[data-state='checked']) {
    background: var(--st-fill-pressed);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    color: var(--st-text);
  }
  :global(.st-segmented-item:focus-visible) {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
</style>
