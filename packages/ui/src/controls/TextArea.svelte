<script lang="ts">
  import type { HTMLTextareaAttributes } from 'svelte/elements';
  import './field.css';

  let {
    value = $bindable(''),
    error,
    disabled = false,
    rows = 2,
    class: className = '',
    ...rest
  }: { value?: string; error?: string; disabled?: boolean } & Omit<HTMLTextareaAttributes, 'value'> = $props();

  const errorId = $props.id();
</script>

<div class={className}>
  <label class="st-field" data-multiline data-invalid={error ? '' : undefined} data-disabled={disabled ? '' : undefined}>
    <textarea
      class="st-field-input"
      bind:value
      {rows}
      {disabled}
      aria-invalid={error ? 'true' : undefined}
      aria-describedby={error ? errorId : undefined}
      {...rest}
    ></textarea>
  </label>
  {#if error}<p id={errorId} class="st-field-error">{error}</p>{/if}
</div>
