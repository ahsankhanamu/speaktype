<script lang="ts">
  import { Eye, EyeOff } from '@lucide/svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';
  import './field.css';

  const REMASK_MS = 30_000;

  let {
    value = $bindable(''),
    revealed = $bindable(false),
    onrequestreveal,
    class: className = '',
    ...rest
  }: { value?: string; revealed?: boolean; onrequestreveal?: () => Promise<boolean> } & Omit<HTMLInputAttributes, 'value' | 'type'> = $props();

  async function toggle() {
    revealed = revealed ? false : (await onrequestreveal?.()) ?? true;
  }

  $effect(() => {
    if (!revealed) return;
    const mask = () => (revealed = false);
    const timer = setTimeout(mask, REMASK_MS);
    window.addEventListener('blur', mask);
    return () => {
      clearTimeout(timer);
      window.removeEventListener('blur', mask);
    };
  });
</script>

<label class="st-field {className}">
  <input class="st-field-input" type={revealed ? 'text' : 'password'} autocomplete="off" spellcheck="false" bind:value {...rest} />
  <button type="button" class="st-field-icon-btn" aria-label={revealed ? 'Hide' : 'Show'} aria-pressed={revealed} onclick={toggle}>
    {#if revealed}<EyeOff size={14} aria-hidden="true" />{:else}<Eye size={14} aria-hidden="true" />{/if}
  </button>
</label>
