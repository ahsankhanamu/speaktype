<script lang="ts">
  import * as Select from '$lib/ui/select/index.js';

  let {
    value = $bindable(''),
    placeholder = '',
    disabled = false,
    ariaLabel = undefined,
    options = [],
  }: {
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    ariaLabel?: string;
    options: Array<{ value: string; label: string; disabled?: boolean }>;
  } = $props();

  const triggerLabel = $derived(
    options.find(o => o.value === value)?.label ?? placeholder ?? '',
  );
</script>

<Select.Root bind:value={value as never} {disabled}>
  <Select.Trigger aria-label={ariaLabel}>
    {triggerLabel}
  </Select.Trigger>
  <Select.Content>
    {#each options as opt (opt.value)}
      <Select.Item value={opt.value} label={opt.label} disabled={opt.disabled} />
    {/each}
  </Select.Content>
</Select.Root>