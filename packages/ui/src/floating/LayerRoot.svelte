<script lang="ts">
  import type { Snippet } from 'svelte';
  import { BitsConfig, Tooltip } from 'bits-ui';
  import TooltipContent from './TooltipContent.svelte';
  import { hint } from './hint.svelte';

  let { children }: { children: Snippet } = $props();
</script>

<BitsConfig defaultPortalTo="#st-layers">
  {@render children()}
  <Tooltip.Provider>
    <Tooltip.Root open={hint.anchor !== null} disableHoverableContent>
      <Tooltip.Portal>
        <TooltipContent label={hint.text} customAnchor={hint.anchor} />
      </Tooltip.Portal>
    </Tooltip.Root>
  </Tooltip.Provider>
</BitsConfig>
<div id="st-layers"></div>
