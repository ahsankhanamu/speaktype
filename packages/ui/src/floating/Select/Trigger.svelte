<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ChevronsUpDown } from '@lucide/svelte';
  import { Select } from 'bits-ui';
  import type { PassThrough } from '../../attrs';
  import '../menu.css';
  import { getLayout } from './context';

  let {
    leading,
    children,
    class: className = '',
    ...rest
  }: { leading?: Snippet; children: Snippet; class?: string; disabled?: boolean } & PassThrough = $props();

  const layout = getLayout();
</script>

<Select.Trigger bind:ref={() => layout.trigger, (el) => (layout.trigger = el)} class="st-trigger {className}" {...rest}>
  {@render leading?.()}
  <span class="label" data-st-label>{@render children()}</span>
  <ChevronsUpDown size={13} strokeWidth={2} aria-hidden="true" />
</Select.Trigger>
