<script lang="ts">
  import { Button, Group, Row, TextField, toast } from '@speaktype/ui';
  import { checkServer, commands, type ProviderConfig } from '../../lib/ipc';
  import Page from '../Page.svelte';
  import Section from '../Section.svelte';
  import { useSettings } from '../settings.svelte';
  import CloudService from './CloudService.svelte';
  import ProviderCards from './ProviderCards.svelte';

  const settings = useSettings();

  let picked = $state<ProviderConfig['kind'] | null>(null);
  let address = $state<string | null>(null);
  let checking = $state(false);
  let checkLine = $state('');

  const provider = $derived(settings.data?.stt_provider ?? null);
  const kind = $derived(picked ?? provider?.kind ?? 'local');
  const url = $derived(address ?? (provider?.kind === 'server' ? provider.baseUrl : settings.data?.api_url) ?? '');

  async function use(config: ProviderConfig) {
    try {
      await commands.sttSetProvider(config);
      await settings.refetch();
    } catch (error) {
      toast.show({ message: `Couldn't switch provider. ${(error as { message?: string })?.message ?? error}` });
    }
  }

  async function choose(next: ProviderConfig['kind']) {
    picked = next;
    if (next === 'local') await use({ kind: 'local', model: settings.data?.model ?? '' });
    if (next === 'server') await use({ kind: 'server', baseUrl: url });
  }

  async function saveAddress() {
    await settings.save({ api_url: url });
    await use({ kind: 'server', baseUrl: url });
  }

  async function check() {
    checking = true;
    try {
      const result = await checkServer(url);
      checkLine = result.status === 'connected' ? 'Connected' : (result.message ?? 'Couldn’t connect');
    } catch (error) {
      checkLine = (error as { message?: string })?.message ?? `${error}`;
    } finally {
      checking = false;
    }
  }
</script>

<Page title="Providers" lede="Choose who turns your voice into text. On this Mac is private and works offline; a service can be faster on older Macs.">
  <Section title="Speech to text">
    <ProviderCards bind:value={() => kind, choose} />
  </Section>

  {#if kind === 'server'}
    <Section title="Your own server">
      <Group>
        <Row kind="control" label="Address" caption={checkLine || 'A whisper.cpp-compatible server'}>
          {#snippet trailing()}
            <TextField aria-label="Address" bind:value={() => url, (next) => (address = next)} onchange={saveAddress} />
            <Button size={28} busy={checking} onclick={check}>Test</Button>
          {/snippet}
        </Row>
      </Group>
    </Section>
  {:else if kind === 'cloud' && settings.data}
    <CloudService initial={provider?.kind === 'cloud' ? provider : null} />
  {/if}
</Page>
