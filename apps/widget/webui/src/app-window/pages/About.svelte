<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Button, Group, Row } from '@speaktype/ui';
  import { liveQuery } from '../../lib/data';
  import Page from '../Page.svelte';
  import Section from '../Section.svelte';

  const version = liveQuery('app-version', getVersion, { invalidateOn: [], refetchOnFocus: false });
</script>

<Page title="About SpeakType" lede="Voice typing that works everywhere. Press a key, speak, and your words appear.">
  <Section>
    <Group>
      <Row kind="control" label="Version">
        {#snippet trailing()}<span class="value">{version.data ?? ''}</span>{/snippet}
      </Row>
      <Row kind="control" label="Created by Ahsan Khan" caption="© 2026 Ahsan Khan. All rights reserved.">
        {#snippet trailing()}<Button size={28} onclick={() => openUrl('https://ahsankhan.dev')}>Visit website</Button>{/snippet}
      </Row>
    </Group>
  </Section>
</Page>

<style>
  .value {
    color: var(--st-text-secondary);
    font-size: var(--st-font-body);
  }
</style>
