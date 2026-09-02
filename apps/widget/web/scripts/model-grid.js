/**
 * Shared model download/load grid for Settings and onboarding.
 */
class ModelGrid {
  constructor(options) {
    this.container =
      typeof options.container === 'string'
        ? document.getElementById(options.container)
        : options.container;
    this.ipc = options.ipc || window.ttipc;
    this.mode = options.mode || 'settings';
    this.recommendedModel = options.recommendedModel || null;
    this.modelSelect = options.modelSelect || null;
    this.getActiveModel = options.getActiveModel || (() => '');
    this.hooks = options.hooks || {};
    this.cardProgress = {};
    this.switchingModel = null;
    this.switchMessage = '';
    this.inlineSwitch = false;
    this.lastModels = [];
    this.lastDownloading = [];
    this.lastWaiting = [];
    this.listenersAttached = false;
    this.unlistenProgress = null;
    this.unlistenQueue = null;
    this.lastListKey = '';
    this.pausePending = new Set();
    this.forceResumeModels = new Set();
    this.downloadingModels = new Set();
    this.containerBound = false;
    this.lastDownloadCount = null;
    this.ensureContainerListener();
  }

  static CLICKABLE_STATES = new Set(['downloaded', 'partial', 'remote']);

  static ICONS = {
    active: `<svg class="model-icon icon-active" viewBox="0 0 24 24" fill="none" stroke="#4ade80" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5"/></svg>`,
    downloading: `<svg class="model-icon icon-downloading" viewBox="0 0 24 24" fill="none" stroke="#facc15" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 8v8M8 12l4 4 4-4"/></svg>`,
    waiting: `<svg class="model-icon icon-waiting" viewBox="0 0 24 24" fill="none" stroke="#94a3b8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>`,
    partial: `<svg class="model-icon icon-partial" viewBox="0 0 24 24" fill="none" stroke="#f59e0b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>`,
    downloaded: `<svg class="model-icon icon-downloaded" viewBox="0 0 24 24" fill="none" stroke="#888" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3"/></svg>`,
    cloud: `<svg class="model-icon icon-cloud" viewBox="0 0 24 24" fill="none" stroke="#888" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.5 19H9a7 7 0 116.71-9h1.79a4.5 4.5 0 110 9z"/><path d="M12 13v4M10 15l2 2 2-2"/></svg>`,
  };

  baseModelDesc(m) {
    const sizeLabel = m.downloaded_size
      || (m.size_bytes ? formatByteSize(m.size_bytes) : '')
      || (m.downloaded_bytes ? formatByteSize(m.downloaded_bytes) : '')
      || m.size
      || '';
    const descLabel = m.desc || '';
    return `${sizeLabel}${sizeLabel && descLabel ? ' — ' : ''}${descLabel}`;
  }

  listKey(models, downloading, waiting) {
    const modelPart = models
      .map(m => [m.id, m.downloaded, m.partial, m.partial_size || 0].join(':'))
      .join('|');
    const forced = [...this.forceResumeModels].sort().join(',');
    return `${modelPart}::${downloading.join(',')}::${waiting.join(',')}::${this.getActiveModel()}::${forced}`;
  }

  inFlightModels(downloading, waiting) {
    const inFlight = new Set(waiting);
    downloading.forEach(model => {
      if (!this.forceResumeModels.has(model)) inFlight.add(model);
    });
    return inFlight;
  }

  publishDownloadCount(count) {
    if (count === this.lastDownloadCount) return;
    this.lastDownloadCount = count;
    this.hooks.onDownloadCount?.(count);
  }

  // Count-only read for callers that need the in-flight total before the grid
  // has ever been rendered; get_model_progress skips the model catalog.
  async refreshDownloadCount() {
    try {
      const result = await this.ipc.getModelProgress();
      const downloading = (result && result.downloading) || [];
      const waiting = (result && result.waiting) || [];
      this.publishDownloadCount(this.inFlightModels(downloading, waiting).size);
    } catch (e) {
      console.error('Failed to read model download count:', e);
    }
  }

