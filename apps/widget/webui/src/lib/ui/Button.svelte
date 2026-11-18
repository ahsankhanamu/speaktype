<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  let {
    children,
    type = 'button',
    variant = 'default',
    size = 'default',
    disabled = false,
    class: extra = '',
    ...rest
  }: {
    children?: Snippet;
    type?: 'button' | 'submit' | 'reset';
    variant?: 'default' | 'primary' | 'secondary' | 'danger';
    size?: 'default' | 'small';
    disabled?: boolean;
    class?: string;
  } & HTMLButtonAttributes = $props();
</script>

<button
  {type}
  {disabled}
  class={`btn ${variant !== 'default' ? variant : ''} ${size === 'small' ? 'btn-small' : ''} ${extra}`.trim()}
  {...rest}
>
  {@render children?.()}
</button>