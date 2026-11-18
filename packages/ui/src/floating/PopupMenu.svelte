<script lang="ts" generics="T extends string">
  import type { Snippet } from 'svelte';
  import { ChevronsUpDown } from '@lucide/svelte';
  import { DropdownMenu } from 'bits-ui';
  import './menu.css';
  import { alignTo } from './aligned';
  import { MenuItem, MenuSeparator } from './Menu';
  import { pop } from '../motion';

  type Choice = { value: T; label: string; icon?: Snippet; separatorBefore?: boolean };

  let {
    items,
    value = $bindable(),
    actions = [],
    label,
  }: { items: Choice[]; value: T; actions?: { label: string; onselect: () => void }[]; label: string } = $props();

  let trigger = $state<HTMLElement | null>(null);
  const current = $derived(items.find((item) => item.value === value));
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger bind:ref={trigger} class="st-trigger" aria-label={label}>
    {#if current?.icon}<span class="icon">{@render current.icon()}</span>{/if}
    <span class="label" data-st-label>{current?.label}</span>
    <ChevronsUpDown size={13} strokeWidth={2} aria-hidden="true" />
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <DropdownMenu.ContentStatic forceMount>
      {#snippet child({ props, open })}
        {#if open}
          <div class="st-aligned" use:alignTo={trigger}>
            <div {...props} class="st-menu" style="--menu-item-height: 30px" transition:pop={{ placement: 'aligned' }}>
              <DropdownMenu.RadioGroup bind:value>
                {#each items as item (item.value)}
                  {#if item.separatorBefore}<MenuSeparator />{/if}
                  <MenuItem value={item.value} label={item.label} icon={item.icon} />
                {/each}
              </DropdownMenu.RadioGroup>
              {#if actions.length}
                <MenuSeparator />
                {#each actions as action (action.label)}<MenuItem label={action.label} onSelect={action.onselect} />{/each}
              {/if}
            </div>
          </div>
        {/if}
      {/snippet}
    </DropdownMenu.ContentStatic>
  </DropdownMenu.Portal>
</DropdownMenu.Root>

<style>
  .icon {
    display: inline-flex;
    flex: none;
    width: 22px;
  }
</style>
