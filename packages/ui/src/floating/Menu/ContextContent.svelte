<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ContextMenu } from 'bits-ui';
  import '../menu.css';
  import { pop } from '../../motion';

  let { children }: { children: Snippet } = $props();
</script>

<ContextMenu.Portal>
  <ContextMenu.Content forceMount collisionPadding={8}>
    {#snippet child({ props, wrapperProps, open })}
      {#if open}
        <div {...wrapperProps}>
          <div {...props} class="st-menu" style="--menu-item-height: 30px" transition:pop>
            {@render children()}
          </div>
        </div>
      {/if}
    {/snippet}
  </ContextMenu.Content>
</ContextMenu.Portal>
