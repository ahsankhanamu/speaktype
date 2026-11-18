<script lang="ts">
  import Buttons from './Buttons.svelte';
  import Choices from './Choices.svelte';
  import Dropdown from './Dropdown.svelte';
  import Feedback from './Feedback.svelte';
  import Status from './Status.svelte';
  import TextFields from './TextFields.svelte';
</script>

<main class="page board">
  <header>
    <h1>Components</h1>
    <p>Every control, every state. Built on bits-ui (shadcn-style, composable). Capsules everywhere: 32 pt in panels and toolbars, 28 pt inside settings rows.</p>
  </header>
  <div class="grid">
    <Buttons />
    <TextFields />
    <Dropdown />
    <Choices />
    <Status />
    <Feedback />
  </div>
</main>

<style>
  .board {
    display: flex;
    flex-direction: column;
    gap: 22px;
    min-height: 1740px;
  }
  h1 {
    margin: 0;
    font-size: var(--st-font-title);
    font-weight: 700;
    letter-spacing: -0.022em;
  }
  p {
    margin: 4px 0 0;
    color: var(--st-text-secondary);
  }
  .board :global(.page) {
    display: contents;
    padding: 0;
  }
</style>