  formatEta(eta) {
    if (!eta || eta <= 0) return '';
    if (eta < 60) return eta + 's remaining';
    return Math.round(eta / 60) + 'm ' + (eta % 60) + 's remaining';
  }

  cardProgressHtml(model, p) {
    const pct = Math.round(p.percent || 0);
    const speed = formatSpeedMbps(p.speed_mbps);
    const eta = this.formatEta(p.eta_secs);
    return `
      <div class="model-card-progress">
        <div class="card-progress-header">
          <span class="card-progress-label">Downloading</span>
          <span class="card-progress-pct">${pct}%</span>
        </div>
        <div class="progress-bar-track card-progress-track">
          <div class="progress-bar-fill card-progress-fill" style="width:${pct}%"></div>
        </div>
        <div class="card-progress-meta">
          <span class="card-progress-speed">${speed}</span>
          <span class="card-progress-eta">${eta}</span>
          <button type="button" class="btn secondary btn-small card-pause-btn" data-action="pause" data-model="${model}">Pause</button>
        </div>
      </div>`;
  }

  ensureCardDownloadingState(card, p) {
    if (!card.classList.contains('downloading')) {
      card.classList.add('downloading');
      card.classList.remove('partial', 'waiting', 'dimmed');
      card.dataset.state = 'downloading';

      const iconWrap = card.querySelector('.model-card-icon');
      if (iconWrap) iconWrap.innerHTML = ModelGrid.ICONS.downloading;

      const actionWrap = card.querySelector('.model-card-action');
      if (actionWrap) {
        actionWrap.innerHTML = `<span class="model-badge badge-downloading">Downloading</span>`;
      }
    }

    const pct = Math.round(p.percent || 0);
    const descEl = card.querySelector('.model-card-desc');
    if (descEl && pct >= 0) {
      const base = descEl.dataset.baseDesc || descEl.textContent.split(' — Incomplete')[0].split(/ — \d+%$/)[0];
      if (!descEl.dataset.baseDesc) descEl.dataset.baseDesc = base;
      descEl.textContent = `${descEl.dataset.baseDesc} — ${pct}%`;
    }
  }

  setBusy(model, busy) {
    this.switchingModel = busy ? model : null;
    if (!busy) {
      this.switchMessage = '';
      this.inlineSwitch = false;
    }
    if (!this.container) return;
    this.container.classList.toggle('busy', busy);
    this.container.querySelectorAll('.model-card').forEach(card => {
      const isTarget = card.dataset.model === model;
      card.classList.toggle('switching', busy && isTarget);
      card.classList.toggle('dimmed', busy && !isTarget);
    });
  }

  findCard(model) {
    return this.container?.querySelector(`.model-card[data-model="${CSS.escape(model)}"]`);
  }

  updateCardProgress(model, p) {
    if (this.pausePending.has(model) && p.phase === 'downloading') return;

    this.cardProgress[model] = { ...this.cardProgress[model], ...p };
    const card = this.findCard(model);
    if (!card) return;

    const live = this.cardProgress[model];
    this.ensureCardDownloadingState(card, live);

    let progressEl = card.querySelector('.model-card-progress');
    if (!progressEl) {
      card.insertAdjacentHTML('beforeend', this.cardProgressHtml(model, live));
      return;
    }

    this.writeCardProgress(progressEl, live);
  }

  writeCardProgress(progressEl, p) {
    const pct = Math.round(p.percent || 0);
    const pctEl = progressEl.querySelector('.card-progress-pct');
    const fillEl = progressEl.querySelector('.card-progress-fill');
    const speedEl = progressEl.querySelector('.card-progress-speed');
    const etaEl = progressEl.querySelector('.card-progress-eta');
    if (pctEl) pctEl.textContent = `${pct}%`;
    if (fillEl) fillEl.style.width = `${pct}%`;
    if (speedEl) {
      speedEl.textContent = p.message
        ? p.message
        : formatSpeedMbps(p.speed_mbps);
    }
    if (etaEl) etaEl.textContent = p.message ? '' : this.formatEta(p.eta_secs);
  }

