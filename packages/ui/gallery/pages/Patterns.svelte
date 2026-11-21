<script lang="ts">
  import { Clock, Cloud, Lock, Mic, Moon, Pencil, Settings } from '@lucide/svelte';
  import {
    Button,
    EmptyState,
    Group,
    Row,
    Segmented,
    Sidebar,
    SidebarGroup,
    SidebarItem,
    Skeleton,
    Switch,
    SwatchRow,
    ThumbnailPicker,
    StatusDot,
  } from '@speaktype/ui';

  let section = $state('appearance');
  let theme = $state('auto');
  let color = $state('#ef643c');
  let thickness = $state('regular');
  let launch = $state(true);

  const nav = [
    [{ id: 'history', label: 'History', icon: Clock, tile: '#30b0c7' }],
    [
      { id: 'general', label: 'General', icon: Settings, tile: '#8e8e93' },
      { id: 'appearance', label: 'Appearance', icon: Moon, tile: '#1c1c1e' },
    ],
    [
      { id: 'dictation', label: 'Dictation', icon: Mic, tile: '#0a84ff' },
      { id: 'sessions', label: 'Sessions', icon: Pencil, tile: '#ff6a3d' },
      { id: 'providers', label: 'Providers', icon: Cloud, tile: '#bf5af2' },
    ],
    [{ id: 'permissions', label: 'Permissions', icon: Lock, tile: '#5e5ce6' }],
  ];
  const swatches = [
    { value: '#ef643c', name: 'SpeakType orange' },
    { value: '#ff453a', name: 'Red' },
    { value: '#ff9f0a', name: 'Amber' },
    { value: '#30d158', name: 'Green' },
    { value: '#0a84ff', name: 'Blue' },
    { value: '#bf5af2', name: 'Purple' },
    { value: '#ffffff', name: 'White' },
  ];
</script>

<div class="window">
  <Sidebar label="SpeakType">
    {#each nav as group}
      <SidebarGroup>
        {#each group as item}
          <SidebarItem label={item.label} icon={item.icon} tileColor={item.tile} selected={section === item.id} onclick={() => (section = item.id)} />
        {/each}
      </SidebarGroup>
    {/each}
  </Sidebar>
  <main>
    <h1>Appearance</h1>
    <Group>
      <Row kind="control" label="Appearance" caption="Follows macOS unless you choose">
        {#snippet trailing()}
          <ThumbnailPicker
            label="Appearance"
            bind:value={theme}
            items={[
              { value: 'auto', label: 'Auto', preview: auto },
              { value: 'light', label: 'Light', preview: light },
              { value: 'dark', label: 'Dark', preview: dark },
            ]}
          />
        {/snippet}
      </Row>
      <Row kind="control" label="Colour" caption="Your first pen colour in a session">
        {#snippet trailing()}<SwatchRow label="Colour" colors={swatches} bind:value={color} />{/snippet}
      </Row>
      <Row kind="control" label="Line thickness">
        {#snippet trailing()}
          <Segmented
            label="Line thickness"
            bind:value={thickness}
            items={[
              { value: 'thin', label: 'Thin' },
              { value: 'regular', label: 'Regular' },
              { value: 'bold', label: 'Bold' },
            ]}
          />
        {/snippet}
      </Row>
      <Row kind="control" label="Open at login">
        {#snippet trailing()}<Switch bind:checked={launch} label="Open at login" />{/snippet}
      </Row>
    </Group>
    <h2>Models</h2>
    <Group>
      <Row kind="icon" label="Balanced" caption="Small · best for most people · ~500 MB" icon={Mic} tileColor="#0a84ff">
        {#snippet trailing()}<StatusDot tone="success" label="In use" /><span class="meta">In use</span>{/snippet}
      </Row>
      <Row kind="icon" label="Medium" caption="Medium · good on Macs with 16 GB or more · ~1.5 GB" icon={Mic} tileColor="#bf5af2">
        {#snippet trailing()}<Button size={28}>Download</Button>{/snippet}
      </Row>
      <Row kind="info" label="Storage used">
        {#snippet trailing()}<span class="meta">650 MB</span>{/snippet}
      </Row>
      <Row kind="info" label="Location">
        {#snippet trailing()}<span class="meta">~/Library/Application Support/SpeakType</span>{/snippet}
      </Row>
    </Group>
    <div class="empty">
      <Group>
        <EmptyState title="No sessions yet" message="Hold the draw key while you talk to start one." icon={Pencil}>
          {#snippet actions()}<Button variant="primary">Start a session</Button>{/snippet}
        </EmptyState>
      </Group>
      <Group>
        <div class="skeleton"><Skeleton height="16px" width="40%" /><Skeleton lines={3} /></div>
      </Group>
    </div>
  </main>
</div>

{#snippet auto()}<span class="shot split"></span>{/snippet}
{#snippet light()}<span class="shot light"></span>{/snippet}
{#snippet dark()}<span class="shot dark"></span>{/snippet}

<style>
  .window {
    display: flex;
    width: 1040px;
    min-height: 920px;
    background: var(--st-window);
  }
  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
    padding: 44px 28px 24px;
  }
  h1 {
    margin: 0 0 0 2px;
    font-size: 20px;
    font-weight: 600;
  }
  h2 {
    margin: 12px 0 -6px 2px;
    font-size: var(--st-font-section);
    font-weight: 600;
  }
  .meta {
    color: var(--st-text-secondary);
    font-size: var(--st-font-caption);
    white-space: nowrap;
  }
  .shot {
    display: block;
    width: 100%;
    height: 100%;
  }
  .light {
    background: #f2f2f4;
  }
  .dark {
    background: #1c1c1e;
  }
  .split {
    background: linear-gradient(90deg, #f2f2f4 50%, #1c1c1e 50%);
  }
  .empty {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .skeleton {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 24px;
  }
</style>
