<script lang="ts">
  import { Group, RadioList, Row, Segmented, SwatchRow, ThumbnailPicker } from '@speaktype/ui';
  import Page from '../Page.svelte';
  import Section from '../Section.svelte';
  import { useSettings } from '../settings.svelte';
  import WidgetThumb from './WidgetThumb.svelte';
  import WindowThumb from './WindowThumb.svelte';

  const settings = useSettings();
  const theme = settings.field('theme', 'auto');
  const size = settings.field('widget_size', 'regular');
  const ink = settings.field('ink_color', 'orange');
  const thickness = settings.field('ink_thickness', 'regular');
  const visibility = settings.field('widget_visibility', 'always');

  const INK_NAMES = {
    orange: 'SpeakType orange',
    red: 'Red',
    amber: 'Amber',
    green: 'Green',
    blue: 'Blue',
    purple: 'Purple',
    white: 'White',
  } as const;
  type Ink = keyof typeof INK_NAMES;
  const swatches = (Object.keys(INK_NAMES) as Ink[]).map((name) => ({ value: `var(--st-ink-${name})`, name: INK_NAMES[name] }));
  const toSwatch = (name: string) => `var(--st-ink-${name})`;
  const fromSwatch = (swatch: string) => swatch.slice('var(--st-ink-'.length, -1) as Ink;
</script>

{#snippet auto()}<WindowThumb mode="auto" />{/snippet}
{#snippet light()}<WindowThumb mode="light" />{/snippet}
{#snippet dark()}<WindowThumb mode="dark" />{/snippet}
{#snippet small()}<WidgetThumb px={30} />{/snippet}
{#snippet regular()}<WidgetThumb px={40} />{/snippet}
{#snippet large()}<WidgetThumb px={50} />{/snippet}

<Page title="Appearance">
  <Section>
    <Group>
      <Row kind="control" label="Appearance" caption="Follows macOS unless you choose">
        {#snippet trailing()}
          <ThumbnailPicker
            label="Appearance"
            bind:value={theme.value}
            items={[
              { value: 'auto', label: 'Auto', preview: auto },
              { value: 'light', label: 'Light', preview: light },
              { value: 'dark', label: 'Dark', preview: dark },
            ]}
          />
        {/snippet}
      </Row>
      <Row kind="control" label="Widget size" caption="The floating mic, while idle">
        {#snippet trailing()}
          <ThumbnailPicker
            label="Widget size"
            bind:value={size.value}
            items={[
              { value: 'small', label: 'Small', preview: small },
              { value: 'regular', label: 'Regular', preview: regular },
              { value: 'large', label: 'Large', preview: large },
            ]}
          />
        {/snippet}
      </Row>
    </Group>
  </Section>

  <Section title="Ink">
    <Group>
      <Row kind="control" label="Colour" caption="Your first pen colour in a session">
        {#snippet trailing()}
          <SwatchRow label="Ink colour" colors={swatches} bind:value={() => toSwatch(ink.value), (swatch) => (ink.value = fromSwatch(swatch))} />
        {/snippet}
      </Row>
      <Row kind="control" label="Line thickness">
        {#snippet trailing()}
          <Segmented
            label="Line thickness"
            bind:value={thickness.value}
            items={[
              { value: 'thin', label: 'Thin' },
              { value: 'regular', label: 'Regular' },
              { value: 'bold', label: 'Bold' },
            ]}
          />
        {/snippet}
      </Row>
    </Group>
  </Section>

  <Section title="Widget">
    <Group>
      <Row kind="control" label="Show the widget">
        {#snippet trailing()}
          <RadioList
            label="Show the widget"
            bind:value={visibility.value}
            items={[
              { value: 'always', label: 'Always' },
              { value: 'active', label: 'Only while dictating or in a session' },
              { value: 'never', label: 'Never — menu bar only' },
            ]}
          />
        {/snippet}
      </Row>
    </Group>
  </Section>
</Page>
