<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Tooltip } from 'bits-ui';
  import TooltipContent from './TooltipContent.svelte';

  let { label, shortcut, children }: { label: string; shortcut?: string[]; children: Snippet<[Record<string, unknown>]> } = $props();
</script>

<Tooltip.Provider delayDuration={500}>
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}{@render children(props)}{/snippet}
    </Tooltip.Trigger>
    <Tooltip.Portal>
      <TooltipContent {label} {shortcut} />
    </Tooltip.Portal>
  </Tooltip.Root>
</Tooltip.Provider>