  clearCardProgress(model) {
    delete this.cardProgress[model];
    this.findCard(model)?.querySelector('.model-card-progress')?.remove();
  }

  async pauseDownload(model) {
    if (this.pausePending.has(model)) return;
    this.pausePending.add(model);
    this.hooks.onHideStatus?.();
    try {
      const result = await this.ipc.pauseModelDownload(model);
      if (result && result.paused) {
        this.forceResumeModels.add(model);
        this.clearCardProgress(model);
        // Keep pausePending until the backend confirms phase=paused, but invalidate
        // the list cache so the next refresh can show Resume.
        this.lastListKey = '';
        await this.refresh(true);
        return;
      }
    } catch (e) {
      console.error('Pause failed:', e);
    }
    this.pausePending.delete(model);
  }

  async cancelQueuedDownload(model) {
    try {
      await this.ipc.cancelModelDownload(model);
    } catch (e) {
      console.error('Cancel failed:', e);
    }
    this.clearCardProgress(model);
    await this.refresh(true);
  }

  setModelSelect(model) {
    if (this.modelSelect) this.modelSelect.value = model;
  }

  async queueDownload(model, { restart = false, activate = false } = {}) {
    await this.ensureListeners();
    this.hooks.onHideStatus?.();
    this.forceResumeModels.delete(model);
    this.pausePending.delete(model);
    try {
      const result = activate
        ? (restart ? await this.ipc.restartModelDownload(model) : await this.ipc.loadModel(model))
        : await this.ipc.queueModelDownload(model, restart);
      if (result && result.status === 'complete') {
        if (activate) await this.activate(model);
        return result;
      }
      if (result && (result.status === 'queued' || result.status === 'downloading' || result.status === 'started')) {
        await this.refresh(true);
        return result;
      }
      if (result && result.status === 'busy') {
        this.hooks.onError?.(result.message || 'Another model operation is already in progress');
        return result;
      }
      if (result && result.status === 'error') {
        this.hooks.onStatus?.('error', result.message || 'Failed to queue download');
        this.hooks.onScheduleHideStatus?.(5000);
      }
      return result;
    } catch (e) {
      this.hooks.onStatus?.('error', 'Failed to queue download: ' + e);
      this.hooks.onScheduleHideStatus?.(5000);
      return null;
    }
  }

  // Paints the new selection from the cached list before any IPC, so the click
  // lands on the same frame instead of after the sidecar restart round-trip.
  beginInlineSwitch(model, message) {
    this.inlineSwitch = true;
    this.switchMessage = message;
    this.setBusy(model, true);
    this.renderCards();
  }

  endInlineSwitch(previousModel) {
    this.setBusy(null, false);
    if (previousModel) this.setModelSelect(previousModel);
    this.renderCards();
  }

  async activate(model) {
    if (this.mode === 'onboarding') {
      return this.queueDownload(model, { activate: true });
    }

    // Read the rendered active card rather than the select, which callers may
    // already have pointed at the incoming model.
    const previousModel = this.container?.querySelector('.model-card[data-state="active"]')?.dataset.model || '';
    this.setModelSelect(model);
    this.beginInlineSwitch(model, `Loading ${model}...`);

    const settings = await this.ipc.getSettings();
    if (!settings) {
      this.endInlineSwitch(previousModel);
      return;
    }
    settings.model = model;
    await this.ipc.saveSettings(settings);
    this.hooks.onModelSaved?.(model);
    await this.ensureListeners();
    try {
      const result = await this.ipc.loadModel(model);
      if (result && result.status === 'busy') {
        this.endInlineSwitch(previousModel);
        this.hooks.onStatus?.('error', result.message || 'Server busy');
        this.hooks.onScheduleHideStatus?.(4000);
      } else if (result && result.status === 'queued') {
        this.setBusy(null, false);
        await this.refresh(true);
      }
      return result;
    } catch (e) {
      this.endInlineSwitch(previousModel);
      this.hooks.onStatus?.('error', 'Failed to load model: ' + e);
      this.hooks.onScheduleHideStatus?.(5000);
      return null;
    }
  }

