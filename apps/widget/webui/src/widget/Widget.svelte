<script lang="ts">
  import { tick } from 'svelte';
  import { AudioLines, Check, EyeOff, History, Mic, Settings, X } from '@lucide/svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Button, DropdownMenu, Halo, Keycap, LayerRoot, MenuItem, MenuSeparator, Popover, Spinner, StatusDot, Tooltip, formatShortcut } from '@speaktype/ui';
  import MicPicker from '../lib/app/MicPicker.svelte';
  import ModelMenu from '../lib/app/ModelMenu.svelte';
  import { liveQuery, useEvent } from '../lib/data';
  import {
    commands,
    getOnboardingStatus,
    getSettings,
    hideWidget,
    openOnboarding,
    openSettings,
    saveWindowPosition,
    toggleRecording,
    cancelRecording,
  } from '../lib/ipc';
  import { useDictation, type Action } from './dictation.svelte';
  import Recording from './Recording.svelte';
  import { fitFor, fitWindow } from './windowFit';

  const SIZES = { small: 30, regular: 40, large: 50 } as const;
  const DRAG_PX = 5;
  const READY_MS = 3000;
  const CARD_MS = 10_000;
  const LOCATE_MS = 2500;

  const settings = liveQuery('settings', getSettings, { invalidateOn: ['settings-changed'] });
  const status = liveQuery('onboarding-status', getOnboardingStatus, {
    invalidateOn: ['sidecar:permissions', 'onboarding:opened', 'onboarding:finished', 'settings-changed'],
  });
  const dictation = useDictation();

  const size = $derived(SIZES[settings.data?.widget_size as keyof typeof SIZES] ?? SIZES.regular);
  const view = $derived(dictation.view);
  const unfinished = $derived(!!status.data && !(status.data.microphone && status.data.accessibility && status.data.model));
  const halo = $derived.by(() => {
    switch (view.kind) {
      case 'recording':
        return 'recording';
      case 'preparing':
        return 'active';
      case 'pasted':
        return 'success';
      case 'notice':
        return 'tone' in view ? view.tone : view.halo;
      default:
        return null;
    }
  });

  let root = $state<HTMLElement>();
  let inner = $state<HTMLElement>();
  let stage = $state<HTMLElement>();
  let menuOpen = $state(false);
  let panel = $state<'mic' | 'model' | null>(null);
  let opening = $state(false);
  const room = $derived(opening || menuOpen || panel !== null);

  let server = $state<{ tone: 'accent' | 'danger' | 'success'; label: string } | null>(null);
  let serverTimer: ReturnType<typeof setTimeout> | undefined;
  let card = $state<string | null>(null);
  let cardTimer: ReturnType<typeof setTimeout> | undefined;
  let locating = $state(false);
  let locateTimer: ReturnType<typeof setTimeout> | undefined;

  const dot = $derived(server ?? (unfinished ? { tone: 'warning' as const, label: 'Finish setup' } : null));

  useEvent('server:starting', () => {
    clearTimeout(serverTimer);
    server = { tone: 'accent', label: 'Server starting…' };
  });
  useEvent<{ reason?: string }>('server:error', ({ reason }) => {
    clearTimeout(serverTimer);
    server = { tone: 'danger', label: `Server error: ${reason ?? 'unknown'}` };
  });
  useEvent('server:ready', () => {
    clearTimeout(serverTimer);
    server = { tone: 'success', label: 'Server ready' };
    serverTimer = setTimeout(() => (server = null), READY_MS);
  });
  useEvent<{ hotkey: string }>('onboarding:finished', ({ hotkey }) => {
    card = `Click the mic or press ${formatShortcut(hotkey.split('+')).join('')}`;
    clearTimeout(cardTimer);
    cardTimer = setTimeout(() => (card = null), CARD_MS);
  });
  useEvent<{ duration_ms?: number }>('widget:highlight', ({ duration_ms }) => {
    locating = true;
    clearTimeout(locateTimer);
    locateTimer = setTimeout(() => (locating = false), duration_ms ?? LOCATE_MS);
  });
  $effect(() => () => {
    clearTimeout(serverTimer);
    clearTimeout(cardTimer);
    clearTimeout(locateTimer);
  });
  $effect(() => {
    if (view.kind === 'recording') card = null;
  });

  let content = $state({ width: 0, height: 0 });
  let hovering = $state(false);
  const fit = $derived(fitFor(content, { hovering: hovering && view.kind === 'idle', menuOpen: room }));

  function fitNow() {
    if (!stage) return Promise.resolve();
    return fitWindow(fit.width, fit.height, stage.offsetLeft + stage.offsetWidth / 2);
  }

  $effect(() => {
    if (!inner) return;
    const observer = new ResizeObserver(() => (content = { width: inner!.offsetWidth, height: inner!.offsetHeight }));
    observer.observe(inner);
    content = { width: inner.offsetWidth, height: inner.offsetHeight };
    return () => observer.disconnect();
  });

  $effect(() => {
    void fit;
    void tick().then(fitNow);
  });

  /** The window grows around the pill before a menu opens, so the menu has room and the pill stays put. */
  async function reveal(show: () => void) {
    opening = true;
    await tick();
    await fitNow();
    show();
    opening = false;
  }

  let press: { x: number; y: number } | undefined;
  let dragged = false;

  function endPress() {
    press = undefined;
    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerup', endPress);
  }

  function onPointerMove(event: PointerEvent) {
    if (!press || Math.hypot(event.screenX - press.x, event.screenY - press.y) < DRAG_PX) return;
    dragged = true;
    endPress();
    getCurrentWindow()
      .startDragging()
      .then(() => saveWindowPosition())
      .catch((error) => console.error('[widget] drag failed', error));
  }

  function onPointerDown(event: PointerEvent) {
    if (event.button !== 0 || view.kind === 'recording' || view.kind === 'transcribing') return;
    if ((event.target as Element).closest('[data-action]')) return;
    dragged = false;
    press = { x: event.screenX, y: event.screenY };
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', endPress);
  }

  function onContextMenu(event: MouseEvent) {
    event.preventDefault();
    void reveal(() => (menuOpen = true));
  }

  const report = (error: unknown) => console.error('[widget]', error);

  function mic() {
    if (dragged) return;
    (unfinished ? openOnboarding() : toggleRecording(false)).catch(report);
  }

  const actions: Record<Action, { label: string; run: () => void }> = {
    pasteAgain: {
      label: 'Paste again',
      run: () => {
        commands.pasteLast().catch(report);
        dictation.dismiss();
      },
    },
    tryAgain: { label: 'Try again', run: () => toggleRecording(false).catch(report) },
    chooseMic: { label: 'Choose mic', run: () => void reveal(() => (panel = 'mic')) },
  };
