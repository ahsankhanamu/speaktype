<script lang="ts">
  import { useEvent } from './useEvent.svelte';

  let {
    name,
    callback,
  }: {
    name: string;
    callback: (payload: unknown) => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  useEvent(name, callback);
</script>
