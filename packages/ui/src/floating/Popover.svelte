<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Popover } from 'bits-ui';
  import './menu.css';
  import { pop } from '../motion';

  let {
    open = $bindable(false),
    trigger,
    children,
  }: { open?: boolean; trigger: Snippet<[Record<string, unknown>]>; children: Snippet } = $props();
</script>

<Popover.Root bind:open>
  <Popover.Trigger>
    {#snippet child({ props })}{@render trigger(props)}{/snippet}
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content forceMount sideOffset={6} collisionPadding={8}>
      {#snippet child({ props, wrapperProps, open: shown })}
        {#if shown}
          <div {...wrapperProps}>
            <div {...props} class="st-menu" style="padding: 12px" transition:pop>
              {@render children()}
            </div>
          </div>
        {/if}
      {/snippet}
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>
