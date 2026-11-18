<script lang="ts" generics="T extends string">
  import { RadioGroup } from 'bits-ui';

  let { items, value = $bindable(), label }: { items: { value: T; label: string }[]; value: T; label?: string } = $props();
</script>

<RadioGroup.Root bind:value class="st-radio-list" aria-label={label}>
  {#each items as item (item.value)}
    <label class="st-radio">
      <RadioGroup.Item value={item.value} class="st-radio-dot" />
      {item.label}
    </label>
  {/each}
</RadioGroup.Root>

<style>
  :global(.st-radio-list) {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .st-radio {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--st-text);
    font-size: var(--st-font-body);
  }
  :global(.st-radio-dot) {
    flex: none;
    box-sizing: border-box;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--st-text) 34%, transparent);
    transition: background var(--st-dur-fast) var(--st-ease-out);
  }
  :global(.st-radio-dot[data-state='checked']) {
    background: radial-gradient(circle, #fff 0 3.5px, var(--st-accent) 4px);
    box-shadow: none;
  }
  :global(.st-radio-dot:focus-visible) {
    outline: none;
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--st-text) 34%, transparent), var(--st-focus-ring);
  }
  :global(.st-radio-dot[data-state='checked']:focus-visible) {
    box-shadow: var(--st-focus-ring);
  }
</style>
