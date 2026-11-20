<script lang="ts">
  import { RadioGroup } from 'bits-ui';

  let {
    colors,
    value = $bindable(),
    label,
  }: { colors: { value: string; name: string }[]; value: string; label?: string } = $props();

  const selectedName = $derived(colors.find((color) => color.value === value)?.name);
</script>

<div class="st-swatches">
  <RadioGroup.Root bind:value orientation="horizontal" class="row" aria-label={label}>
    {#each colors as color (color.value)}
      <RadioGroup.Item value={color.value} class="st-swatch" aria-label={color.name} style="background: {color.value}" />
    {/each}
  </RadioGroup.Root>
  <span class="name">{selectedName}</span>
</div>

<style>
  .st-swatches {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }
  :global(.st-swatches .row) {
    display: flex;
    gap: 12px;
  }
  :global(.st-swatch) {
    flex: none;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    box-shadow: inset 0 0 0 0.5px var(--st-edge);
  }
  :global(.st-swatch[data-state='checked']) {
    box-shadow: 0 0 0 2px var(--st-window), 0 0 0 4px var(--st-accent);
  }
  :global(.st-swatch:focus-visible) {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  :global(.st-swatch[data-state='checked']:focus-visible) {
    box-shadow: 0 0 0 2px var(--st-window), 0 0 0 4px var(--st-accent);
  }
  .name {
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
  }
</style>