  async handleProgress(p) {
    if (!p || !p.model) return;

    const model = p.model;
    const message = p.message || '';

    if (this.pausePending.has(model) && (p.phase === 'downloading' || p.phase === 'retrying')) {
      return;
    }

    if (p.phase === 'preparing' || p.phase === 'stopping' || p.phase === 'starting' || p.phase === 'loading') {
      const text = message || `Switching to ${model}...`;
      if (this.inlineSwitch && this.switchingModel === model) {
        this.switchMessage = text;
        this.applySwitchLabel(this.findCard(model), text);
        return;
      }
      this.setBusy(model, true);
      this.hooks.onLoading?.(text);
      return;
    }

    if (p.phase === 'waiting') {
      this.hooks.onHideStatus?.();
      this.cardProgress[model] = { phase: 'waiting', queue_position: p.queue_position };
      await this.refresh(true);
      return;
    }

    if (p.phase === 'downloading' || p.phase === 'retrying') {
      this.hooks.onHideStatus?.();
      const pct = p.percent || (p.total_bytes > 0 ? (p.bytes_downloaded / p.total_bytes) * 100 : 0);
      // If the card still looks idle (common on onboarding before a full refresh),
      // rebuild once so Pause/progress chrome appears.
      const card = this.findCard(model);
      if (!card || (!card.classList.contains('downloading') && !card.classList.contains('waiting'))) {
        this.cardProgress[model] = {
          phase: 'downloading',
          percent: pct,
          speed_mbps: p.speed_mbps || 0,
          eta_secs: p.eta_secs || 0,
          message: p.phase === 'retrying' ? (p.message || 'Retrying…') : undefined,
        };
        this.lastListKey = '';
        await this.refresh(true);
        return;
      }
      this.updateCardProgress(model, {
        phase: 'downloading',
        percent: pct,
        speed_mbps: p.speed_mbps || 0,
        eta_secs: p.eta_secs || 0,
        message: p.phase === 'retrying' ? (p.message || 'Retrying…') : undefined,
      });
      return;
    }

    if (p.phase === 'paused') {
      this.hooks.onHideStatus?.();
      this.setBusy(null, false);
      this.pausePending.delete(model);
      this.clearCardProgress(model);
      this.forceResumeModels.add(model);
      this.lastListKey = '';
      await this.refresh(true);
      this.hooks.onPaused?.(model);
      return;
    }

    if (p.phase === 'done') {
      // An inline switch already shows the result on the card; the banner sits
      // above the grid and would shove every card down on show and hide.
      const wasInlineSwitch = this.inlineSwitch && this.switchingModel === model;
      this.setBusy(null, false);
      this.pausePending.delete(model);
      this.forceResumeModels.delete(model);
      this.clearCardProgress(model);
      this.lastListKey = '';
      if (this.mode === 'settings' && wasInlineSwitch) {
        this.hooks.onHideStatus?.();
      } else if (this.mode === 'settings') {
        this.hooks.onStatus?.('success', message || `${model} model ready`);
        this.hooks.onScheduleHideStatus?.(5000);
      }
      await this.refresh(true);
      this.hooks.onReady?.(model, message);
      return;
    }

    if (p.phase === 'cancelled') {
      this.hooks.onHideStatus?.();
      this.setBusy(null, false);
      this.pausePending.delete(model);
      this.forceResumeModels.delete(model);
      this.clearCardProgress(model);
      this.lastListKey = '';
      await this.refresh(true);
      return;
    }

    if (p.phase === 'error') {
      this.hooks.onHideStatus?.();
      this.setBusy(null, false);
      this.pausePending.delete(model);
      this.clearCardProgress(model);
      this.lastListKey = '';
      if (this.mode === 'settings') {
        this.hooks.onStatus?.('error', message || 'Model operation failed');
        this.hooks.onScheduleHideStatus?.(6000);
      }
      await this.refresh(true);
      this.hooks.onError?.(message || 'Model operation failed');
    }
  }

