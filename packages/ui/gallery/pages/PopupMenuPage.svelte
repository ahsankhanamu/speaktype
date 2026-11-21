<script lang="ts" module>
  import { createRawSnippet } from 'svelte';

  const screen = (dot: string) =>
    createRawSnippet(() => ({ render: () => `<span class="screen"><i style="${dot}"></i></span>` }));

  const spots = [
    { value: 'bottom-right', label: 'Bottom right', icon: screen('right: 3px; bottom: 3px') },
    { value: 'bottom-centre', label: 'Bottom centre', icon: screen('left: 8px; bottom: 3px') },
    { value: 'top-right', label: 'Top right', icon: screen('right: 3px; top: 3px') },
    { value: 'bottom-left', label: 'Bottom left', icon: screen('left: 3px; bottom: 3px') },
    { value: 'last', label: 'Where I last dragged it', separatorBefore: true },
  ];
</script>

<script lang="ts">
  import { PopupMenu } from '@speaktype/ui';

  let position = $state('bottom-right');
</script>

<div class="page">
  <section class="card holder">
    <p class="sec">Pop-up menu</p>
    <p class="cap">The menu opens with the current item exactly over the button, labels on one shared column.</p>
    <div class="trigger" data-screenshot-click>
      <PopupMenu
        label="Widget position"
        bind:value={position}
        items={spots}
        actions={[{ label: 'Reset widget position', onselect: () => (position = 'bottom-right') }]}
      />
    </div>
  </section>
</div>

<style>
  .holder {
    min-height: 520px;
  }
  .trigger {
    margin: 120px 0 0 240px;
    width: 240px;
  }
  :global(.screen) {
    position: relative;
    display: block;
    width: 22px;
    height: 16px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--st-text) 50%, transparent);
  }
  :global(.screen i) {
    position: absolute;
    width: 6px;
    height: 4px;
    border-radius: 2px;
    background: var(--st-text);
  }
</style>
