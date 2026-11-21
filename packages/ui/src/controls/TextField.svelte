<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import './field.css';

  let {
    value = $bindable(''),
    error,
    disabled = false,
    class: className = '',
    ...rest
  }: { value?: string; error?: string; disabled?: boolean } & Omit<HTMLInputAttributes, 'value'> = $props();

  const errorId = $props.id();
</script>

<div class={className}>
  <label class="st-field" data-invalid={error ? '' : undefined} data-disabled={disabled ? '' : undefined}>
    <input
      class="st-field-input"
      bind:value
      {disabled}
      aria-invalid={error ? 'true' : undefined}
      aria-describedby={error ? errorId : undefined}
      {...rest}
    />
  </label>
  {#if error}<p id={errorId} class="st-field-error">{error}</p>{/if}
</div>