  async ensureListeners() {
    if (this.listenersAttached) return;
    this.listenersAttached = true;
    this.unlistenProgress = await this.ipc.listen('model:progress', (event) => {
      this.handleProgress(typeof event.payload === 'string' ? JSON.parse(event.payload) : event.payload);
    });
    this.unlistenQueue = await this.ipc.listen('model:queue', () => {
      this.refresh(true);
    });
  }

  // One listener on the grid drives every card, so reconciliation can reuse
  // nodes without worrying about stale per-card handlers.
  ensureContainerListener() {
    if (!this.container || this.containerBound) return;
    this.containerBound = true;
    this.container.addEventListener('click', (e) => this.handleCardClick(e));
  }

  handleCardClick(e) {
    const card = e.target.closest('.model-card');
    if (!card || !this.container.contains(card)) return;

    const model = card.dataset.model;
    if (!model) return;

    const actionEl = e.target.closest('[data-action]');
    const action = actionEl && card.contains(actionEl) ? actionEl.dataset.action : null;

    if (action === 'pause') {
      e.stopPropagation();
      e.preventDefault();
      this.pauseDownload(model);
      return;
    }

    if (action === 'cancel') {
      e.stopPropagation();
      this.cancelQueuedDownload(model);
      return;
    }

    if (this.switchingModel) return;

    const state = card.dataset.state;
    if (state === 'active') return;

    if (action === 'resume') {
      e.stopPropagation();
      this.setModelSelect(model);
      this.queueDownload(model, { activate: true });
      return;
    }

    if (action === 'restart') {
      e.stopPropagation();
      this.setModelSelect(model);
      this.queueDownload(model, { restart: true, activate: true });
      return;
    }

    if (state === 'downloaded') {
      if (this.mode === 'onboarding') {
        this.setModelSelect(model);
        this.queueDownload(model, { activate: true });
      } else {
        this.activate(model);
      }
      return;
    }

    if (state === 'remote' && (action === 'download' || this.mode === 'onboarding')) {
      this.setModelSelect(model);
      this.queueDownload(model, this.mode === 'onboarding' ? { activate: true } : {});
    }
  }

