<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Select } from 'bits-ui';
  import '../menu.css';
  import { alignTo } from '../aligned';
  import { pop } from '../../motion';
  import { getLayout } from './context';

  let { children }: { children: Snippet } = $props();

  const layout = getLayout();
</script>

<Select.Portal>
  {#if layout.placement === 'below'}
    <Select.Content forceMount sideOffset={6} collisionPadding={8}>
      {#snippet child({ props, wrapperProps, open })}
        {#if open}
          <div {...wrapperProps}>
            <div {...props} class="st-menu" style:min-width="var(--bits-select-anchor-width)" transition:pop={{ placement: 'below' }}>
              {@render children()}
            </div>
          </div>
        {/if}
      {/snippet}
    </Select.Content>
  {:else}
    <Select.ContentStatic forceMount>
      {#snippet child({ props, open })}
        {#if open}
          <div class="st-aligned" use:alignTo={layout.trigger}>
            <div {...props} class="st-menu" transition:pop={{ placement: 'aligned' }}>
              {@render children()}
            </div>
          </div>
        {/if}
      {/snippet}
    </Select.ContentStatic>
  {/if}
</Select.Portal>
