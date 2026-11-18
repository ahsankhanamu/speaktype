<script lang="ts">
  import Button from '../controls/Button.svelte';
  import Sheet from './Sheet.svelte';

  let open = $state(false);
</script>

<Button onclick={() => (open = true)}>Open</Button>
<Sheet bind:open title="Speech models">
  <p>Body</p>
  {#snippet footer()}<Button variant="primary" onclick={() => (open = false)}>Done</Button>{/snippet}
</Sheet>