  cardParts(m, waiting) {
    const activeModel = this.getActiveModel();
    const selectedModel = this.modelSelect?.value || activeModel;
    const isDownloaded = m.downloaded;
    const isPartial = !!(m.partial || this.forceResumeModels.has(m.id));
    const isActive = m.id === activeModel && isDownloaded;
    const liveDl = this.cardProgress[m.id];
    // forceResume wins over a stale active-queue race after Pause.
    const isDownloading = (this.downloadingModels.has(m.id)
      || (liveDl && liveDl.phase === 'downloading'))
      && !this.forceResumeModels.has(m.id);
    const isWaiting = waiting.includes(m.id);
    const isSelected = m.id === selectedModel && !isActive;
    const isRecommended = this.recommendedModel && m.id === this.recommendedModel;
    const isSwitching = !!this.switchingModel && this.switchingModel === m.id;
    const cardBusy = this.switchingModel && this.switchingModel !== m.id;
    const isDimmed = !isActive && !isDownloading && (isWaiting || !!cardBusy);

    let state;
    if (isActive) state = 'active';
    else if (isDownloading) state = 'downloading';
    else if (isWaiting) state = 'waiting';
    else if (isPartial) state = 'partial';
    else if (isDownloaded) state = 'downloaded';
    else state = 'remote';

    const waitPos = isWaiting ? waiting.indexOf(m.id) + 1 : 0;

    let iconSvg;
    let actionHtml;
    if (state === 'active') {
      iconSvg = ModelGrid.ICONS.active;
      actionHtml = `<span class="model-badge badge-active">Active</span>`;
    } else if (state === 'downloading') {
      iconSvg = ModelGrid.ICONS.downloading;
      actionHtml = `<span class="model-badge badge-downloading">Downloading</span>`;
    } else if (state === 'waiting') {
      iconSvg = ModelGrid.ICONS.waiting;
      actionHtml = `
        <span class="model-badge badge-waiting">Waiting${waitPos > 0 ? ' #' + waitPos : ''}</span>
        <button type="button" class="model-badge badge-cancel-queue card-cancel-btn" data-action="cancel" data-model="${m.id}">Cancel</button>`;
    } else if (state === 'partial') {
      iconSvg = ModelGrid.ICONS.partial;
      actionHtml = `
        <span class="model-badge badge-partial clickable-badge" data-action="resume">Resume</span>
        <span class="model-badge badge-restart clickable-badge" data-action="restart">Restart</span>`;
    } else if (state === 'downloaded') {
      iconSvg = ModelGrid.ICONS.downloaded;
      actionHtml = `<span class="model-badge badge-load clickable-badge" data-action="load">${this.mode === 'onboarding' ? 'Use' : 'Load'}</span>`;
    } else {
      iconSvg = ModelGrid.ICONS.cloud;
      actionHtml = `<span class="model-badge badge-download clickable-badge" data-action="download">Download</span>`;
    }

    const baseDesc = this.baseModelDesc(m);
    const livePct = liveDl && liveDl.phase === 'downloading' ? Math.round(liveDl.percent || 0) : null;
    const descText = isDownloading && livePct != null
      ? `${baseDesc} — ${livePct}%`
      : `${baseDesc}${isPartial && m.partial_label && !isDownloading ? ' — ' + m.partial_label : ''}`;

    const recTag = isRecommended ? ' <span class="model-tag-rec">recommended</span>' : '';

    const showSpinner = isSwitching && this.inlineSwitch;
    if (showSpinner) {
      actionHtml = `<span class="model-switch-spinner" role="status"></span>` + actionHtml;
    }

    return {
      model: m.id,
      state,
      isDownloading,
      switchLabel: showSpinner ? (this.switchMessage || `Loading ${m.id}...`) : '',
      progress: liveDl || { percent: 0, speed_mbps: 0, eta_secs: 0 },
      className: 'model-card'
        + (isActive ? ' active' : '')
        + (isRecommended ? ' recommended' : '')
        + (isDownloading ? ' downloading' : '')
        + (isWaiting ? ' waiting' : '')
        + (isPartial && !isDownloading && !isWaiting ? ' partial' : '')
        + (isSelected ? ' selected' : '')
        + (isDimmed ? ' dimmed' : '')
        + (isSwitching ? ' switching' : '')
        + (ModelGrid.CLICKABLE_STATES.has(state) ? ' clickable' : ''),
      iconSvg,
      actionHtml,
      actionSig: `${state}:${waitPos}:${showSpinner ? 1 : 0}`,
      nameHtml: `${m.id}${recTag}`,
      baseDesc,
      descText,
    };
  }

  createCard(parts) {
    const card = document.createElement('div');
    card.className = parts.className;
    card.dataset.model = parts.model;
    card.dataset.state = parts.state;
    card.innerHTML = `
      <div class="model-card-row">
        <div class="model-card-icon">${parts.iconSvg}</div>
        <div class="model-card-body">
          <div class="model-card-name">${parts.nameHtml}</div>
          <div class="model-card-desc"></div>
        </div>
        <div class="model-card-action"></div>
      </div>
    `;

    const descEl = card.querySelector('.model-card-desc');
    descEl.dataset.baseDesc = parts.baseDesc;
    descEl.textContent = parts.descText;

    const actionWrap = card.querySelector('.model-card-action');
    actionWrap.innerHTML = parts.actionHtml;
    actionWrap.dataset.sig = parts.actionSig;
    this.applySwitchLabel(card, parts.switchLabel);

    if (parts.isDownloading) {
      card.insertAdjacentHTML('beforeend', this.cardProgressHtml(parts.model, parts.progress));
    }
    return card;
  }

  // The spinner node is kept across phases so its rotation never restarts.
  applySwitchLabel(card, label) {
    const spinner = card?.querySelector('.model-switch-spinner');
    if (!spinner || !label) return;
    if (spinner.getAttribute('aria-label') === label) return;
    spinner.setAttribute('aria-label', label);
    spinner.title = label;
  }

