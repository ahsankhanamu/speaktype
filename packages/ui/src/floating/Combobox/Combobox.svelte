<script lang="ts">
  import { Check, ChevronsUpDown, Search } from '@lucide/svelte';
  import { Combobox } from 'bits-ui';
  import '../../controls/field.css';
  import '../menu.css';
  import { pop } from '../../motion';

  type Item = { value: string; label: string; hint?: string };

  let {
    items,
    value = $bindable(),
    open = $bindable(false),
    query = $bindable(''),
    label,
    emptyMessage = () => 'No matches',
  }: { items: Item[]; value: string; open?: boolean; query?: string; label: string; emptyMessage?: (query: string) => string } = $props();

  let field = $state<HTMLElement | null>(null);
  const selectedLabel = $derived(items.find((item) => item.value === value)?.label ?? '');
  const needle = $derived(query.trim().toLowerCase());
  const matches = $derived(needle ? items.filter((item) => item.label.toLowerCase().includes(needle)) : items);

  function parts(text: string) {
    const at = needle ? text.toLowerCase().indexOf(needle) : -1;
    return at < 0 ? { before: '', bold: '', after: text } : { before: text.slice(0, at), bold: text.slice(at, at + needle.length), after: text.slice(at + needle.length) };
  }

  function reset(isOpen: boolean) {
    if (!isOpen) query = '';
  }
</script>

<Combobox.Root type="single" bind:value bind:open {items} inputValue={open ? query : selectedLabel} onOpenChange={reset}>
  <div class="st-field" bind:this={field}>
    <Search size={14} strokeWidth={2.2} aria-hidden="true" />
    <Combobox.Input
      class="st-field-input"
      aria-label={label}
      placeholder={selectedLabel}
      oninput={(e) => (query = e.currentTarget.value)}
    />
    {#if query && matches.length}<span class="count">{matches.length}</span>{/if}
    {#if !open}
      <Combobox.Trigger class="st-field-icon-btn" aria-label="Show {label} options" tabindex={-1}>
        <ChevronsUpDown size={13} strokeWidth={2} aria-hidden="true" />
      </Combobox.Trigger>
    {/if}
  </div>
  <Combobox.Portal>
    <Combobox.Content forceMount sideOffset={6} collisionPadding={8} customAnchor={field}>
      {#snippet child({ props, wrapperProps, open })}
        {#if open}
          <div {...wrapperProps}>
            <div {...props} class="st-menu" style:min-width="var(--bits-combobox-anchor-width)" transition:pop={{ placement: 'below' }}>
              {#each matches as item (item.value)}
                {@const { before, bold, after } = parts(item.label)}
                <Combobox.Item value={item.value} label={item.label} class="st-menu-item">
                  {#snippet children({ selected })}
                    <span class="label">{before}{#if bold}<b>{bold}</b>{/if}{after}</span>
                    {#if item.hint}<span class="hint">{item.hint}</span>{/if}
                    {#if selected}<Check size={12} strokeWidth={2.2} aria-hidden="true" />{/if}
                  {/snippet}
                </Combobox.Item>
              {:else}
                <div class="st-menu-empty">{emptyMessage(query)}</div>
              {/each}
            </div>
          </div>
        {/if}
      {/snippet}
    </Combobox.Content>
  </Combobox.Portal>
</Combobox.Root>

<style>
  .count,
  .hint {
    color: var(--st-text-tertiary);
    font-size: 11px;
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.st-menu-item[data-highlighted]) .hint {
    color: rgba(255, 255, 255, 0.75);
  }
</style>
