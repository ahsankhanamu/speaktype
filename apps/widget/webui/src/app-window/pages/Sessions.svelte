<script lang="ts">
  import { Button, Group, Row, Switch } from '@speaktype/ui';
  import { openSystemPane } from '../../lib/ipc';
  import Page from '../Page.svelte';
  import { navigate } from '../router.svelte';
  import Section from '../Section.svelte';
  import { useSettings } from '../settings.svelte';

  const settings = useSettings();
  const drawAnytime = settings.field('draw_anytime', true);
  const clipboard = settings.field('scribe_clipboard_monitor', true);
</script>

<Page title="Sessions">
  <Section>
    <Group>
      <Row kind="icon" label="Screen Recording" caption="Needed to capture screenshots. Voice and copied text work without it.">
        {#snippet trailing()}
          <Button size={28} onclick={() => openSystemPane('com.apple.preference.security?Privacy_ScreenCapture')}>Open System Settings…</Button>
        {/snippet}
      </Row>
    </Group>
  </Section>

  <Section title="Drawing">
    <Group>
      <Row kind="control" label="Draw anytime" caption="Hold the draw key outside a session to start one">
        {#snippet trailing()}<Switch label="Draw anytime" bind:checked={drawAnytime.value} />{/snippet}
      </Row>
      <Row kind="control" label="Shortcuts" caption="Session, screenshot and draw keys live in Keyboard">
        {#snippet trailing()}<Button size={28} onclick={() => navigate('keyboard')}>Open Keyboard…</Button>{/snippet}
      </Row>
    </Group>
  </Section>

  <Section title="Capture">
    <Group>
      <Row kind="control" label="Include text I copy">
        {#snippet trailing()}<Switch label="Include text I copy" bind:checked={clipboard.value} />{/snippet}
      </Row>
    </Group>
  </Section>
</Page>