</script>

<LayerRoot>
  <div
    class="widget"
    style:--size="{size}px"
    style:--k={size / 40}
    style:width="{fit.width}px"
    style:height="{fit.height}px"
    bind:this={root}
  >
    <div class="content" bind:this={inner}>
    {#if card !== null}
      <div class="card">
        <div class="card-text">
          <p class="card-title">You're all set</p>
          <p class="card-hint">{card}</p>
        </div>
        <button type="button" class="card-close" aria-label="Dismiss" onclick={() => (card = null)}>
          <X size={12} strokeWidth={2.2} aria-hidden="true" />
        </button>
      </div>
    {/if}

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="stage"
      class:locating
      bind:this={stage}
      onpointerdown={onPointerDown}
      oncontextmenu={onContextMenu}
      onpointerenter={() => (hovering = true)}
      onpointerleave={() => (hovering = false)}
    >
      <Halo tone={halo}>
        <div class="pill {view.kind}" class:haloed={halo !== null} class:round={view.kind === 'idle' || view.kind === 'pasted'}>
          {#if view.kind === 'idle'}
            <Tooltip label="Dictate" shortcut={(settings.data?.shortcuts?.dictation ?? 'Super+Control').split('+')}>
              {#snippet children(tooltipProps)}
                <button type="button" {...tooltipProps} class="mic" aria-label="Dictate" onclick={mic}>
                  <Mic size={18} strokeWidth={2} aria-hidden="true" />
                </button>
              {/snippet}
            </Tooltip>
            {#if dot}
              <span class="dot"><StatusDot tone={dot.tone} label={dot.label} /></span>
            {/if}
          {:else if view.kind === 'recording'}
            <Recording onstop={() => toggleRecording(true).catch(report)} oncancel={() => cancelRecording().catch(report)} />
          {:else if view.kind === 'transcribing'}
            <span class="dots" role="status" aria-label="Transcribing"><i></i><i></i><i></i></span>
          {:else if view.kind === 'preparing'}
            <span class="spinner"><Spinner /></span>
            <span class="text">Getting ready…</span>
          {:else if view.kind === 'pasted'}
            <Check size={18} strokeWidth={2.2} color="var(--st-success)" role="img" aria-label="Pasted" />
          {:else}
            {#if view.dot}<span class="amber"></span>{/if}
            {#if view.copiedKeys}
              <span class="text">Copied. Press <Keycap key="⌘" /><Keycap key="V" /> to paste</span>
            {:else}
              <span class="text">{view.message}</span>
            {/if}
            {#if view.action}
              {@const action = actions[view.action]}
              <Button size={28} class="pill-action" data-action onclick={action.run}>{action.label}</Button>
            {/if}
          {/if}
        </div>
      </Halo>

      <DropdownMenu.Root bind:open={menuOpen}>
        <DropdownMenu.Trigger>
          {#snippet child({ props })}<span {...props} class="anchor" aria-hidden="true"></span>{/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content>
          <MenuItem label="Microphone" onSelect={() => (panel = 'mic')}>
            {#snippet icon()}<Mic size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
          </MenuItem>
          <MenuItem label="Speech model" onSelect={() => (panel = 'model')}>
            {#snippet icon()}<AudioLines size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
          </MenuItem>
          <MenuSeparator />
          <MenuItem label="History" onSelect={() => commands.openSettings('history').catch(report)}>
            {#snippet icon()}<History size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
          </MenuItem>
          <MenuItem label="Settings" onSelect={() => openSettings().catch(report)}>
            {#snippet icon()}<Settings size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
          </MenuItem>
          <MenuSeparator />
          <MenuItem label="Hide widget" onSelect={() => hideWidget().catch(report)}>
            {#snippet icon()}<EyeOff size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
          </MenuItem>
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <Popover bind:open={() => panel !== null, (open) => !open && (panel = null)}>
        {#snippet trigger(props)}<span {...props} class="anchor" aria-hidden="true"></span>{/snippet}
        <div class="popover-body">
          {#if panel === 'mic'}
            <MicPicker />
          {:else if panel === 'model'}
            <ModelMenu trigger="row" onopenproviders={() => openSettings().catch(report)} />
          {/if}
        </div>
      </Popover>
    </div>
    </div>
  </div>
</LayerRoot>

<style>
  :global(html),
  :global(body) {
    overflow: hidden;
    background: transparent;
    user-select: none;
    -webkit-user-select: none;
  }

  .widget {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    color: rgba(255, 255, 255, 0.88);
    font-family: var(--st-font-family);
    font-size: 12px;
  }
  .content {
    display: flex;
    flex: none;
    align-items: center;
    gap: 12px;
  }

  .stage {
    position: relative;
    display: inline-flex;
    flex: none;
  }
  .anchor {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .stage.locating::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 999px;
    box-shadow: 0 0 0 1.5px var(--st-accent);
    animation: locate 0.9s var(--st-ease-out) 3;
    pointer-events: none;
  }
  @keyframes locate {
    from {
      opacity: 1;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(1.5);
    }
  }

  .pill {
    position: relative;
    display: flex;
    align-items: center;
    gap: calc(10px * var(--k));
    box-sizing: border-box;
    height: var(--size);
    padding: 0 calc(16px * var(--k));
    border-radius: 999px;
    background: rgba(40, 40, 42, 0.96);
    box-shadow:
      0 0 0 0.5px rgba(255, 255, 255, 0.14),
      0 0 0 1px rgba(0, 0, 0, 0.25),
      0 6px 18px rgba(0, 0, 0, 0.25);
    white-space: nowrap;
  }
  .pill.haloed {
    box-shadow: none;
  }
  .pill.round {
    justify-content: center;
    width: calc(44px * var(--k));
    height: calc(44px * var(--k));
    padding: 0;
  }
  .pill.recording {
    padding: 0 calc(7px * var(--k)) 0 calc(8px * var(--k));
  }
  .pill.preparing {
    gap: calc(8px * var(--k));
  }
  .pill.notice {
    gap: calc(8px * var(--k));
    padding: 0 calc(16px * var(--k));
  }
  .pill.notice:has(:global(.pill-action)) {
    gap: calc(10px * var(--k));
    padding: 0 calc(7px * var(--k)) 0 calc(14px * var(--k));
  }
  .pill.transcribing {
    gap: calc(6px * var(--k));
    padding: 0 calc(18px * var(--k));
  }

  .mic {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: #fff;
    cursor: default;
  }
  .mic:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }
  .dot {
    position: absolute;
    top: 0;
    right: 0;
    display: flex;
    padding: 2px;
    border-radius: 50%;
    background: rgba(40, 40, 42, 1);
  }

  .text {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: rgba(255, 255, 255, 0.88);
    --keycap-size: 18px;
  }
  .spinner {
    display: flex;
    color: #64b5ff;
  }
  .amber {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--st-warning);
  }
  .pill :global(.pill-action) {
    height: calc(26px * var(--k));
    padding: 0 10px;
    border-radius: 999px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.9);
  }

  .dots {
    display: inline-flex;
    gap: calc(6px * var(--k));
  }
  .dots i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #fff;
    animation: pulse 1.2s ease-in-out infinite;
  }
  .dots i:nth-child(2) {
    animation-delay: 0.2s;
  }
  .dots i:nth-child(3) {
    animation-delay: 0.4s;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 0.25;
    }
    40% {
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .dots i {
      animation: none;
    }
    .dots i:nth-child(2) {
      opacity: 0.55;
    }
    .dots i:nth-child(3) {
      opacity: 0.25;
    }
  }

  .card {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    box-sizing: border-box;
    padding: 8px 8px 8px 14px;
    border-radius: 14px;
    background: rgba(40, 40, 42, 0.96);
    box-shadow:
      0 0 0 0.5px rgba(255, 255, 255, 0.14),
      0 0 0 1px rgba(0, 0, 0, 0.25),
      0 6px 18px rgba(0, 0, 0, 0.25);
  }
  .card-text p {
    margin: 0;
  }
  .card-title {
    font-size: 13px;
    font-weight: 600;
  }
  .card-hint {
    margin-top: 2px !important;
    color: rgba(255, 255, 255, 0.6);
    font-size: 12px;
  }
  .card-close {
    display: grid;
    flex: none;
    place-items: center;
    align-self: flex-start;
    width: 20px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: rgba(255, 255, 255, 0.55);
    cursor: default;
  }
  .card-close:hover {
    background: var(--st-fill);
    color: #fff;
  }
  .card-close:focus-visible {
    outline: none;
    box-shadow: var(--st-focus-ring);
  }

  .popover-body {
    width: 300px;
  }
</style>
