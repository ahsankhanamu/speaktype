<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Select } from 'bits-ui';
  import { setLayout } from './context';

  let {
    placement = 'aligned',
    value = $bindable(''),
    open = $bindable(false),
    disabled = false,
    children,
  }: { placement?: 'aligned' | 'below'; value?: string; open?: boolean; disabled?: boolean; children: Snippet } = $props();

  let trigger = $state<HTMLElement | null>(null);

  setLayout({
    get trigger() {
      return trigger;
    },
    set trigger(el) {
      trigger = el;
    },
    get placement() {
      return placement;
    },
  });
</script>

<Select.Root type="single" bind:value bind:open {disabled} {children} />
