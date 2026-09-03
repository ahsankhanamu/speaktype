<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getModels,
    getModelProgress,
    loadModel,
    queueModelDownload,
    restartModelDownload,
    cancelModelDownload,
    pauseModelDownload,
    saveSettings,
    getSettings,
    listenEvent,
  } from './ipc';
  import { formatByteSize, formatSpeedMbps } from './format';

  interface GridModel {
    id: string;
    downloaded?: boolean;
    downloaded_size?: string;
    size_bytes?: number;
    downloaded_bytes?: number;
    size?: string;
    desc?: string;
    partial?: boolean;
    partial_size?: number;
    expected_size?: number;
    partial_label?: string;
    [key: string]: unknown;
  }

  interface ProgressPayload {
    model?: string;
    phase?: string;
    message?: string;
    percent?: number;
    speed_mbps?: number;
    eta_secs?: number;
    total_bytes?: number;
    bytes_downloaded?: number;
    queue_position?: number;
    [key: string]: unknown;
  }

  interface Hooks {
    onReady?: (model: string, message: string) => void;
    onError?: (message: string) => void;
    onPaused?: (model: string) => void;
    onRefresh?: () => void;
    onStatus?: (type: string, message: string) => void;
    onScheduleHideStatus?: (ms: number) => void;
    onHideStatus?: () => void;
    onModelSaved?: (model: string) => void;
    onDownloadCount?: (count: number) => void;
    onLoading?: (text: string) => void;
  }

  interface CardPart {
    model: string;
    state: string;
    isDownloading: boolean;
    isRecommended: boolean;
    showSpinner: boolean;
    switchLabel: string;
    progress: Record<string, unknown>;
    className: string;
    waitPos: number;
    baseDesc: string;
    descText: string;
    livePct: number | null;
    showPartialLabel: boolean;
    partialLabel: string;
  }

  let {
    mode = 'settings',
    recommendedModel = null,
    getActiveModel = () => '',
    modelSelect = null,
    hooks = {},
    downloadCount = $bindable(0),
  }: {
    mode?: 'settings' | 'onboarding';
    recommendedModel?: string | null;
    getActiveModel?: () => string;
    modelSelect?: HTMLSelectElement | null;
    hooks?: Hooks;
    downloadCount?: number;
  } = $props();

  export const CLICKABLE_STATES = new Set(['downloaded', 'partial', 'remote']);

  const ICONS = {
    active: `<svg class="model-icon icon-active" viewBox="0 0 24 24" fill="none" stroke="#4ade80" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5"/></svg>`,
    downloading: `<svg class="model-icon icon-downloading" viewBox="0 0 24 24" fill="none" stroke="#facc15" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 8v8M8 12l4 4 4-4"/></svg>`,
    waiting: `<svg class="model-icon icon-waiting" viewBox="0 0 24 24" fill="none" stroke="#94a3b8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>`,
    partial: `<svg class="model-icon icon-partial" viewBox="0 0 24 24" fill="none" stroke="#f59e0b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>`,
    downloaded: `<svg class="model-icon icon-downloaded" viewBox="0 0 24 24" fill="none" stroke="#888" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3"/></svg>`,
    cloud: `<svg class="model-icon icon-cloud" viewBox="0 0 24 24" fill="none" stroke="#888" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.5 19H9a7 7 0 116.71-9h1.79a4.5 4.5 0 110 9z"/><path d="M12 13v4M10 15l2 2 2-2"/></svg>`,
  };

  let cardProgress = $state<Record<string, ProgressPayload>>({});
  let switchingModel = $state<string | null>(null);
  let switchMessage = $state('');
  let inlineSwitch = $state(false);
  let lastListKey = $state('');
  let pausePending = $state<Set<string>>(new Set());
  let forceResumeModels = $state<Set<string>>(new Set());
  let downloadingModels = $state<Set<string>>(new Set());
  let lastDownloadCount = $state<number | null>(null);

  let models = $state<GridModel[]>([]);
  let downloading: string[] = $state([]);
  let waiting: string[] = $state([]);

  let listenersAttached = false;
  let unlistenProgress: (() => void) | null = null;
  let unlistenQueue: (() => void) | null = null;
  let containerEl: HTMLDivElement | null = null;

  function baseModelDesc(m: GridModel): string {
    const sizeLabel =
      m.downloaded_size ||
      (m.size_bytes ? formatByteSize(m.size_bytes) : '') ||
      (m.downloaded_bytes ? formatByteSize(m.downloaded_bytes) : '') ||
      m.size ||
      '';
    const descLabel = m.desc || '';
    return `${sizeLabel}${sizeLabel && descLabel ? ' — ' : ''}${descLabel}`;
  }

  function listKey(models: GridModel[], downloading: string[], waiting: string[]): string {
    const modelPart = models
      .map((m) => [m.id, m.downloaded, m.partial, m.partial_size || 0].join(':'))
      .join('|');
    const forced = [...forceResumeModels].sort().join(',');
    return `${modelPart}::${downloading.join(',')}::${waiting.join(',')}::${getActiveModel()}::${forced}`;
  }

  function inFlightModels(downloading: string[], waiting: string[]): Set<string> {
    const inFlight = new Set(waiting);
    downloading.forEach((model) => {
      if (!forceResumeModels.has(model)) inFlight.add(model);
    });
    return inFlight;
  }

  function publishDownloadCount(count: number) {
    if (count === lastDownloadCount) return;
    lastDownloadCount = count;
    downloadCount = count;
    hooks.onDownloadCount?.(count);
  }

  export async function refreshDownloadCount() {
    try {
      const result: any = await getModelProgress();
      const dl = (result && result.downloading) || [];
      const w = (result && result.waiting) || [];
      publishDownloadCount(inFlightModels(dl, w).size);
    } catch (e) {
      console.error('Failed to read model download count:', e);
    }
  }

  function formatEta(eta: number): string {
    if (!eta || eta <= 0) return '';
    if (eta < 60) return eta + 's remaining';
    return Math.round(eta / 60) + 'm ' + (eta % 60) + 's remaining';
  }

  function cardParts(m: GridModel, waiting: string[]): CardPart {
    const activeModel = getActiveModel();
    const selectedModel = modelSelect?.value || activeModel;
    const isDownloaded = !!m.downloaded;
    const isPartial = !!(m.partial || forceResumeModels.has(m.id));
    const isActive = m.id === activeModel && isDownloaded;
    const liveDl = cardProgress[m.id];
    const isDownloading =
      (downloadingModels.has(m.id) || (liveDl && liveDl.phase === 'downloading')) &&
      !forceResumeModels.has(m.id);
    const isWaiting = waiting.includes(m.id);
    const isSelected = m.id === selectedModel && !isActive;
    const isRecommended = !!recommendedModel && m.id === recommendedModel;
    const isSwitching = !!switchingModel && switchingModel === m.id;
    const cardBusy = switchingModel && switchingModel !== m.id;
    const isDimmed = !isActive && !isDownloading && (isWaiting || !!cardBusy);

    let state: string;
    if (isActive) state = 'active';
    else if (isDownloading) state = 'downloading';
    else if (isWaiting) state = 'waiting';
    else if (isPartial) state = 'partial';
    else if (isDownloaded) state = 'downloaded';
    else state = 'remote';

    const waitPos = isWaiting ? waiting.indexOf(m.id) + 1 : 0;

    const baseDesc = baseModelDesc(m);
    const livePct = liveDl && liveDl.phase === 'downloading' ? Math.round(liveDl.percent || 0) : null;
    const descText = isDownloading && livePct != null
      ? `${baseDesc} — ${livePct}%`
      : `${baseDesc}${isPartial && m.partial_label && !isDownloading ? ' — ' + m.partial_label : ''}`;

    const showSpinner = isSwitching && inlineSwitch;
    const showPartialLabel = isPartial && !!m.partial_label && !isDownloading;

    const className =
      'model-card' +
      (isActive ? ' active' : '') +
      (isRecommended ? ' recommended' : '') +
      (isDownloading ? ' downloading' : '') +
      (isWaiting ? ' waiting' : '') +
      (isPartial && !isDownloading && !isWaiting ? ' partial' : '') +
      (isSelected ? ' selected' : '') +
      (isDimmed ? ' dimmed' : '') +
      (isSwitching ? ' switching' : '') +
      (CLICKABLE_STATES.has(state) ? ' clickable' : '');

    return {
      model: m.id,
      state,
      isDownloading,
      isRecommended,
      showSpinner,
      switchLabel: showSpinner ? switchMessage || `Loading ${m.id}...` : '',
      progress: (liveDl as Record<string, unknown>) || { percent: 0, speed_mbps: 0, eta_secs: 0 },
      className,
      waitPos,
      baseDesc,
      descText,
      livePct,
      showPartialLabel,
      partialLabel: m.partial_label || '',
    };
  }

  let cards = $derived(models.map((m) => cardParts(m, waiting)));

  function iconFor(state: string): string {
    if (state === 'active') return ICONS.active;
    if (state === 'downloading') return ICONS.downloading;
    if (state === 'waiting') return ICONS.waiting;
    if (state === 'partial') return ICONS.partial;
    if (state === 'downloaded') return ICONS.downloaded;
    return ICONS.cloud;
  }

  function clearCardProgress(model: string) {
    delete cardProgress[model];
  }

  async function pauseDownload(model: string) {
    if (pausePending.has(model)) return;
    pausePending.add(model);
    hooks.onHideStatus?.();
    try {
      const result: any = await pauseModelDownload(model);
      if (result && result.paused) {
        forceResumeModels.add(model);
        clearCardProgress(model);
        lastListKey = '';
        await refresh(true);
        return;
      }
    } catch (e) {
      console.error('Pause failed:', e);
    }
    pausePending.delete(model);
  }

  async function cancelQueuedDownload(model: string) {
    try {
      await cancelModelDownload(model);
    } catch (e) {
      console.error('Cancel failed:', e);
    }
    clearCardProgress(model);
    await refresh(true);
  }

  export function setModelSelect(model: string) {
    if (modelSelect) modelSelect.value = model;
  }

  async function queueDownload(model: string, { restart = false, activate = false } = {}) {
    await ensureListeners();
    hooks.onHideStatus?.();
    forceResumeModels.delete(model);
    pausePending.delete(model);
    try {
      const result: any = activate
        ? restart
          ? await restartModelDownload(model)
          : await loadModel(model)
        : await queueModelDownload(model, restart);
      if (result && result.status === 'complete') {
        if (activate) await activateModel(model);
        return result;
      }
      if (result && (result.status === 'queued' || result.status === 'downloading' || result.status === 'started')) {
        await refresh(true);
        return result;
      }
      if (result && result.status === 'busy') {
        hooks.onError?.(result.message || 'Another model operation is already in progress');
        return result;
      }
      if (result && result.status === 'error') {
        hooks.onStatus?.('error', result.message || 'Failed to queue download');
        hooks.onScheduleHideStatus?.(5000);
      }
      return result;
    } catch (e) {
      hooks.onStatus?.('error', 'Failed to queue download: ' + e);
      hooks.onScheduleHideStatus?.(5000);
      return null;
    }
  }

  function beginInlineSwitch(model: string, message: string) {
    inlineSwitch = true;
    switchMessage = message;
    setBusy(model, true);
  }

  function endInlineSwitch(previousModel: string) {
    setBusy(null, false);
    if (previousModel) setModelSelect(previousModel);
  }

  function setBusy(model: string | null, busy: boolean) {
    switchingModel = busy ? model : null;
    if (!busy) {
      switchMessage = '';
      inlineSwitch = false;
    }
  }

  async function activateModel(model: string) {
    if (mode === 'onboarding') {
      return queueDownload(model, { activate: true });
    }

    const previousModel =
      containerEl?.querySelector('.model-card[data-state="active"]')?.getAttribute('data-model') || '';
    setModelSelect(model);
    beginInlineSwitch(model, `Loading ${model}...`);

    const settings: any = await getSettings();
    if (!settings) {
      endInlineSwitch(previousModel);
      return;
    }
    settings.model = model;
    await saveSettings(settings);
    hooks.onModelSaved?.(model);
    await ensureListeners();
    try {
      const result: any = await loadModel(model);
      if (result && result.status === 'busy') {
        endInlineSwitch(previousModel);
        hooks.onStatus?.('error', result.message || 'Server busy');
        hooks.onScheduleHideStatus?.(4000);
      } else if (result && result.status === 'queued') {
        setBusy(null, false);
        await refresh(true);
      }
      return result;
    } catch (e) {
      endInlineSwitch(previousModel);
      hooks.onStatus?.('error', 'Failed to load model: ' + e);
      hooks.onScheduleHideStatus?.(5000);
      return null;
    }
  }

  async function handleProgress(p: ProgressPayload) {
    if (!p || !p.model) return;

    const model = p.model;
    const message = p.message || '';

    if (pausePending.has(model) && (p.phase === 'downloading' || p.phase === 'retrying')) {
      return;
    }

    if (p.phase === 'preparing' || p.phase === 'stopping' || p.phase === 'starting' || p.phase === 'loading') {
      const text = message || `Switching to ${model}...`;
      if (inlineSwitch && switchingModel === model) {
        switchMessage = text;
        return;
      }
      setBusy(model, true);
      hooks.onLoading?.(text);
      return;
    }

    if (p.phase === 'waiting') {
      hooks.onHideStatus?.();
      cardProgress[model] = { phase: 'waiting', queue_position: p.queue_position };
      await refresh(true);
      return;
    }

    if (p.phase === 'downloading' || p.phase === 'retrying') {
      hooks.onHideStatus?.();
      const pct = p.percent || (p.total_bytes && p.total_bytes > 0 ? (p.bytes_downloaded! / p.total_bytes) * 100 : 0);
      const card = containerEl?.querySelector(`.model-card[data-model="${CSS.escape(model)}"]`);
      const cardDownloading = card?.classList.contains('downloading');
      const cardWaiting = card?.classList.contains('waiting');
      if (!card || (!cardDownloading && !cardWaiting)) {
        cardProgress[model] = {
          phase: 'downloading',
          percent: pct,
          speed_mbps: p.speed_mbps || 0,
          eta_secs: p.eta_secs || 0,
          message: p.phase === 'retrying' ? p.message || 'Retrying…' : undefined,
        };
        lastListKey = '';
        await refresh(true);
        return;
      }
      cardProgress[model] = {
        phase: 'downloading',
        percent: pct,
        speed_mbps: p.speed_mbps || 0,
        eta_secs: p.eta_secs || 0,
        message: p.phase === 'retrying' ? p.message || 'Retrying…' : undefined,
      };
      return;
    }

    if (p.phase === 'paused') {
      hooks.onHideStatus?.();
      setBusy(null, false);
      pausePending.delete(model);
      clearCardProgress(model);
      forceResumeModels.add(model);
      lastListKey = '';
      await refresh(true);
      hooks.onPaused?.(model);
      return;
    }

    if (p.phase === 'done') {
      const wasInlineSwitch = inlineSwitch && switchingModel === model;
      setBusy(null, false);
      pausePending.delete(model);
      forceResumeModels.delete(model);
      clearCardProgress(model);
      lastListKey = '';
      if (mode === 'settings' && wasInlineSwitch) {
        hooks.onHideStatus?.();
      } else if (mode === 'settings') {
        hooks.onStatus?.('success', message || `${model} model ready`);
        hooks.onScheduleHideStatus?.(5000);
      }
      await refresh(true);
      hooks.onReady?.(model, message);
      return;
    }

    if (p.phase === 'cancelled') {
      hooks.onHideStatus?.();
      setBusy(null, false);
      pausePending.delete(model);
      forceResumeModels.delete(model);
      clearCardProgress(model);
      lastListKey = '';
      await refresh(true);
      return;
    }

    if (p.phase === 'error') {
      hooks.onHideStatus?.();
      setBusy(null, false);
      pausePending.delete(model);
      clearCardProgress(model);
      lastListKey = '';
      if (mode === 'settings') {
        hooks.onStatus?.('error', message || 'Model operation failed');
        hooks.onScheduleHideStatus?.(6000);
      }
      await refresh(true);
      hooks.onError?.(message || 'Model operation failed');
    }
  }

  export async function ensureListeners() {
    if (listenersAttached) return;
    listenersAttached = true;
    unlistenProgress = await listenEvent<ProgressPayload>('model:progress', (payload) => {
      handleProgress(typeof payload === 'string' ? JSON.parse(payload) : payload);
    });
    unlistenQueue = await listenEvent('model:queue', () => {
      refresh(true);
    });
  }

  function handleCardKeydown(e: KeyboardEvent) {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    const target = e.target as Element | null;
    const card = target?.closest('.model-card');
    if (!card || !target) return;
    const actionEl = target.closest('[data-action]');
    if (actionEl && card.contains(actionEl)) return;
    const model = (card as HTMLElement).dataset.model;
    if (!model) return;
    e.preventDefault();
    handleCardClick(e as unknown as MouseEvent);
  }

  function handleCardClick(e: MouseEvent) {
    const target = e.target as Element | null;
    const card = target?.closest('.model-card');
    if (!card || !target || !containerEl || !containerEl.contains(card)) return;

    const model = (card as HTMLElement).dataset.model;
    if (!model) return;

    const actionEl = target.closest('[data-action]');
    const action = actionEl && card.contains(actionEl) ? (actionEl as HTMLElement).dataset.action : null;

    if (action === 'pause') {
      e.stopPropagation();
      e.preventDefault();
      pauseDownload(model);
      return;
    }

    if (action === 'cancel') {
      e.stopPropagation();
      cancelQueuedDownload(model);
      return;
    }

    if (switchingModel) return;

    const state = (card as HTMLElement).dataset.state;
    if (state === 'active') return;

    if (action === 'resume') {
      e.stopPropagation();
      setModelSelect(model);
      queueDownload(model, { activate: true });
      return;
    }

    if (action === 'restart') {
      e.stopPropagation();
      setModelSelect(model);
      queueDownload(model, { restart: true, activate: true });
      return;
    }

    if (state === 'downloaded') {
      if (mode === 'onboarding') {
        setModelSelect(model);
        queueDownload(model, { activate: true });
      } else {
        activateModel(model);
      }
      return;
    }

    if (state === 'remote' && (action === 'download' || mode === 'onboarding')) {
      setModelSelect(model);
      queueDownload(model, mode === 'onboarding' ? { activate: true } : {});
    }
  }

  export async function refresh(force = false) {
    try {
      const result: any = await getModels();
      models = (result && result.available) || [];
      downloading = (result && result.downloading) || [];
      waiting = (result && result.waiting) || [];
      const serverProgress = (result && result.progress) || [];

      downloadingModels = new Set(downloading);
      publishDownloadCount(inFlightModels(downloading, waiting).size);

      for (const p of serverProgress) {
        if (p.model && (!cardProgress[p.model] || cardProgress[p.model].phase === 'downloading')) {
          cardProgress[p.model] = {
            ...cardProgress[p.model],
            phase: 'downloading',
            percent: p.percent || 0,
            speed_mbps: cardProgress[p.model]?.speed_mbps || 0,
            eta_secs: cardProgress[p.model]?.eta_secs || 0,
          };
        }
      }

      models.forEach((m) => {
        if (forceResumeModels.has(m.id) && m.partial && !downloading.includes(m.id)) {
          forceResumeModels.delete(m.id);
        }
      });

      lastListKey = listKey(models, downloading, waiting);
      hooks.onRefresh?.();
    } catch (e) {
      console.error('Failed to load model list:', e);
    }
  }

  onMount(() => {
    return () => {
      try {
        unlistenProgress?.();
      } catch { /* ignore */ }
      try {
        unlistenQueue?.();
      } catch { /* ignore */ }
      listenersAttached = false;
    };
  });