  syncCard(card, parts) {
    if (card.className !== parts.className) card.className = parts.className;
    if (card.dataset.state !== parts.state) card.dataset.state = parts.state;

    const iconWrap = card.querySelector('.model-card-icon');
    if (iconWrap && iconWrap.innerHTML !== parts.iconSvg) iconWrap.innerHTML = parts.iconSvg;

    const nameEl = card.querySelector('.model-card-name');
    if (nameEl && nameEl.innerHTML !== parts.nameHtml) nameEl.innerHTML = parts.nameHtml;

    const descEl = card.querySelector('.model-card-desc');
    if (descEl) {
      if (descEl.dataset.baseDesc !== parts.baseDesc) descEl.dataset.baseDesc = parts.baseDesc;
      if (descEl.textContent !== parts.descText) descEl.textContent = parts.descText;
    }

    const actionWrap = card.querySelector('.model-card-action');
    if (actionWrap && actionWrap.dataset.sig !== parts.actionSig) {
      actionWrap.innerHTML = parts.actionHtml;
      actionWrap.dataset.sig = parts.actionSig;
    }
    this.applySwitchLabel(card, parts.switchLabel);

    const progressEl = card.querySelector('.model-card-progress');
    if (parts.isDownloading && !progressEl) {
      card.insertAdjacentHTML('beforeend', this.cardProgressHtml(parts.model, parts.progress));
    } else if (parts.isDownloading) {
      this.writeCardProgress(progressEl, parts.progress);
    } else if (progressEl) {
      progressEl.remove();
    }
  }

  async refresh(force = false) {
    if (!this.container) return;

    try {
      const result = await this.ipc.getModels();
      const models = (result && result.available) || [];
      const downloading = (result && result.downloading) || [];
      const waiting = (result && result.waiting) || [];
      const serverProgress = (result && result.progress) || [];

      // Published before the unchanged-list fast path so the count stays live
      // for sections that never render the grid.
      this.downloadingModels = new Set(downloading);
      this.publishDownloadCount(this.inFlightModels(downloading, waiting).size);

      for (const p of serverProgress) {
        if (p.model && (!this.cardProgress[p.model] || this.cardProgress[p.model].phase === 'downloading')) {
          this.cardProgress[p.model] = {
            ...this.cardProgress[p.model],
            phase: 'downloading',
            percent: p.percent || 0,
            speed_mbps: this.cardProgress[p.model]?.speed_mbps || 0,
            eta_secs: this.cardProgress[p.model]?.eta_secs || 0,
          };
        }
      }

      this.lastModels = models;
      this.lastDownloading = downloading;
      this.lastWaiting = waiting;

      const listKey = this.listKey(models, downloading, waiting);
      if (!force && listKey === this.lastListKey && this.container.childElementCount > 0) {
        models.forEach(m => {
          const liveDl = this.cardProgress[m.id];
          const isDownloading = downloading.includes(m.id)
            || (liveDl && liveDl.phase === 'downloading');
          if (isDownloading && liveDl) {
            this.updateCardProgress(m.id, liveDl);
          }
        });
        return;
      }

      this.renderCards();
    } catch (e) {
      console.error('Failed to load model list:', e);
    }
  }

  renderCards() {
    if (!this.container) return;

    const models = this.lastModels;
    const downloading = this.lastDownloading;
    const waiting = this.lastWaiting;
    if (!models.length) return;

    this.lastListKey = this.listKey(models, downloading, waiting);
    this.ensureContainerListener();

    const stale = new Map();
    this.container.querySelectorAll('.model-card').forEach(card => {
      stale.set(card.dataset.model, card);
    });

    models.forEach((m, i) => {
      const parts = this.cardParts(m, waiting);

      if (this.forceResumeModels.has(m.id) && m.partial && !downloading.includes(m.id)) {
        this.forceResumeModels.delete(m.id);
      }

      let card = stale.get(m.id);
      if (card) {
        stale.delete(m.id);
        this.syncCard(card, parts);
      } else {
        card = this.createCard(parts);
      }

      const at = this.container.children[i];
      if (at !== card) this.container.insertBefore(card, at || null);
    });

    stale.forEach(card => card.remove());

    this.hooks.onRefresh?.();
  }
}

window.ModelGrid = ModelGrid;
