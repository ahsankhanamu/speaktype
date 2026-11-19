<script lang="ts">
  import { onMount, tick, type Component } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { Mic } from '@lucide/svelte';
  import { Alert, LayerRoot, MicGlyph, SearchField, Sidebar, SidebarGroup, SidebarItem, Toast } from '@speaktype/ui';
  import { liveQuery } from '../lib/data';
  import { applyTheme } from '../lib/theme';
  import About from './pages/About.svelte';
  import Advanced from './pages/Advanced.svelte';
  import Appearance from './pages/Appearance.svelte';
  import Dictation from './pages/Dictation.svelte';
  import General from './pages/General.svelte';
  import History from './pages/History.svelte';
  import Keyboard from './pages/Keyboard.svelte';
  import Permissions from './pages/Permissions.svelte';
  import Providers from './pages/Providers.svelte';
  import Sessions from './pages/Sessions.svelte';
  import Storage from './pages/Storage.svelte';
  import { GROUPS, PAGES, currentPage, navigate, startRouter, type PageId } from './router.svelte';
  import { filter, type SearchEntry } from './search';
  import { useSettings } from './settings.svelte';

  const VIEWS: Record<PageId, Component> = {
    history: History, general: General, keyboard: Keyboard, appearance: Appearance, dictation: Dictation, sessions: Sessions,
    providers: Providers, permissions: Permissions, storage: Storage, advanced: Advanced, about: About,
  };
  const HIGHLIGHT_MS = 2000;

  const settings = useSettings();
  const version = liveQuery('app-version', getVersion, { invalidateOn: [], refetchOnFocus: false });

  let query = $state('');
  let scroller = $state<HTMLElement | null>(null);
  const matches = $derived(filter(query));
  const View = $derived(VIEWS[currentPage.id]);

  onMount(startRouter);
  $effect(() => {
    if (settings.data) applyTheme(settings.data.theme);
  });

  async function reveal({ page, anchor }: SearchEntry) {
    query = '';
    navigate(page);
    await tick();
    const row = [...(scroller?.querySelectorAll<HTMLElement>('.st-row, h2') ?? [])].find(
      (candidate) => (candidate.querySelector('.label') ?? candidate).textContent === anchor,
    );
    row?.scrollIntoView({ block: 'center' });
    row?.setAttribute('data-match', '');
    setTimeout(() => row?.removeAttribute('data-match'), HIGHLIGHT_MS);
  }
</script>

<LayerRoot>
  <div class="window">
    <Sidebar label="SpeakType">
      <SearchField bind:value={query} />
      <button type="button" class="app" onclick={() => navigate('about')}>
        <span class="icon"><MicGlyph size={18} /></span>
        <span class="text">
          <strong>SpeakType</strong>
          <span>{version.data ? `Version ${version.data}` : ''}</span>
        </span>
      </button>
      {#if query.trim()}
        <SidebarGroup>
          {#each matches as hit (hit.page + hit.label)}
            {@const page = PAGES.find((p) => p.id === hit.page)}
            <SidebarItem
              label={`${page?.label ?? 'About'} › ${hit.label}`}
              icon={page?.icon ?? Mic}
              tileColor={page?.tile ?? '#8e8e93'}
              selected={false}
              onclick={() => reveal(hit)}
            />
          {:else}
            <p class="none">No settings match “{query.trim()}”.</p>
          {/each}
        </SidebarGroup>
      {:else}
        {#each GROUPS as group}
          <SidebarGroup>
            {#each group as item (item.id)}
              <SidebarItem
                label={item.label}
                icon={item.icon}
                tileColor={item.tile}
                selected={currentPage.id === item.id}
                onclick={() => navigate(item.id)}
              />
            {/each}
          </SidebarGroup>
        {/each}
      {/if}
    </Sidebar>
    <main bind:this={scroller}>
      {#key currentPage.id}<View />{/key}
    </main>
  </div>
  <Toast />
  <Alert />
</LayerRoot>

<style>
  /* Only the page scrolls. On macOS 26 the webview runs under the title bar, so a scrolled
     document would slide the sidebar beneath the traffic lights. */
  :global(html),
  :global(body) {
    height: 100%;
    /* clip, not hidden: focus and scrollIntoView can still scroll a hidden box. */
    overflow: clip;
  }
  :global(body) {
    margin: 0;
    -webkit-font-smoothing: antialiased;
    user-select: none;
  }
  :global(.st-row[data-match]) {
    background: color-mix(in srgb, var(--st-accent) 14%, transparent);
    border-radius: var(--st-radius-card);
  }
  .window {
    --page-top: 20px;
    display: flex;
    height: 100vh;
    background: var(--st-window);
    color: var(--st-text);
    font-family: var(--st-font-family);
  }
  /* macOS sidebar search is a compact 28 pt field. */
  .window :global(.st-sidebar .st-field) {
    height: var(--st-h-row-control);
    font-size: var(--st-font-info);
  }
  main {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
  }
  .app {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    border: 0;
    border-radius: var(--st-radius-selection);
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: default;
  }
  .app:hover {
    background: var(--st-hover);
  }
  .app:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .icon {
    display: grid;
    flex: none;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 7px;
    background: linear-gradient(180deg, #4a4a4e, #2c2c2e);
    color: #fff;
    box-shadow: 0 1px 2px var(--st-shadow);
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  strong {
    font-size: 13px;
  }
  .text span {
    color: var(--st-text-secondary);
    font-size: 11px;
  }
  .none {
    margin: 0;
    padding: 8px 10px;
    color: var(--st-text-secondary);
    font-size: var(--st-font-info);
  }
</style>