</script>

<div
  bind:this={containerEl}
  class="model-grid"
  class:busy={!!switchingModel}
  role="grid"
  aria-label="Available models"
  tabindex="-1"
  onclick={handleCardClick}
  onkeydown={handleCardKeydown}
>
  {#each cards as p (p.model)}
    <div class={p.className} data-model={p.model} data-state={p.state}>
      <div class="model-card-row">
        <div class="model-card-icon">{@html iconFor(p.state)}</div>
        <div class="model-card-body">
          <div class="model-card-name">
            <span>{p.model}</span>
            {#if p.isRecommended}<span class="model-tag-rec">recommended</span>{/if}
          </div>
          <div class="model-card-desc" data-base-desc={p.baseDesc}>
            {p.descText}
          </div>
        </div>
        <div class="model-card-action">
          {#if p.showSpinner}
            <span class="model-switch-spinner" role="status" aria-label={p.switchLabel} title={p.switchLabel}></span>
          {/if}
          {#if p.state === 'active'}
            <span class="model-badge badge-active">Active</span>
          {:else if p.state === 'downloading'}
            <span class="model-badge badge-downloading">Downloading</span>
          {:else if p.state === 'waiting'}
            <span class="model-badge badge-waiting">Waiting{p.waitPos > 0 ? ` #${p.waitPos}` : ''}</span>
            <button
              type="button"
              class="model-badge badge-cancel-queue card-cancel-btn"
              data-action="cancel"
            >Cancel</button>
          {:else if p.state === 'partial'}
            <span class="model-badge badge-partial clickable-badge" data-action="resume">Resume</span>
            <span class="model-badge badge-restart clickable-badge" data-action="restart">Restart</span>
          {:else if p.state === 'downloaded'}
            <span class="model-badge badge-load clickable-badge" data-action="load">{mode === 'onboarding' ? 'Use' : 'Load'}</span>
          {:else}
            <span class="model-badge badge-download clickable-badge" data-action="download">Download</span>
          {/if}
        </div>
      </div>
      {#if p.isDownloading}
        <div class="model-card-progress">
          <div class="card-progress-header">
            <span class="card-progress-label">Downloading</span>
            <span class="card-progress-pct">{Math.round(Number(p.progress.percent) || 0)}%</span>
          </div>
          <div class="progress-bar-track card-progress-track">
            <div
              class="progress-bar-fill card-progress-fill"
              style="width:{Math.round(Number(p.progress.percent) || 0)}%"
            ></div>
          </div>
          <div class="card-progress-meta">
            <span class="card-progress-speed">{p.progress.message || formatSpeedMbps(Number(p.progress.speed_mbps))}</span>
            <span class="card-progress-eta">{p.progress.message ? '' : formatEta(Number(p.progress.eta_secs))}</span>
            <button type="button" class="btn secondary btn-small card-pause-btn" data-action="pause">Pause</button>
          </div>
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  /* Shared model picker grid (Settings + onboarding) */

  .model-grid-host {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .model-grid-host.scrollable {
    flex: 1;
    overflow-y: auto;
    overscroll-behavior: contain;
    -webkit-overflow-scrolling: touch;
    padding-right: 2px;
  }

  .model-grid-host.scrollable::-webkit-scrollbar { width: 6px; }
  .model-grid-host.scrollable::-webkit-scrollbar-track { background: transparent; }
  .model-grid-host.scrollable::-webkit-scrollbar-thumb {
    background: var(--scrollbar);
    border-radius: 3px;
  }
  .model-grid-host.scrollable::-webkit-scrollbar-thumb:hover {
    background: var(--scrollbar-hover);
  }

  .model-grid {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .model-grid.busy :global(.model-card.dimmed) {
    opacity: 0.45;
  }

  .model-card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0;
    padding: 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    transition: background-color 140ms ease, border-color 140ms ease, opacity 140ms ease;
  }

  .model-card.active {
    border-color: var(--success);
    background: var(--success-soft-5);
  }

  .model-card.clickable {
    cursor: pointer;
  }

  .model-card.clickable:hover {
    background: var(--surface-3);
    border-color: var(--border-strong);
  }

  .model-card.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .model-card.partial {
    border-color: var(--warning-alt);
    background: var(--warning-soft);
  }

  .model-card.recommended:not(.active):not(.downloading) {
    border-color: var(--accent-border);
  }

  .model-card-row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
  }

  .model-card.downloading {
    border-color: var(--warning-download);
    background: var(--warning-download-soft);
    opacity: 1;
  }

  .model-card.waiting {
    border-color: var(--waiting);
    background: var(--waiting-soft);
    opacity: 0.85;
  }

  .model-card-progress {
    width: 100%;
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .card-progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }

  .card-progress-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--warning-download);
  }

  .card-progress-pct {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }

  .card-progress-track {
    margin-bottom: 6px;
  }

  .card-progress-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 11px;
    color: var(--text-dim);
  }

  .card-progress-meta .card-pause-btn {
    margin-left: auto;
    padding: 2px 8px;
    font-size: 11px;
  }

  .model-card-icon {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .model-icon {
    width: 24px;
    height: 24px;
  }

  .model-icon.icon-active { stroke: var(--success); }
  .model-icon.icon-downloading {
    stroke: var(--warning-download);
    animation: model-grid-pulse 1.5s ease-in-out infinite;
  }

  @keyframes model-grid-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .model-icon.icon-waiting { stroke: var(--waiting-icon); }
  .model-icon.icon-downloaded { stroke: var(--text-dim); }
  .model-icon.icon-cloud { stroke: var(--text-dim); }
  .model-icon.icon-partial { stroke: var(--warning-alt); }

  .model-card-body {
    flex: 1;
    min-width: 0;
  }

  .model-card-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
    margin-bottom: 2px;
  }

  .model-card-desc {
    font-size: 12px;
    color: var(--text-dim);
    line-height: 1.3;
  }

  .model-tag-rec {
    font-size: 10px;
    color: var(--accent-muted);
    font-weight: 600;
  }

  /* Reserved so Load -> Active -> spinner never nudges the trailing edge */
  .model-card-action {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    justify-content: flex-end;
    min-width: 84px;
  }

  .model-badge {
    display: inline-block;
    padding: 3px 10px;
    border-radius: 12px;
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
  }

  .badge-active {
    background: var(--success-soft);
    color: var(--success);
  }

  .badge-load {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .badge-download {
    background: rgba(168, 85, 247, 0.15);
    color: #a855f7;
  }

  .badge-downloading {
    background: var(--warning-download-soft);
    color: var(--warning-download);
  }

  .badge-waiting {
    background: rgba(100, 116, 139, 0.2);
    color: var(--waiting-icon);
  }

  .badge-cancel-queue {
    background: rgba(239, 68, 68, 0.12);
    color: #f87171;
    cursor: pointer;
    border: none;
  }

  .badge-cancel-queue:hover {
    filter: brightness(1.15);
  }

  .badge-partial {
    background: var(--warning-soft);
    color: var(--warning-alt);
  }

  .badge-restart {
    background: rgba(239, 68, 68, 0.12);
    color: #f87171;
  }

  .model-card.switching {
    opacity: 1;
  }

  .model-switch-spinner {
    flex-shrink: 0;
    width: 12px;
    height: 12px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    border-radius: var(--radius-pill);
    animation: model-switch-spin 0.7s linear infinite;
  }

  @keyframes model-switch-spin {
    to { transform: rotate(360deg); }
  }

  @media (prefers-reduced-motion: reduce) {
    .model-card { transition: none; }
    .model-icon.icon-downloading { animation: none; }
    .model-switch-spinner { animation: none; }
  }

  .model-card-action .model-badge + .model-badge {
    margin-left: 0;
  }

  .clickable-badge {
    cursor: pointer;
    transition: filter 0.15s;
  }

  .clickable-badge:hover {
    filter: brightness(1.3);
  }

  .progress-bar-track {
    width: 100%;
    height: 8px;
    background: var(--surface-4);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 4px;
    transition: width 0.2s ease;
    min-width: 2px;
  }

  :global(.model-grid-host .btn.secondary.btn-small) {
    border: 1px solid var(--border-dashed);
    background: var(--surface-3);
    color: var(--text-muted);
    border-radius: 6px;
    cursor: pointer;
    font-family: inherit;
  }

  :global(.model-grid-host .btn.secondary.btn-small:hover) {
    background: var(--surface-4);
  }
</style>
