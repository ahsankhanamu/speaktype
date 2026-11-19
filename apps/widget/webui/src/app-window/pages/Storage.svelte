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

  // History keeps either a number of dictations or an age. A missing count is a pre-v2 settings
  // file, which keeps 100; null for both means forever.
  const limit = $derived(settings.data?.history_limit === undefined ? 100 : settings.data.history_limit);
  const days = $derived(settings.data?.history_max_days ?? null);
  const choice = $derived(days !== null ? `days:${days}` : limit === null ? 'forever' : `count:${limit}`);

  function setRetention(value: string) {
    const [kind, amount] = value.split(':');
    settings.save({
      history_limit: kind === 'count' ? Number(amount) : null,
      history_max_days: kind === 'days' ? Number(amount) : null,
    });
  }

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
      <Row kind="control" label="Keep history" caption="Older dictations are removed as new ones come in">
        {#snippet trailing()}
          <PopupMenu
            label="Keep history"
            bind:value={() => choice, setRetention}
            items={[
              { value: 'count:100', label: 'Last 100' },
              { value: 'count:500', label: 'Last 500' },
              { value: 'count:1000', label: 'Last 1,000' },
              { value: 'days:30', label: '30 days', separatorBefore: true },
              { value: 'days:90', label: '3 months' },
              { value: 'days:365', label: '1 year' },
              { value: 'forever', label: 'Forever', separatorBefore: true },
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
