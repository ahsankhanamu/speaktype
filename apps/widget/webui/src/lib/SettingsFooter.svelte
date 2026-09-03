<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    left,
    right,
    hidden = false,
  }: {
    left?: Snippet | undefined;
    right?: Snippet | undefined;
    hidden?: boolean;
  } = $props();
</script>

<footer class="pane-footer" {hidden}>
  <div class="actions">
    <div class="actions-left">
      {@render left?.()}
    </div>
    <div class="actions-right">
      {@render right?.()}
    </div>
  </div>
</footer>

<style>
  .pane-footer {
    padding: var(--space-3) var(--space-6);
  }

  .pane-footer[hidden] {
    display: none;
  }

  :global(.actions) {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 38px;
  }

  :global(.actions-left),
  :global(.actions-right) {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  :global(.actions-right) {
    margin-left: auto;
  }
</style>