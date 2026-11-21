<script lang="ts">
  import { Search, X } from '@lucide/svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';
  import './field.css';

  let {
    value = $bindable(''),
    placeholder = 'Search',
    disabled = false,
    class: className = '',
    ...rest
  }: { value?: string; disabled?: boolean } & Omit<HTMLInputAttributes, 'value'> = $props();
</script>

<label class="st-field {className}" data-disabled={disabled ? '' : undefined}>
  <Search size={14} strokeWidth={2.2} aria-hidden="true" />
  <input class="st-field-input" type="text" role="searchbox" bind:value {placeholder} {disabled} {...rest} />
  {#if value}
    <button type="button" class="st-field-icon-btn" aria-label="Clear search" onclick={() => (value = '')}>
      <X size={14} aria-hidden="true" />
    </button>
  {/if}
</label>
