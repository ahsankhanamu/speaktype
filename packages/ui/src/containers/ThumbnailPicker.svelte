<script lang="ts" generics="T extends string">
  import type { Snippet } from 'svelte';
  import { RadioGroup } from 'bits-ui';

  let {
    items,
    value = $bindable(),
    label,
  }: { items: { value: T; label: string; preview: Snippet }[]; value: T; label?: string } = $props();
</script>

<RadioGroup.Root bind:value orientation="horizontal" class="st-thumbs" aria-label={label}>
  {#each items as item (item.value)}
    <RadioGroup.Item value={item.value} class="st-thumb">
      {#snippet children({ checked })}
        <span class="preview" class:checked>{@render item.preview()}</span>
        <span class="caption" class:checked>{item.label}</span>
      {/snippet}
    </RadioGroup.Item>
  {/each}
</RadioGroup.Root>

<style>
  :global(.st-thumbs) {
    display: flex;
    gap: 16px;
  }
  :global(.st-thumb) {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 0;
    border: 0;
    border-radius: 12px;
    background: none;
    color: inherit;
    font: inherit;
    cursor: default;
  }
  :global(.st-thumb:focus-visible) {
    outline: none;
  }
  :global(.st-thumb:focus-visible) .preview {
    box-shadow: var(--st-focus-ring);
  }
  .preview {
    position: relative;
    display: block;
    width: 110px;
    height: 72px;
    overflow: hidden;
    border-radius: 10px;
    box-shadow: 0 0 0 0.5px var(--st-edge);
  }
  .preview.checked {
    box-shadow: 0 0 0 3px var(--st-accent);
  }
  .caption {
    color: var(--st-text-secondary);
    font-size: var(--st-font-body);
  }
  .caption.checked {
    color: var(--st-text);
    font-weight: 600;
  }
</style>
