<script lang="ts">
  import { Button, Group, Row, Switch, toast } from '@speaktype/ui';
  import { resetWidgetPosition } from '../../lib/ipc';
  import Page from '../Page.svelte';
  import { setLaunchAtLogin } from '../pending';
  import Section from '../Section.svelte';
  import { useSettings } from '../settings.svelte';

  const settings = useSettings();
  const launch = settings.field('launch_at_login', false);
  const sounds = settings.field('sounds', true);

  async function setLaunch(on: boolean) {
    try {
      await setLaunchAtLogin(on);
      launch.value = on;
    } catch (error) {
      toast.show({ message: `Couldn't change launch at login. ${(error as { message?: string })?.message ?? error}` });
    }
  }

  async function resetPosition() {
    await resetWidgetPosition();
    toast.show({ message: 'Widget moved back to its default spot' });
  }
</script>

<Page title="General">
  <Section>
    <Group>
      <Row kind="control" label="Launch at login" caption="Open SpeakType when you log in to your Mac">
        {#snippet trailing()}<Switch label="Launch at login" bind:checked={() => launch.value, setLaunch} />{/snippet}
      </Row>
      <Row kind="control" label="Sounds" caption="A soft sound when dictation starts and stops">
        {#snippet trailing()}<Switch label="Sounds" bind:checked={sounds.value} />{/snippet}
      </Row>
      <Row kind="control" label="Reset widget position" caption="Bring the widget back if it ends up off screen">
        {#snippet trailing()}<Button size={28} onclick={resetPosition}>Reset widget position</Button>{/snippet}
      </Row>
    </Group>
  </Section>
</Page>
