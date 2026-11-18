<script lang="ts">
  import { Alert, confirmAlert } from '@speaktype/ui';
  import { onMount } from 'svelte';

  onMount(() => {
    void confirmAlert({
      title: 'Delete this session?',
      message: "Its marks, audio and prompt are deleted from this Mac. This can't be undone.",
      confirmLabel: 'Delete',
      destructive: true,
    });
  });
</script>

<Alert />
