<script lang="ts">
  import { Button, Group, PopupMenu, Row, confirmAlert, toast } from '@speaktype/ui';
  import { useModels } from '../../lib/app/models.svelte';
  import { formatByteSize } from '../../lib/format';
  import { clearHistory, openModelsFolder, openRecordingsFolder } from '../../lib/ipc';
  import Page from '../Page.svelte';
  import Section from '../Section.svelte';
  import { useSettings } from '../settings.svelte';

  const settings = useSettings();
  const models = useModels();

  // A missing field is a pre-v2 settings file, which keeps 100 entries; null means forever.
  const limit = $derived(settings.data?.history_limit === undefined ? 100 : settings.data.history_limit);
  const choice = $derived(limit === null ? 'forever' : String(limit));

  const setLimit = (value: string) => settings.save({ history_limit: value === 'forever' ? null : Number(value) });

  async function clear() {
    const confirmed = await confirmAlert({
      title: 'Delete all dictations?',
      message: 'This removes your dictation history and any saved audio from this Mac. It can’t be undone.',
      confirmLabel: 'Delete all',
      destructive: true,
    });
    if (!confirmed) return;
    await clearHistory();
    toast.show({ message: 'History deleted' });
  }
</script>

<Page title="Storage">
  <Section title="History">
    <Group>
      <Row kind="control" label="Keep history" caption="Older dictations are removed once you pass the limit">
        {#snippet trailing()}
          <PopupMenu
            label="Keep history"
            bind:value={() => choice, setLimit}
            items={[
              { value: '100', label: 'Last 100' },
              { value: '1000', label: 'Last 1,000' },
              { value: 'forever', label: 'Forever' },
            ]}
          />
        {/snippet}
      </Row>
      <Row kind="control" label="Clear history" caption="Delete every dictation and its saved audio">
        {#snippet trailing()}<Button size={28} variant="destructive" onclick={clear}>Delete all…</Button>{/snippet}
      </Row>
    </Group>
  </Section>

  <Section title="On this Mac">
    <Group>
      <Row kind="control" label="Speech models on disk" caption={models.loaded ? formatByteSize(models.bytesOnDisk) || '0 MB' : ''}>
        {#snippet trailing()}<Button size={28} onclick={() => openModelsFolder()}>Show in Finder</Button>{/snippet}
      </Row>
      <Row kind="control" label="Saved recordings" caption="Audio kept while Save recording audio is on">
        {#snippet trailing()}<Button size={28} onclick={() => openRecordingsFolder()}>Show in Finder</Button>{/snippet}
      </Row>
    </Group>
  </Section>
</Page>
