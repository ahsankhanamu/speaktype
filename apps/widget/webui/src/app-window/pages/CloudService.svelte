<script lang="ts">
  import { untrack } from 'svelte';
  import { Cloud } from '@lucide/svelte';
  import { Button, Combobox, Group, PopupMenu, Row, SecureField, Switch, TextField, toast } from '@speaktype/ui';
  import { commands, type CloudService } from '../../lib/ipc';
  import Section from '../Section.svelte';

  type CloudConfig = { service: CloudService; baseUrl: string; model: string; fallbackToLocal: boolean };

  const SERVICES = [
    { value: 'openai', label: 'OpenAI', baseUrl: 'https://api.openai.com/v1' },
    { value: 'groq', label: 'Groq', baseUrl: 'https://api.groq.com/openai/v1' },
    { value: 'deepgram', label: 'Deepgram', baseUrl: 'https://api.deepgram.com/v1' },
    { value: 'custom', label: 'Custom OpenAI-compatible', baseUrl: '' },
  ] as const;

  let { initial }: { initial: CloudConfig | null } = $props();
  const start = untrack(() => initial);

  let cfg = $state<CloudConfig>(start ?? { service: 'openai', baseUrl: SERVICES[0].baseUrl, model: '', fallbackToLocal: true });
  let key = $state('');
  let revealed = $state(false);
  let hasKey = $state(false);
  let models = $state<string[]>([]);
  let status = $state<'idle' | 'connecting' | 'connected' | 'failed'>('idle');
  let problem = $state('');
  let testing = $state(false);
  let testLine = $state('');

  const name = $derived(SERVICES.find((s) => s.value === cfg.service)?.label ?? 'the service');
  const message = (error: unknown) => (error as { message?: string })?.message ?? `${error}`;

  $effect(() => {
    commands.sttHasApiKey(cfg.service).then((has) => (hasKey = has));
  });

  async function apply(next: CloudConfig) {
    await commands.sttSetProvider({ kind: 'cloud', ...next });
    cfg = next;
  }

  async function change(next: CloudConfig) {
    try {
      await apply(next);
    } catch (error) {
      toast.show({ message: `Couldn't save that. ${message(error)}` });
    }
  }

  if (start) {
    commands.sttListModels()
      .then((list) => {
        models = list.map((m) => m.id);
        status = 'connected';
      })
      .catch((error) => {
        status = 'failed';
        problem = message(error);
      });
  }

  function pickService(service: CloudService) {
    const known = SERVICES.find((s) => s.value === service)!;
    cfg = { ...cfg, service, baseUrl: known.baseUrl, model: '' };
    key = '';
    models = [];
    status = 'idle';
    testLine = '';
  }

  async function connect() {
    status = 'connecting';
    problem = '';
    try {
      if (key) await commands.sttSetApiKey(cfg.service, key);
      hasKey = hasKey || !!key;
      key = '';
      await apply(cfg);
      models = (await commands.sttListModels()).map((m) => m.id);
      status = 'connected';
    } catch (error) {
      status = 'failed';
      problem = message(error);
    }
  }

  async function reveal() {
    try {
      key = await commands.sttRevealApiKey(cfg.service);
      return true;
    } catch (error) {
      toast.show({ message: `Couldn't show the key. ${message(error)}` });
      return false;
    }
  }

  async function test() {
    testing = true;
    try {
      const { latencyMs } = await commands.sttTest();
      testLine = `Works · ${(latencyMs / 1000).toFixed(1)} s for a 3 s test clip`;
    } catch (error) {
      testLine = message(error);
    } finally {
      testing = false;
    }
  }

  const modelCaption = $derived(
    status === 'connected' ? `Connected · ${models.length} speech models found` : status === 'failed' ? problem : status === 'connecting' ? 'Connecting…' : 'Connect to load models',
  );
</script>

<Section title="Cloud service">
  <Group>
    <Row kind="control" label="1. Service" caption="Known services fill in the address for you">
      {#snippet trailing()}
        <PopupMenu label="Service" items={SERVICES.map(({ value, label }) => ({ value, label }))} bind:value={() => cfg.service, pickService} />
      {/snippet}
    </Row>
    <Row kind="control" label="2. Address" caption="Only needs editing for a custom service">
      {#snippet trailing()}<TextField aria-label="Address" bind:value={cfg.baseUrl} disabled={cfg.service !== 'custom'} />{/snippet}
    </Row>
    <Row kind="control" label="3. API key" caption="Saved in your Keychain. Showing it asks for Touch ID or your Mac password.">
      {#snippet trailing()}
        <SecureField aria-label="API key" bind:value={key} bind:revealed onrequestreveal={reveal} placeholder={hasKey ? 'Saved in Keychain' : 'Paste your key'} />
        <Button size={28} busy={status === 'connecting'} disabled={!key && !hasKey} onclick={connect}>Connect</Button>
      {/snippet}
    </Row>
    <Row kind="control" label="4. Model" caption={modelCaption}>
      {#snippet trailing()}
        {#if status === 'connected'}
          <Combobox label="Model" items={models.map((id) => ({ value: id, label: id }))} bind:value={() => cfg.model, (model) => change({ ...cfg, model })} />
        {:else}
          <TextField aria-label="Model" disabled placeholder="Connect to load models" />
        {/if}
      {/snippet}
    </Row>
    <Row kind="control" label="Test" caption={testLine}>
      {#snippet trailing()}
        <Button size={28} busy={testing} disabled={!cfg.model} onclick={test}>{testLine ? 'Test again' : 'Test'}</Button>
      {/snippet}
    </Row>
  </Group>
</Section>

<Section>
  <Group>
    <Row
      kind="icon"
      label="Your voice will be sent to {name}"
      caption="Only the audio of each dictation and session narration. History, screenshots and sessions stay on this Mac. The widget shows a small cloud while this is on."
      icon={Cloud}
      tileColor="var(--st-warning)"
    />
    <Row kind="control" label="If the service can't be reached, use On this Mac" caption="Needs a downloaded speech model">
      {#snippet trailing()}
        <Switch label="Fall back to On this Mac" bind:checked={() => cfg.fallbackToLocal, (on) => change({ ...cfg, fallbackToLocal: on })} />
      {/snippet}
    </Row>
  </Group>
</Section>
