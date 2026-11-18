<script lang="ts">
  import { Check } from '@lucide/svelte';
  import { Select } from 'bits-ui';
  import '../menu.css';

  let { value, label, disabled = false }: { value: string; label: string; disabled?: boolean } = $props();
</script>

<Select.Item {value} {label} {disabled} class="st-menu-item">
  {#snippet children({ selected })}
    <span class="label" data-st-label>{label}</span>
    {#if selected}<Check size={12} strokeWidth={2.2} aria-hidden="true" />{/if}
  {/snippet}
</Select.Item>

<style>
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
