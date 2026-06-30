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
    this.listenersAttached = false;
    this.unlistenProgress = null;
    this.unlistenQueue = null;
    this.lastListKey = '';
    this.pausePending = new Set();
  }

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
      .map(m => [m.id, m.downloaded, m.partial].join(':'))
      .join('|');
    return `${modelPart}::${downloading.join(',')}::${waiting.join(',')}::${this.getActiveModel()}`;
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
          <button type="button" class="btn secondary btn-small card-pause-btn" data-model="${model}">Pause</button>
        </div>
      </div>`;
  }

  ensureCardDownloadingState(card, p) {
    if (!card.classList.contains('downloading')) {
      card.classList.add('downloading');
      card.classList.remove('partial', 'waiting', 'dimmed');

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

  bindPauseButton(scope, model) {
    const btn = scope?.querySelector('.card-pause-btn');
    if (!btn || btn.dataset.bound === '1') return;
    btn.dataset.bound = '1';
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      e.preventDefault();
      this.pauseDownload(model);
    });
  }

  setBusy(model, busy) {
    this.switchingModel = busy ? model : null;
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
      progressEl = card.querySelector('.model-card-progress');
      this.bindPauseButton(progressEl, model);
      return;
    }

    const pct = Math.round(live.percent || 0);
    const pctEl = progressEl.querySelector('.card-progress-pct');
    const fillEl = progressEl.querySelector('.card-progress-fill');
    const speedEl = progressEl.querySelector('.card-progress-speed');
    const etaEl = progressEl.querySelector('.card-progress-eta');
    if (pctEl) pctEl.textContent = `${pct}%`;
    if (fillEl) fillEl.style.width = `${pct}%`;
    if (speedEl) speedEl.textContent = formatSpeedMbps(live.speed_mbps);
    if (etaEl) etaEl.textContent = this.formatEta(live.eta_secs);
    this.bindPauseButton(progressEl, model);
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
        this.clearCardProgress(model);
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

  async activate(model) {
    if (this.mode === 'onboarding') {
      return this.queueDownload(model, { activate: true });
    }

    const settings = await this.ipc.getSettings();
    if (!settings) return;
    settings.model = model;
    await this.ipc.saveSettings(settings);
    this.hooks.onModelSaved?.(model);
    await this.ensureListeners();
    this.setBusy(model, true);
    this.hooks.onLoading?.(`Loading ${model}...`);
    try {
      const result = await this.ipc.loadModel(model);
      if (result && result.status === 'busy') {
        this.setBusy(null, false);
        this.hooks.onStatus?.('error', result.message || 'Server busy');
        this.hooks.onScheduleHideStatus?.(4000);
      } else if (result && result.status === 'queued') {
        this.setBusy(null, false);
        await this.refresh(true);
      }
      return result;
    } catch (e) {
      this.setBusy(null, false);
      this.hooks.onStatus?.('error', 'Failed to load model: ' + e);
      this.hooks.onScheduleHideStatus?.(5000);
      return null;
    }
  }

  handleProgress(p) {
    if (!p || !p.model) return;

    const model = p.model;
    const message = p.message || '';

    if (this.pausePending.has(model) && p.phase === 'downloading') return;

    if (p.phase === 'preparing' || p.phase === 'stopping' || p.phase === 'starting' || p.phase === 'loading') {
      this.setBusy(model, true);
      this.hooks.onLoading?.(message || `Switching to ${model}...`);
      return;
    }

    if (p.phase === 'waiting') {
      this.hooks.onHideStatus?.();
      this.cardProgress[model] = { phase: 'waiting', queue_position: p.queue_position };
      this.refresh(true);
      return;
    }

    if (p.phase === 'downloading') {
      this.hooks.onHideStatus?.();
      const pct = p.percent || (p.total_bytes > 0 ? (p.bytes_downloaded / p.total_bytes) * 100 : 0);
      this.updateCardProgress(model, {
        phase: 'downloading',
        percent: pct,
        speed_mbps: p.speed_mbps || 0,
        eta_secs: p.eta_secs || 0,
      });
      return;
    }

    if (p.phase === 'paused') {
      this.hooks.onHideStatus?.();
      this.setBusy(null, false);
      this.pausePending.delete(model);
      this.clearCardProgress(model);
      this.refresh(true);
      this.hooks.onPaused?.(model);
      return;
    }

    if (p.phase === 'done') {
      this.setBusy(null, false);
      this.clearCardProgress(model);
      if (this.mode === 'settings') {
        this.hooks.onStatus?.('success', message || `${model} model ready`);
        this.hooks.onScheduleHideStatus?.(5000);
      }
      this.refresh(true);
      this.hooks.onReady?.(model, message);
      return;
    }

    if (p.phase === 'cancelled') {
      this.hooks.onHideStatus?.();
      this.setBusy(null, false);
      this.clearCardProgress(model);
      this.refresh(true);
      return;
    }

    if (p.phase === 'error') {
      this.hooks.onHideStatus?.();
      this.setBusy(null, false);
      this.clearCardProgress(model);
      if (this.mode === 'settings') {
        this.hooks.onStatus?.('error', message || 'Model operation failed');
        this.hooks.onScheduleHideStatus?.(6000);
      }
      this.refresh(true);
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

  bindCard(card, m, ctx) {
    const {
      isActive, isDownloading, isWaiting, isPartial, isDownloaded, isSelected,
    } = ctx;

    if (isDownloading) {
      this.bindPauseButton(card, m.id);
    }

    card.querySelectorAll('.card-cancel-btn').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        this.cancelQueuedDownload(m.id);
      });
    });

    const cardBusy = this.switchingModel && this.switchingModel !== m.id;
    if ((isDownloading || isWaiting || cardBusy) && !isActive) {
      if (!isDownloading) card.classList.add('dimmed');
    }

    if (isActive) return;

    if (this.mode === 'onboarding') {
      if (isDownloading || isWaiting) return;
      if (isPartial) {
        card.classList.add('clickable');
        card.querySelector('[data-action="resume"]')?.addEventListener('click', (e) => {
          e.stopPropagation();
          if (this.switchingModel) return;
          this.setModelSelect(m.id);
          this.queueDownload(m.id, { activate: true });
        });
        card.querySelector('[data-action="restart"]')?.addEventListener('click', (e) => {
          e.stopPropagation();
          if (this.switchingModel) return;
          this.setModelSelect(m.id);
          this.queueDownload(m.id, { restart: true, activate: true });
        });
        return;
      }
      card.classList.add('clickable');
      card.addEventListener('click', () => {
        if (this.switchingModel) return;
        this.setModelSelect(m.id);
        this.queueDownload(m.id, { activate: true });
      });
      return;
    }

    if (isDownloaded && !isActive) {
      card.classList.add('clickable');
      card.addEventListener('click', () => {
        if (this.switchingModel) return;
        this.setModelSelect(m.id);
        this.activate(m.id);
      });
    } else if (isPartial) {
      card.classList.add('clickable');
      card.querySelector('[data-action="resume"]')?.addEventListener('click', (e) => {
        e.stopPropagation();
        if (this.switchingModel) return;
        this.setModelSelect(m.id);
        this.queueDownload(m.id, { activate: true });
      });
      card.querySelector('[data-action="restart"]')?.addEventListener('click', (e) => {
        e.stopPropagation();
        if (this.switchingModel) return;
        this.setModelSelect(m.id);
        this.queueDownload(m.id, { restart: true, activate: true });
      });
    } else if (!isDownloaded && !isWaiting && !isDownloading) {
      card.classList.add('clickable');
      card.querySelector('.clickable-badge')?.addEventListener('click', (e) => {
        e.stopPropagation();
        if (this.switchingModel) return;
        this.setModelSelect(m.id);
        this.queueDownload(m.id);
      });
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
      const activeModel = this.getActiveModel();

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
      this.lastListKey = listKey;

      const selectedModel = this.modelSelect?.value || activeModel;
      this.container.innerHTML = '';

      models.forEach(m => {
        const isDownloaded = m.downloaded;
        const isPartial = m.partial;
        const isActive = m.id === activeModel && isDownloaded;
        const liveDl = this.cardProgress[m.id];
        const isDownloading = downloading.includes(m.id)
          || (liveDl && liveDl.phase === 'downloading');
        const isWaiting = waiting.includes(m.id);
        const isSelected = m.id === selectedModel && !isActive;
        const isRecommended = this.recommendedModel && m.id === this.recommendedModel;

        const card = document.createElement('div');
        card.className = 'model-card'
          + (isActive ? ' active' : '')
          + (isRecommended ? ' recommended' : '')
          + (isDownloading ? ' downloading' : '')
          + (isWaiting ? ' waiting' : '')
          + (isPartial && !isDownloading && !isWaiting ? ' partial' : '')
          + (isSelected ? ' selected' : '');
        card.dataset.model = m.id;

        const baseDesc = this.baseModelDesc(m);
        const livePct = liveDl && liveDl.phase === 'downloading' ? Math.round(liveDl.percent || 0) : null;
        const descText = isDownloading && livePct != null
          ? `${baseDesc} — ${livePct}%`
          : `${baseDesc}${isPartial && m.partial_label && !isDownloading ? ' — ' + m.partial_label : ''}`;

        let iconSvg;
        let actionHtml;

        if (isActive) {
          iconSvg = ModelGrid.ICONS.active;
          actionHtml = `<span class="model-badge badge-active">Active</span>`;
        } else if (isDownloading) {
          iconSvg = ModelGrid.ICONS.downloading;
          actionHtml = `<span class="model-badge badge-downloading">Downloading</span>`;
        } else if (isWaiting) {
          iconSvg = ModelGrid.ICONS.waiting;
          const pos = waiting.indexOf(m.id) + 1;
          actionHtml = `
            <span class="model-badge badge-waiting">Waiting${pos > 0 ? ' #' + pos : ''}</span>
            <button type="button" class="model-badge badge-cancel-queue card-cancel-btn" data-model="${m.id}">Cancel</button>`;
        } else if (isPartial) {
          iconSvg = ModelGrid.ICONS.partial;
          actionHtml = `
            <span class="model-badge badge-partial clickable-badge" data-action="resume">Resume</span>
            <span class="model-badge badge-restart clickable-badge" data-action="restart">Restart</span>`;
        } else if (isDownloaded) {
          iconSvg = ModelGrid.ICONS.downloaded;
          actionHtml = `<span class="model-badge badge-load clickable-badge">${this.mode === 'onboarding' ? 'Use' : 'Load'}</span>`;
        } else {
          iconSvg = ModelGrid.ICONS.cloud;
          actionHtml = `<span class="model-badge badge-download clickable-badge">Download</span>`;
        }

        const recTag = isRecommended ? ' <span class="model-tag-rec">recommended</span>' : '';

        card.innerHTML = `
          <div class="model-card-row">
            <div class="model-card-icon">${iconSvg}</div>
            <div class="model-card-body">
              <div class="model-card-name">${m.id}${recTag}</div>
              <div class="model-card-desc" data-base-desc="${baseDesc.replace(/"/g, '&quot;')}">${descText}</div>
            </div>
            <div class="model-card-action">${actionHtml}</div>
          </div>
        `;

        if (isDownloading) {
          const progressWrap = document.createElement('div');
          progressWrap.innerHTML = this.cardProgressHtml(m.id, liveDl || { percent: 0, speed_mbps: 0, eta_secs: 0 });
          card.appendChild(progressWrap.firstElementChild);
        }

        this.bindCard(card, m, {
          isActive, isDownloading, isWaiting, isPartial, isDownloaded, isSelected,
        });

        this.container.appendChild(card);
      });

      this.hooks.onRefresh?.();
    } catch (e) {
      console.error('Failed to load model list:', e);
    }
  }
}

window.ModelGrid = ModelGrid;
