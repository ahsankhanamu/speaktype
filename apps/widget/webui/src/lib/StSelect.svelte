<script lang="ts">
  export let value: string | undefined = undefined;
  export let placeholder: string = '';
  export let disabled: boolean = false;
  export let ariaLabel: string | undefined = undefined;

  export let options: Array<{ value: string; label: string; disabled?: boolean }> = [];
</script>

<select
  bind:value
  {disabled}
  aria-label={ariaLabel}
  class="st-select"
>
  {#if placeholder}
    <option value="" disabled>{placeholder}</option>
  {/if}
  {#each options as opt (opt.value)}
    <option value={opt.value} disabled={opt.disabled}>{opt.label}</option>
  {/each}
</select>

<style>
  .st-select {
    width: 100%;
    padding: 10px 12px;
    background: var(--surface-2, #252525);
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    color: var(--text-strong, #fff);
    font-size: 14px;
    font-family: inherit;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s, background-color 0.15s;
    box-sizing: border-box;
  }

  .st-select:hover:not(:disabled) {
    background: var(--surface-3, #2a2a2a);
  }

  .st-select:focus-visible {
    outline: none;
    border-color: var(--accent, #3b82f6);
    box-shadow: 0 0 0 3px var(--accent-ring, rgba(59, 130, 246, 0.35));
  }

  .st-select:disabled {
    cursor: default;
    opacity: 0.55;
  }
</style>