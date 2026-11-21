<script lang="ts">
  import { Button, Group, Row, Select, Sheet, truncate } from '@speaktype/ui';

  let edge = $state('item-6');
  let inSheet = $state('item-2');
  let sheetOpen = $state(false);
  let toggleOpen = $state(false);

  const items = Array.from({ length: 8 }, (_, i) => ({ value: `item-${i + 1}`, label: `Item ${i + 1}` }));
  const longPath = '/Users/someone/Library/Application Support/SpeakType/Sessions/2026-11-19-checkout-bug/images/03-mark.png';
</script>

<div class="edge">
  <Select.Root bind:value={edge}>
    <Select.Trigger data-testid="edge-trigger">{edge}</Select.Trigger>
    <Select.Content>
      {#each items as item}<Select.Item value={item.value} label={item.label} />{/each}
    </Select.Content>
  </Select.Root>
</div>

<div class="stack">
  <Button data-testid="open-sheet" onclick={() => (sheetOpen = true)}>Open sheet with select</Button>
  <Button data-testid="toggle-sheet" onclick={() => (toggleOpen = !toggleOpen)}>Toggle</Button>
  <div class="path"><span data-testid="long-path" use:truncate={{ mode: 'middle', full: longPath }}></span></div>
  <div class="path"><span data-testid="short-path" use:truncate={{ mode: 'middle', full: '~/Desktop/a.png' }}></span></div>
  <div class="path"><span data-testid="long-text" use:truncate={{ mode: 'tail', full: 'A long transcript line that does not fit into its row at all, however wide.' }}></span></div>
  <div class="variants">
    {#each ['primary', 'secondary', 'plain', 'destructive'] as const as variant}
      <Button data-testid="variant-{variant}" {variant}>{variant}</Button>
    {/each}
  </div>
  <Button data-testid="idle-button" variant="primary">Copy</Button>
  <Button data-testid="busy-button" variant="primary" busy>Copy</Button>
  <Group>
    <Row kind="info" label="Info" />
    <Row kind="control" label="Control" />
    <Row kind="icon" label="Icon" caption="Two lines" />
  </Group>
</div>

<Sheet bind:open={sheetOpen} title="Sheet with a select">
  <Select.Root bind:value={inSheet} placement="aligned">
    <Select.Trigger data-testid="sheet-trigger">{inSheet}</Select.Trigger>
    <Select.Content>
      {#each items as item}<Select.Item value={item.value} label={item.label} />{/each}
    </Select.Content>
  </Select.Root>
  {#snippet footer()}<Button variant="primary" onclick={() => (sheetOpen = false)}>Done</Button>{/snippet}
</Sheet>

<Sheet bind:open={toggleOpen} title="Toggled">
  <p>Body</p>
</Sheet>

<style>
  .edge {
    position: fixed;
    top: 600px;
    left: 200px;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 480px;
    padding: 24px;
  }
  .variants {
    display: flex;
    gap: 8px;
  }
  .path {
    width: 200px;
    font-size: var(--st-font-info);
  }
  .path span {
    display: block;
  }
</style>
