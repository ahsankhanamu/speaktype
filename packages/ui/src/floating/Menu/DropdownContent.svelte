<script lang="ts">
  import type { Snippet } from 'svelte';
  import { DropdownMenu } from 'bits-ui';
  import '../menu.css';
  import { pop } from '../../motion';

  let { children }: { children: Snippet } = $props();
</script>

<DropdownMenu.Portal>
  <DropdownMenu.Content forceMount sideOffset={6} align="start" collisionPadding={8}>
    {#snippet child({ props, wrapperProps, open })}
      {#if open}
        <div {...wrapperProps}>
          <div {...props} class="st-menu" transition:pop>
            {@render children()}
          </div>
        </div>
      {/if}
    {/snippet}
  </DropdownMenu.Content>
</DropdownMenu.Portal>
