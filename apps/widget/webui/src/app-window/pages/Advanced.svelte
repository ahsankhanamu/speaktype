<script lang="ts">
  import { Button, Group, Row, Segmented, Switch, TextField, toast } from '@speaktype/ui';
  import { liveQuery } from '../../lib/data';
  import { checkServer, getServerStatus, loadServer, serverInfo, unloadModel } from '../../lib/ipc';
  import Page from '../Page.svelte';
  import Section from '../Section.svelte';
  import { useSettings } from '../settings.svelte';
  import DebugRecords from './DebugRecords.svelte';

  const settings = useSettings();
  const guard = settings.field('hallucination_guard', true);
  const retries = settings.field('hallucination_retries', 1);
  const THRESHOLDS = [
    { key: 'quality_no_speech_prob', label: 'Silence threshold', caption: 'Above this chance of silence, a segment is dropped' },
    { key: 'quality_avg_logprob', label: 'Confidence threshold', caption: 'Below this average confidence, the model was guessing' },
    { key: 'quality_compression_ratio', label: 'Repetition threshold', caption: 'Above this ratio, the text is repeating itself' },
  ] as const;

  const statusQuery = liveQuery('server-status', getServerStatus, {
    invalidateOn: ['server:starting', 'server:ready', 'server:stopped', 'server:error'],
  });
  const model = liveQuery('server-info', serverInfo, {
    invalidateOn: ['server:starting', 'server:ready', 'server:stopped', 'server:error'],
  });
  let apiUrl = $state<string | null>(null);
  let checkResult = $state('');
  let checkTone = $state<'success' | 'danger'>();
  let busy = $state<'unload' | 'load' | 'check' | null>(null);

  const url = $derived(apiUrl ?? settings.data?.api_url ?? '');
  const inMemory = $derived(Boolean(model.data?.model));
  const serverCaption = $derived(
    !statusQuery.data ? ''
      : !statusQuery.data.embedded ? 'This build has no bundled server'
      : statusQuery.data.running && model.data?.model ? `Running · ${model.data.size ?? 'size unknown'} in memory`
      : statusQuery.data.running ? `Running on port ${statusQuery.data.port}`
      : statusQuery.data.stopped ? 'Not loaded · loads again on the next dictation or when you press Load now'
      : 'Starting…',
  );

  async function load() {
    busy = 'load';
    try {
      await loadServer();
      await Promise.all([statusQuery.refetch(), model.refetch()]);
    } catch (error) {
      toast.show({ message: `Couldn't load the model. ${(error as { message?: string })?.message ?? error}` });
    } finally {
      busy = null;
    }
  }

  async function unload() {
    busy = 'unload';
    try {
      await unloadModel();
      await Promise.all([statusQuery.refetch(), model.refetch()]);
    } catch (error) {
      toast.show({ message: `Couldn't unload the model. ${(error as { message?: string })?.message ?? error}` });
    } finally {
      busy = null;
    }
  }

  async function check() {
    busy = 'check';
    try {
      const result = await checkServer(url);
      const connected = result.status === 'connected';
      checkResult = connected ? 'Connected' : (result.message ?? 'Couldn’t connect');
      checkTone = connected ? 'success' : 'danger';
    } catch (error) {
      checkResult = (error as { message?: string })?.message ?? `${error}`;
      checkTone = 'danger';
    } finally {
      busy = null;
    }
  }

  const setThreshold = (key: (typeof THRESHOLDS)[number]['key'], text: string) => {
    const value = Number(text);
    if (text.trim() && Number.isFinite(value)) settings.save({ [key]: value });
  };
</script>

<Page title="Advanced">
  <Section title="Hallucination filter">
    <Group>
      <Row kind="control" label="Hallucination filter" caption="Drops text a model invents from silence or noise, and tries again">
        {#snippet trailing()}<Switch label="Hallucination filter" bind:checked={guard.value} />{/snippet}
      </Row>
      <Row kind="control" label="Retries" caption="Extra attempts when a result looks wrong">
        {#snippet trailing()}
          <Segmented
            label="Retries"
            bind:value={() => String(retries.value), (next) => (retries.value = Number(next))}
            items={[{ value: '0', label: '0' }, { value: '1', label: '1' }, { value: '2', label: '2' }]}
          />
        {/snippet}
      </Row>
      {#each THRESHOLDS as { key, label, caption } (key)}
        <Row kind="control" {label} {caption}>
          {#snippet trailing()}
            <TextField
              aria-label={label}
              inputmode="decimal"
              value={String(settings.data?.[key] ?? '')}
              onchange={(event) => setThreshold(key, event.currentTarget.value)}
            />
          {/snippet}
        </Row>
      {/each}
    </Group>
  </Section>

  <Section title="Transcription server">
    <Group>
      <Row kind="control" label="Transcription server" caption={serverCaption}>
        {#snippet trailing()}
          {#if statusQuery.data?.embedded}
            {#if inMemory}
              <Button size={28} busy={busy === 'unload'} onclick={unload}>Unload model</Button>
            {:else if statusQuery.data.stopped}
              <Button size={28} busy={busy === 'load'} onclick={load}>Load now</Button>
            {/if}
          {/if}
        {/snippet}
      </Row>
      <Row kind="control" label="API URL" caption={checkResult} captionTone={checkTone}>
        {#snippet trailing()}
          <TextField class="api-url" aria-label="API URL" bind:value={() => url, (next) => (apiUrl = next)} onchange={() => settings.save({ api_url: url })} />
          <Button size={28} busy={busy === 'check'} onclick={check}>Check</Button>
        {/snippet}
      </Row>
    </Group>
  </Section>

  <DebugRecords />
</Page>

<style>
  /* Wide enough for a full local address (http://127.0.0.1:8002/inference). */
  :global(.api-url) {
    width: 300px;
  }
  :global(.api-url .st-field) {
    width: 100%;
  }
</style>
