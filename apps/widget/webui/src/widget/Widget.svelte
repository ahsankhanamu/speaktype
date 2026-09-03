<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { PhysicalPosition, LogicalSize } from '@tauri-apps/api/dpi';
  import { listen } from '@tauri-apps/api/event';
  import Waveform from './Waveform.svelte';
  import { formatHotkey, formatDuration } from '../lib/format';
  import {
    toggleRecording,
    cancelRecording,
    openSettings,
    openOnboarding,
    getOnboardingStatus,
    getSettings,
    requestAccessibility,
    openSystemPane,
    saveWindowPosition,
    invokeCommand,
  } from '../lib/ipc';

  let widgetRoot: HTMLDivElement;
  let beaconRings: HTMLDivElement;
  let readyCard: HTMLDivElement;
  let readyConnector: SVGSVGElement;
  let readyHint: HTMLParagraphElement;
  let readyClose: HTMLButtonElement;
  let widgetShell: HTMLDivElement;
  let widgetEl: HTMLDivElement;
  let timerEl: HTMLSpanElement;
  let recCancel: HTMLButtonElement;
  let recStop: HTMLButtonElement;
  let overlay: HTMLDivElement;
  let permWarning: HTMLSpanElement;
  let serverDot: HTMLSpanElement;
  let waveform: Waveform;
  let stepMic: HTMLDivElement;
  let stepMicStatus: HTMLDivElement;
  let stepMicDesc: HTMLParagraphElement;
  let stepAcc: HTMLDivElement;
  let stepAccStatus: HTMLDivElement;
  let stepAccDesc: HTMLParagraphElement;
  let gotIt: HTMLButtonElement;

  const BEACON_RING_EXTRA = 28;
  const SHELL_VERT_SLACK = 14;
  const READY_PAD_X = 24;
  const READY_PAD_Y = 16;
  const READY_MIN_W = 260;
  const READY_MIN_H = 72;
  const IDLE_SHELL_W = 48;
  const ACTIVE_SHELL_W = 272;
  const LEVEL_AMP = 15;
  const PEAK_AMP = 6;
  const LEVEL_SMOOTH = 0.11;
  const LEVEL_WRITE_EPS = 0.02;
  const DEFAULT_FLASH_MS = 1500;
  const DRAG_THRESHOLD = 5;
  const OVERLAY_DISMISS_MS = 3000;
  const CRASH_FLASH_MS = 3000;
  const SERVER_READY_DISMISS_MS = 3000;
  const LOCATE_HIGHLIGHT_MS = 2500;
  const GUIDE_READY_MS = 10000;
  const STATUS_POLL_MS = 2000;

  let currentState = 'idle';
  let permissionsMissing = false;
  let beaconActive = false;
  let readyVisible = false;
  let readyDismissed = false;
  let onboardingSessionActive = false;
  let readyGuideTimer: ReturnType<typeof setTimeout> | null = null;
  let statusPollInterval: ReturnType<typeof setInterval> | null = null;
  let setupComplete = false;
  let locateHighlightActive = false;
  let recordingStart = 0;
  let waveAnimFrame: number | null = null;
  let targetLevel = 0;
  let smoothedLevel = 0;
  let writtenLevel = -1;
  let lastTimerText = '';
  let lastWindowWidth = IDLE_SHELL_W + 24;
  let lastWindowHeight = 0;
  let resizeSeq: Promise<void> = Promise.resolve();
  let locateTimer: ReturnType<typeof setTimeout> | null = null;
  let _dragState: { x: number; y: number; dragging: boolean; source: string } | null = null;
  let rootPaddingCache: { x: number; y: number } | null = null;
  let unlisteners: Array<() => void> = [];

  function rootPadding() {
    if (!rootPaddingCache) {
      const s = getComputedStyle(widgetRoot);
      rootPaddingCache = {
        x: parseFloat(s.paddingLeft) + parseFloat(s.paddingRight),
        y: parseFloat(s.paddingTop) + parseFloat(s.paddingBottom),
      };
    }
    return rootPaddingCache;
  }

  function setWindowSize(w: number, h: number) {
    getCurrentWindow()
      .setSize(new LogicalSize(w, h))
      .catch(() => {});
  }

  async function setWindowSizeKeepRightEdgeImpl(w: number, h: number) {
    if (w === lastWindowWidth && h === lastWindowHeight) return;
    try {
      const win = getCurrentWindow();
      const delta = w - lastWindowWidth;
      if (delta !== 0) {
        const scale = await win.scaleFactor();
        const pos = await win.outerPosition();
        await win.setPosition(
          new PhysicalPosition(Math.round(pos.x - delta * scale), pos.y),
        );
      }
      await win.setSize(new LogicalSize(w, h));
      lastWindowWidth = w;
      lastWindowHeight = h;
    } catch (e) {
      setWindowSize(w, h);
      lastWindowWidth = w;
      lastWindowHeight = h;
    }
  }

  function setWindowSizeKeepRightEdge(w: number, h: number) {
    resizeSeq = resizeSeq.then(() => setWindowSizeKeepRightEdgeImpl(w, h));
    return resizeSeq;
  }

  function waitForLayout() {
    return new Promise((resolve) => {
      requestAnimationFrame(() => requestAnimationFrame(resolve));
    });
  }

  async function fitReadyWindow() {
    await waitForLayout();
    if (readyCard.classList.contains('hidden')) return;
    const shellW = widgetShell.offsetWidth || 48;
    const cardW = readyCard.offsetWidth || 168;
    const connW = readyConnector.classList.contains('hidden')
      ? 0
      : readyConnector.getBoundingClientRect().width || 28;
    const gaps = 12;
    const rootPad = 24;
    const w = cardW + connW + shellW + gaps + rootPad + READY_PAD_X;
    const h = Math.max(readyCard.offsetHeight, shellW) + READY_PAD_Y;
    await setWindowSizeKeepRightEdge(Math.max(w, READY_MIN_W), Math.max(h, READY_MIN_H));
  }

  async function fitWindowToContent() {
    await waitForLayout();
    if (readyVisible && !readyCard.classList.contains('hidden')) {
      await fitReadyWindow();
      return;
    }
    const pad = rootPadding();
    const shellW = widgetShell.offsetWidth || IDLE_SHELL_W;
    const shellH = widgetShell.offsetHeight || IDLE_SHELL_W;
    const showBeaconRipple = currentState === 'idle' && (beaconActive || locateHighlightActive);
    let contentW = shellW;
    let contentH = shellH + SHELL_VERT_SLACK * 2;
    if (showBeaconRipple) {
      contentW = shellW + BEACON_RING_EXTRA;
      contentH = Math.max(contentH, shellH + BEACON_RING_EXTRA * 2);
    }
    if (currentState === 'recording') {
      contentH = Math.max(contentH, shellH + 40);
    }
    const w = Math.ceil(contentW + pad.x);
    const h = Math.ceil(contentH + pad.y);
    await setWindowSizeKeepRightEdge(w, h);
  }

  function syncWindowSize() {
    fitWindowToContent().catch(() => {});
  }

  async function initWidgetLayout() {
    try {
      const win = getCurrentWindow();
      const size = await win.innerSize();
      const scale = await win.scaleFactor();
      lastWindowWidth = Math.round(size.width / scale);
      lastWindowHeight = Math.round(size.height / scale);
    } catch (e) {
      lastWindowWidth = IDLE_SHELL_W + 24;
      lastWindowHeight = 0;
    }
    syncWindowSize();
  }

  function syncBeaconVisuals() {
    const show = beaconActive && currentState === 'idle';
    widgetEl.classList.toggle('beacon', show);
    widgetRoot.classList.toggle('beacon-active', show);
    beaconRings.classList.toggle('hidden', !show);
    syncWindowSize();
  }

  function setBeacon(on: boolean) {
    beaconActive = on;
    syncBeaconVisuals();
  }

  function clearBeaconVisuals() {
    beaconActive = false;
    widgetEl.classList.remove('beacon', 'locate');
    widgetRoot.classList.remove('beacon-active');
    beaconRings.classList.add('hidden');
  }

  function dismissReady() {
    readyDismissed = true;
    readyVisible = false;
    readyCard.classList.add('hidden');
    readyConnector.classList.add('hidden');
    widgetRoot.classList.remove('ready');
    if (readyGuideTimer) clearTimeout(readyGuideTimer);
    syncWindowSize();
  }

  async function showReadyTooltip(hotkey: string) {
    readyDismissed = false;
    readyVisible = true;
    clearBeaconVisuals();
    readyHint.textContent = 'Click the mic or press ' + formatHotkey(hotkey, { compact: true });
    widgetRoot.classList.add('ready');
    readyCard.classList.remove('hidden');
    readyConnector.classList.remove('hidden');
    await fitReadyWindow();
    if (readyGuideTimer) clearTimeout(readyGuideTimer);
    readyGuideTimer = setTimeout(() => dismissReady(), GUIDE_READY_MS);
  }

  async function maybeShowReadyGuide() {
    if (readyDismissed || readyVisible) return;
    try {
      const s = await getOnboardingStatus();
      if (!s?.microphone || !s?.accessibility || !s?.model) return;
      const settings = await getSettings();
      await showReadyTooltip(settings?.hotkey || 'Super+Control');
    } catch (e) {
      console.error('[ready] guide error:', e);
    }
  }

  function syncPermissionState(mic: boolean, acc: boolean) {
    permissionsMissing = !mic || !acc;
    permWarning.style.display = permissionsMissing ? 'block' : 'none';
  }

  async function runStatusPoll() {
    if (currentState === 'recording' || currentState === 'transcribing') return;
    let s: any;
    try {
      s = await getOnboardingStatus();
    } catch (e) {
      console.error('[status] poll error:', e);
      return;
    }
    if (!s) return;
    const mic = !!s.microphone;
    const acc = !!s.accessibility;
    const overlayVisible = overlay.style.display !== 'none';
    syncPermissionState(mic, acc);
    if (overlayVisible) updatePermissionOverlay(mic, acc);
    const allDone = mic && acc && !!s.model;
    if (!readyVisible && !readyDismissed) {
      if (allDone && !onboardingSessionActive) {
        setBeacon(false);
      } else {
        onboardingSessionActive = onboardingSessionActive || !allDone;
        setBeacon(true);
      }
    }
    setupComplete = allDone && !onboardingSessionActive;
    if (setupComplete && !overlayVisible) stopStatusPolling();
  }

  function startStatusPolling() {
    if (statusPollInterval) return;
    runStatusPoll();
    statusPollInterval = setInterval(runStatusPoll, STATUS_POLL_MS);
  }

  function stopStatusPolling() {
    if (statusPollInterval) {
      clearInterval(statusPollInterval);
      statusPollInterval = null;
    }
  }

  function resumeStatusPolling() {
    setupComplete = false;
    startStatusPolling();
  }

  function setState(s: string) {
    currentState = s;
    widgetEl.className = 'widget ' + s;
    if (s === 'recording' || s === 'transcribing') {
      dismissReady();
      widgetEl.classList.remove('beacon', 'locate');
      widgetRoot.classList.remove('beacon-active');
      beaconRings.classList.add('hidden');
      stopStatusPolling();
    } else {
      syncBeaconVisuals();
      if (!setupComplete) startStatusPolling();
    }
    syncWindowSize();
  }

  function startTimer() {
    recordingStart = Date.now();
    lastTimerText = '0.0s';
    timerEl.textContent = lastTimerText;
  }

  function resetWaveform() {
    targetLevel = 0;
    smoothedLevel = 0;
    writtenLevel = 0;
    widgetEl.style.setProperty('--level', '0');
    waveform.reset();
  }

  function setTargetLevel(rms: number, peak: number) {
    targetLevel = Math.min(rms * LEVEL_AMP, 1);
    waveform.setLevel(Math.min(peak * PEAK_AMP, 1));
  }

  function tickWaveform(ts: number) {
    if (currentState !== 'recording') {
      waveAnimFrame = null;
      return;
    }
    smoothedLevel += (targetLevel - smoothedLevel) * LEVEL_SMOOTH;
    if (Math.abs(smoothedLevel - writtenLevel) > LEVEL_WRITE_EPS) {
      writtenLevel = smoothedLevel;
      widgetEl.style.setProperty('--level', smoothedLevel.toFixed(3));
    }
    waveform.frame(ts);
    const elapsed = (Date.now() - recordingStart) / 1000;
    const text = elapsed >= 60 ? formatDuration(elapsed) : elapsed.toFixed(1) + 's';
    if (text !== lastTimerText) {
      lastTimerText = text;
      timerEl.textContent = text;
    }
    waveAnimFrame = requestAnimationFrame(tickWaveform);
  }

  function startWaveformLoop() {
    if (!waveAnimFrame) waveAnimFrame = requestAnimationFrame(tickWaveform);
  }

  function stopWaveformLoop() {
    if (waveAnimFrame) {
      cancelAnimationFrame(waveAnimFrame);
      waveAnimFrame = null;
    }
    resetWaveform();
  }

  function flashState(s: string, ms?: number) {
    setState(s);
    setTimeout(() => setState('idle'), ms || DEFAULT_FLASH_MS);
  }

  function highlightWidget(ms?: number) {
    const duration = ms || LOCATE_HIGHLIGHT_MS;
    locateHighlightActive = true;
    widgetEl.classList.add('locate');
    widgetRoot.classList.add('beacon-active');
    beaconRings.classList.remove('hidden');
    syncWindowSize();
    if (locateTimer) clearTimeout(locateTimer);
    locateTimer = setTimeout(() => {
      locateHighlightActive = false;
      widgetEl.classList.remove('locate');
      if (!beaconActive) {
        widgetRoot.classList.remove('beacon-active');
        beaconRings.classList.add('hidden');
      }
      syncWindowSize();
    }, duration);
  }

  function startDragTracking(e: MouseEvent, source: string) {
    if (e.button !== 0) return;
    if (source === 'widget') {
      if (currentState === 'transcribing') return;
      _dragState = { x: e.screenX, y: e.screenY, dragging: false, source: 'widget' };
      return;
    }
    if (currentState === 'recording' || currentState === 'transcribing') return;
    _dragState = { x: e.screenX, y: e.screenY, dragging: false, source };
  }

  function bindDragSurface(el: Element | null, source: string, ignoreSelector?: string) {
    if (!el) return;
    el.addEventListener('mousedown', (e) => {
      const me = e as MouseEvent;
      if (ignoreSelector && (me.target as Element).closest(ignoreSelector)) return;
      startDragTracking(me, source);
    });
  }

  function updatePermissionOverlay(micOk: boolean, accOk: boolean) {
    if (!micOk) {
      stepMicStatus.textContent = '!';
      stepMicStatus.className = 'step-status step-pending';
      stepMicDesc.textContent = 'Click to open System Settings';
      stepMic.style.display = '';
      stepMic.classList.add('clickable');
      stepMic.onclick = () => openSystemPane('com.apple.preference.security?Privacy_Microphone');
    } else {
      stepMicStatus.textContent = '\u2713';
      stepMicStatus.className = 'step-status step-done';
      stepMicDesc.textContent = 'Microphone access granted';
      stepMic.classList.remove('clickable');
      stepMic.onclick = null;
    }

    if (!accOk) {
      stepAccStatus.textContent = '!';
      stepAccStatus.className = 'step-status step-pending';
      stepAccDesc.textContent = 'Click to open System Settings';
      stepAcc.style.display = '';
      stepAcc.classList.add('clickable');
      stepAcc.onclick = () => openSystemPane('com.apple.preference.security?Privacy_Accessibility');
    } else {
      stepAccStatus.textContent = '\u2713';
      stepAccStatus.className = 'step-status step-done';
      stepAccDesc.textContent = 'Accessibility access granted';
      stepAcc.classList.remove('clickable');
      stepAcc.onclick = null;
    }

    if (micOk && accOk) {
      gotIt.style.display = '';
      gotIt.textContent = 'Done';
      gotIt.onclick = () => {
        overlay.style.display = 'none';
      };
      setTimeout(() => {
        overlay.style.display = 'none';
      }, OVERLAY_DISMISS_MS);
    } else {
      gotIt.style.display = '';
      gotIt.textContent = 'Open Settings';
      gotIt.onclick = () => {
        openSettings();
        overlay.style.display = 'none';
      };
    }
  }

  function onDocumentMouseMove(e: MouseEvent) {
    if (!_dragState || _dragState.dragging) return;
    if (currentState === 'recording') return;
    const dx = e.screenX - _dragState.x;
    const dy = e.screenY - _dragState.y;
    if (Math.abs(dx) > DRAG_THRESHOLD || Math.abs(dy) > DRAG_THRESHOLD) {
      _dragState.dragging = true;
      getCurrentWindow()
        .startDragging()
        .catch(() => {});
    }
  }

  function onDocumentMouseUp(e: MouseEvent) {
    if (!_dragState) return;
    const wasDrag = _dragState.dragging;
    const source = _dragState.source;
    _dragState = null;
    if (wasDrag) {
      saveWindowPosition().catch(() => {});
      return;
    }
    if (source !== 'widget') return;
    if (currentState === 'transcribing') return;
    if (currentState === 'recording') return;
    if (permissionsMissing) {
      openOnboarding();
    } else {
      toggleRecording(currentState === 'recording').catch((e2) =>
        console.error('[toggle] error:', e2),
      );
    }
  }

  onMount(() => {
    bindDragSurface(widgetShell, 'widget', '.rec-btn');
    bindDragSurface(readyCard, 'ready', '.ready-close');
    bindDragSurface(readyConnector, 'ready');

    document.addEventListener('mousemove', onDocumentMouseMove);
    document.addEventListener('mouseup', onDocumentMouseUp);

    widgetEl.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      openSettings().catch(() => {});
    });

    readyClose.onclick = (e) => {
      e.stopPropagation();
      dismissReady();
    };

    recStop.onclick = (e) => {
      e.stopPropagation();
      if (currentState !== 'recording') return;
      toggleRecording(true).catch((e2) => console.error('[stop] error:', e2));
    };

    recCancel.onclick = (e) => {
      e.stopPropagation();
      if (currentState !== 'recording') return;
      cancelRecording().catch((e2) => console.error('[cancel] error:', e2));
    };

    const registerListen = (ev: string, fn: (payload: any) => void) => {
      listen(ev, (event) => fn(event.payload)).then((unlisten) => {
        unlisteners.push(unlisten);
      });
    };

    const parse = (payload: any) => {
      if (typeof payload === 'string') {
        try {
          return JSON.parse(payload);
        } catch {
          return payload;
        }
      }
      return payload;
    };

    registerListen('sidecar:ready', () => console.log('[sidecar] ready'));
    registerListen('sidecar:recording_started', () => {
      setState('recording');
      startTimer();
      startWaveformLoop();
    });
    registerListen('sidecar:audio_level', (p) => {
      if (currentState !== 'recording') return;
      const d = parse(p);
      const level = d.level || 0;
      setTargetLevel(level, typeof d.peak === 'number' ? d.peak : level);
    });
    registerListen('sidecar:recording_stopped', () => {
      stopWaveformLoop();
    });
    registerListen('sidecar:recording_cancelled', () => {
      stopWaveformLoop();
      setState('idle');
    });
    registerListen('sidecar:transcribing', () => {
      stopWaveformLoop();
      setState('transcribing');
    });
    registerListen('sidecar:long_audio', (p) => {
      const d = parse(p);
      console.log('[widget] Long audio detected:', d.duration_secs, 's');
    });
    registerListen('sidecar:pasted', () => flashState('success'));
    registerListen('sidecar:no_speech', () => {
      stopWaveformLoop();
      flashState('error');
    });
    registerListen('sidecar:busy', () => {
      stopWaveformLoop();
      flashState('error');
    });
    registerListen('sidecar:error', () => {
      stopWaveformLoop();
      flashState('error');
    });
    registerListen('sidecar:crashed', () => {
      stopWaveformLoop();
      flashState('error', CRASH_FLASH_MS);
    });
    registerListen('sidecar:permissions', (p) => {
      const d = parse(p);
      syncPermissionState(!!d.microphone, !!d.accessibility);
      if (permissionsMissing) resumeStatusPolling();
    });
    registerListen('sidecar:device_changed', (p) => {
      const d = parse(p);
      console.log('[device] changed:', d.name, 'inputs:', d.count);
    });

    registerListen('server:starting', () => {
      serverDot.className = 'server-status starting';
      serverDot.title = 'Server starting...';
      serverDot.style.display = 'block';
    });
    registerListen('server:ready', (p) => {
      const d = parse(p);
      serverDot.className = 'server-status ready';
      serverDot.title = 'Server ready (port ' + (d.port || '?') + ')';
      setTimeout(() => {
        serverDot.style.display = 'none';
      }, SERVER_READY_DISMISS_MS);
    });
    registerListen('server:error', (p) => {
      const d = parse(p);
      serverDot.className = 'server-status error';
      serverDot.title = 'Server error: ' + (d.reason || 'unknown');
      serverDot.style.display = 'block';
    });
    registerListen('widget:highlight', (p) => {
      const d = parse(p) || {};
      highlightWidget(d.duration_ms);
    });
    registerListen('onboarding:opened', () => {
      onboardingSessionActive = true;
      setBeacon(true);
      resumeStatusPolling();
    });
    registerListen('onboarding:finished', (p) => {
      const d = parse(p) || {};
      onboardingSessionActive = false;
      stopStatusPolling();
      showReadyTooltip(d.hotkey || 'Super+Control');
    });
    registerListen('sidecar:onboarding_required', () => {
      onboardingSessionActive = true;
      setBeacon(true);
      resumeStatusPolling();
    });

    startStatusPolling();
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) {
        stopStatusPolling();
      } else if (setupComplete) {
        runStatusPoll();
      } else {
        startStatusPolling();
      }
    });

    let resizeFrame: number | null = null;
    const resizeObserver = new ResizeObserver(() => {
      if (resizeFrame) return;
      resizeFrame = requestAnimationFrame(() => {
        resizeFrame = null;
        syncWindowSize();
      });
    });
    resizeObserver.observe(widgetShell);
    resizeObserver.observe(readyCard);

    initWidgetLayout().then(() => {
      setTimeout(maybeShowReadyGuide, 500);
    });
    console.log('[widget] loaded');

    return () => {
      document.removeEventListener('mousemove', onDocumentMouseMove);
      document.removeEventListener('mouseup', onDocumentMouseUp);
      unlisteners.forEach((u) => u());
      unlisteners = [];
      if (statusPollInterval) clearInterval(statusPollInterval);
      if (readyGuideTimer) clearTimeout(readyGuideTimer);
      if (locateTimer) clearTimeout(locateTimer);
      if (waveAnimFrame) cancelAnimationFrame(waveAnimFrame);
      resizeObserver.disconnect();
    };
  });
</script>

<div id="widget-root" class="widget-root" bind:this={widgetRoot}>
  <div id="ready-card" class="ready-card hidden" bind:this={readyCard}>
    <button
      class="ready-close"
      id="ready-close"
      title="Dismiss"
      aria-label="Dismiss"
      bind:this={readyClose}
    >×</button>
    <p class="ready-title">You're all set</p>
    <p class="ready-hint" id="ready-hint" bind:this={readyHint}>Click the mic to start dictating</p>
  </div>
  <svg
    id="ready-connector"
    class="ready-connector hidden"
    viewBox="0 0 28 12"
    aria-hidden="true"
    bind:this={readyConnector}
  >
    <defs>
      <marker
        id="ready-arrowhead"
        viewBox="0 0 8 8"
        refX="7"
        refY="4"
        markerWidth="7"
        markerHeight="7"
        orient="auto"
      >
        <path d="M1 1 L7 4 L1 7 Z" fill="currentColor" />
      </marker>
    </defs>
    <line
      class="ready-connector-line"
      x1="1"
      y1="6"
      x2="20"
      y2="6"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      marker-end="url(#ready-arrowhead)"
    />
  </svg>
  <div class="widget-shell" id="widget-shell" bind:this={widgetShell}>
    <div class="beacon-rings hidden" id="beacon-rings" aria-hidden="true" bind:this={beaconRings}>
      <span class="beacon-ring"></span>
      <span class="beacon-ring beacon-ring--delay"></span>
    </div>
    <div id="widget" class="widget idle" bind:this={widgetEl}>
      <div class="state-idle">
        <div class="icon-container">
          <svg class="mic-icon" viewBox="0 0 24 24" fill="currentColor">
            <path
              d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z"
            />
            <path
              d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z"
            />
          </svg>
          <span class="perm-warning" id="permWarning" title="Permissions needed" bind:this={permWarning}></span>
          <span
            class="server-status"
            id="serverStatus"
            title="Server starting..."
            bind:this={serverDot}
          ></span>
        </div>
      </div>
      <div class="state-recording">
        <button
          type="button"
          class="rec-btn rec-cancel"
          id="rec-cancel"
          title="Cancel"
          aria-label="Cancel recording"
          bind:this={recCancel}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
            <path d="M6 6l12 12M18 6l-12 12" />
          </svg>
        </button>
        <Waveform bind:this={waveform} />
        <span class="timer" bind:this={timerEl}>0.0s</span>
        <button
          type="button"
          class="rec-btn rec-stop"
          id="rec-stop"
          title="Stop and transcribe"
          aria-label="Stop recording"
          bind:this={recStop}
        >
          <svg viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="6" width="12" height="12" rx="2" />
          </svg>
        </button>
      </div>
      <div class="state-transcribing">
        <div class="dots">
          <span class="dot"></span><span class="dot"></span><span class="dot"></span>
        </div>
      </div>
      <div class="state-success">
        <svg class="check-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M4.5 12.75l6 6 9-13.5" />
        </svg>
      </div>
      <div class="state-error">
        <svg class="error-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
          <path d="M6 6l12 12M18 6l-12 12" />
        </svg>
      </div>
    </div>
  </div>
</div>

<div id="welcome-overlay" class="welcome-overlay" style="display:none" bind:this={overlay}>
  <div class="welcome-card">
    <div class="welcome-icon">
      <svg viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z" />
        <path d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z" />
      </svg>
    </div>
    <h2>Welcome to SpeakType</h2>
    <p class="welcome-desc">Push-to-talk voice typing that works everywhere.</p>

    <div class="permission-step" id="perm-step-mic" bind:this={stepMic}>
      <div class="step-status" id="step-mic-status" bind:this={stepMicStatus}>1</div>
      <div class="step-content">
        <div class="step-title">Microphone Access</div>
        <div class="step-desc" id="step-mic-desc" bind:this={stepMicDesc}>Required for recording your voice</div>
      </div>
    </div>

    <div class="permission-step" id="perm-step-acc" bind:this={stepAcc}>
      <div class="step-status" id="step-acc-status" bind:this={stepAccStatus}>2</div>
      <div class="step-content">
        <div class="step-title">Accessibility Access</div>
        <div class="step-desc" id="step-acc-desc" bind:this={stepAccDesc}>Required for pasting text into other apps</div>
      </div>
    </div>

    <button class="welcome-btn" id="welcome-got-it" style="display:none" bind:this={gotIt}>Got it!</button>
  </div>
</div>

<style>
  :global {
  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }
  :global(html),
  :global(body) {
    background: transparent;
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    width: 100%;
    height: 100%;
  }
  .widget-root {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-content: center;
    width: 100%;
    height: 100%;
    padding: 6px 12px;
    box-sizing: border-box;
  }
  .widget-root.ready {
    position: relative;
    flex-direction: row;
    align-items: center;
    justify-content: flex-end;
    gap: 0;
    padding: 6px 12px;
  }
  .ready-card {
    position: absolute;
    right: 88px;
    top: 50%;
    transform: translateY(-50%);
    z-index: 3;
    flex-shrink: 0;
    width: max-content;
    max-width: 240px;
    padding: 8px 28px 8px 12px;
    text-align: right;
    pointer-events: auto;
    cursor: grab;
    border-radius: 8px;
    background: var(--widget-panel);
    border: 1px solid var(--widget-panel-border);
    animation: readyFade 0.25s ease-out;
  }
  .ready-card:active {
    cursor: grabbing;
  }
  .ready-card.hidden {
    display: none;
  }
  .ready-connector {
    position: absolute;
    right: 54px;
    top: 50%;
    margin-top: -6px;
    width: 28px;
    height: 12px;
    flex-shrink: 0;
    color: var(--widget-text-faint);
    overflow: visible;
    z-index: 2;
  }
  .ready-connector.hidden {
    display: none;
  }
  .ready-connector-line {
    opacity: 0;
    animation: connectorFade 0.2s ease-out 0.08s forwards;
  }
  @keyframes readyFade {
    from {
      opacity: 0;
      transform: translateY(-50%) translateX(-6px);
    }
    to {
      opacity: 1;
      transform: translateY(-50%) translateX(0);
    }
  }
  @keyframes connectorFade {
    to {
      opacity: 1;
    }
  }
  .ready-close {
    position: absolute;
    top: 6px;
    right: 6px;
    width: 18px;
    height: 18px;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--widget-text-faint);
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    font-family: inherit;
    transition: color 0.15s, background 0.15s;
  }
  .ready-close:hover {
    color: var(--widget-icon-hover);
    background: var(--widget-btn-hover);
  }
  .ready-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--widget-text);
    letter-spacing: -0.01em;
    line-height: 1.3;
  }
  .ready-hint {
    font-size: 11px;
    color: var(--widget-text-muted);
    margin-top: 3px;
    line-height: 1.35;
    word-break: break-word;
  }
  .widget-shell {
    position: relative;
    width: 48px;
    height: 48px;
    flex-shrink: 0;
    overflow: visible;
    cursor: pointer;
    transition: width 0.35s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .widget-shell:has(.widget.recording) {
    width: 272px;
  }
  .widget-shell:has(.widget.transcribing) {
    width: 148px;
  }
  .beacon-rings {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 48px;
    height: 48px;
    margin-left: -24px;
    margin-top: -24px;
    pointer-events: none;
    z-index: 0;
  }
  .beacon-rings.hidden {
    display: none;
  }
  .beacon-ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 2px solid rgba(239, 100, 60, 0.75);
    will-change: transform, opacity;
    transform: translateZ(0);
    animation: beaconRipple 2.4s cubic-bezier(0.22, 1, 0.36, 1) infinite;
  }
  .beacon-ring--delay {
    animation-delay: 1.2s;
  }
  @keyframes beaconRipple {
    0% {
      transform: scale(1) translateZ(0);
      opacity: 0.85;
    }
    100% {
      transform: scale(2.15) translateZ(0);
      opacity: 0;
    }
  }
  .widget {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    height: 48px;
    width: 48px;
    cursor: pointer;
    transition: width 0.35s cubic-bezier(0.22, 1, 0.36, 1), border-radius 0.35s cubic-bezier(0.22, 1, 0.36, 1), background-color 0.25s ease, box-shadow 0.14s ease-out;
  }
  .state-idle,
  .state-recording,
  .state-transcribing,
  .state-success,
  .state-error {
    display: none;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .widget.idle {
    background: var(--widget-idle);
    box-shadow: 0 2px 12px var(--shadow);
  }
  .widget.idle .state-idle {
    display: flex;
  }
  .icon-container {
    position: relative;
    width: 22px;
    height: 22px;
  }
  .mic-icon {
    width: 22px;
    height: 22px;
    color: var(--widget-text);
    transition: transform 0.15s;
  }
  .widget.idle:hover .mic-icon {
    color: var(--widget-text);
    transform: scale(1.12);
  }
  .widget.idle.beacon {
    box-shadow: 0 0 16px rgba(255, 110, 50, 0.45), 0 2px 12px rgba(0, 0, 0, 0.5);
  }
  .widget.idle.beacon .mic-icon {
    color: rgba(255, 200, 170, 0.98);
  }
  .widget.locate {
    animation: locateGlow 1.2s ease-in-out infinite;
  }
  .widget.idle.locate {
    box-shadow: 0 0 18px rgba(255, 110, 50, 0.55), 0 2px 12px rgba(0, 0, 0, 0.5);
  }
  .widget.idle.locate .mic-icon {
    color: rgba(255, 200, 170, 0.98);
  }
  @keyframes locateGlow {
    0%,
    100% {
      filter: brightness(1);
    }
    50% {
      filter: brightness(1.12);
    }
  }
  .perm-warning {
    display: none;
    position: absolute;
    top: -3px;
    right: -3px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--warning);
    box-shadow: 0 0 6px rgba(234, 179, 8, 0.6);
    animation: permPulse 1.5s ease-in-out infinite;
    pointer-events: none;
  }
  @keyframes permPulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }
  .server-status {
    display: none;
    position: absolute;
    bottom: -3px;
    right: -3px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    pointer-events: none;
  }
  .server-status.starting {
    background: var(--warning);
    box-shadow: 0 0 6px rgba(234, 179, 8, 0.6);
    animation: permPulse 1.5s ease-in-out infinite;
  }
  .server-status.ready {
    background: var(--success);
    box-shadow: 0 0 6px rgba(74, 222, 128, 0.6);
  }
  .server-status.error {
    background: var(--danger);
    box-shadow: 0 0 6px rgba(239, 68, 68, 0.6);
  }
  .widget.recording {
    --level: 0;
    background: var(--widget-shell);
    border: 1px solid var(--widget-panel-border);
    box-shadow: 0 0 calc(6px + 12px * var(--level)) rgba(239, 100, 60, calc(0.12 + 0.3 * var(--level))), 0 2px 8px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.06);
    width: 272px;
    border-radius: 24px;
    overflow: visible;
    animation: recordIn 0.32s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .widget.recording .state-recording {
    display: flex;
  }
  @keyframes recordIn {
    from {
      opacity: 0.65;
      transform: scale(0.97);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
  .state-recording {
    width: 100%;
    padding: 0 6px;
    gap: 6px;
  }
  .rec-btn {
    flex-shrink: 0;
    position: relative;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: 1px solid var(--widget-panel-border);
    border-radius: 50%;
    background: var(--widget-btn-bg);
    color: var(--widget-text-faint);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.06);
    cursor: pointer;
    pointer-events: auto;
    transition: background 0.15s, color 0.15s, border-color 0.15s, box-shadow 0.15s;
  }
  .rec-btn svg {
    width: 14px;
    height: 14px;
  }
  .rec-btn:hover {
    background: rgba(255, 255, 255, 0.18);
    color: var(--widget-text);
    border-color: rgba(255, 255, 255, 0.14);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.08);
  }
  .rec-cancel:hover {
    background: rgba(248, 113, 113, 0.18);
    color: #f87171;
  }
  .rec-stop svg {
    width: 16px;
    height: 16px;
  }
  .rec-stop {
    color: rgba(239, 100, 60, 0.95);
  }
  .rec-stop:hover {
    background: rgba(239, 100, 60, 0.22);
    color: #ff8a65;
  }
  .timer {
    color: var(--widget-icon);
    font-size: 11px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    min-width: 36px;
    margin-right: 2px;
    text-align: right;
    pointer-events: none;
  }
  .widget.transcribing {
    background: var(--widget-shell);
    border: 1px solid var(--widget-panel-border);
    box-shadow: 0 0 14px rgba(255, 255, 255, 0.12), 0 2px 8px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.06);
    width: 148px;
    border-radius: 24px;
  }
  .widget.transcribing .state-transcribing {
    display: flex;
  }
  .dots {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--widget-icon-hover);
    animation: dotPulse 1.2s ease-in-out infinite;
  }
  .dot:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dot:nth-child(3) {
    animation-delay: 0.3s;
  }
  @keyframes dotPulse {
    0%,
    100% {
      opacity: 0.3;
      transform: scale(0.8);
    }
    50% {
      opacity: 1;
      transform: scale(1.3);
    }
  }
  .widget.success {
    background: var(--widget-shell);
    border: 1px solid var(--widget-panel-border);
    box-shadow: 0 0 16px rgba(74, 222, 128, 0.35), 0 2px 8px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.06);
    animation: successPop 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .widget.success .state-success {
    display: flex;
  }
  .check-icon {
    width: 22px;
    height: 22px;
    color: var(--success);
    stroke-dasharray: 30;
    stroke-dashoffset: 30;
    animation: drawCheck 0.4s ease-out 0.05s forwards;
  }
  @keyframes drawCheck {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes successPop {
    0% {
      transform: scale(0.8);
      opacity: 0;
    }
    60% {
      transform: scale(1.08);
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }
  .widget.error {
    background: var(--widget-shell);
    border: 1px solid var(--widget-panel-border);
    box-shadow: 0 0 14px rgba(248, 113, 60, 0.3), 0 2px 8px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.06);
    animation: errorShake 0.4s ease-out;
  }
  .widget.error .state-error {
    display: flex;
  }
  .error-icon {
    width: 20px;
    height: 20px;
    color: var(--danger);
    stroke-dasharray: 24;
    stroke-dashoffset: 24;
    animation: drawX 0.3s ease-out 0.05s forwards;
  }
  @keyframes drawX {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes errorShake {
    0% {
      transform: scale(0.9);
    }
    25% {
      transform: translateX(-3px);
    }
    50% {
      transform: translateX(3px);
    }
    75% {
      transform: translateX(-2px);
    }
    100% {
      transform: translateX(0) scale(1);
    }
  }
  .welcome-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }
  .welcome-card {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 28px;
    width: 320px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
  .welcome-icon {
    width: 48px;
    height: 48px;
    margin: 0 auto 12px;
    background: linear-gradient(135deg, #3b82f6, #8b5cf6);
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .welcome-icon svg {
    width: 28px;
    height: 28px;
    color: white;
  }
  .welcome-card h2 {
    text-align: center;
    font-size: 18px;
    color: var(--text-strong, #fff);
    margin-bottom: 4px;
  }
  .welcome-desc {
    text-align: center;
    font-size: 13px;
    color: var(--text-dim);
    margin-bottom: 20px;
  }
  .permission-step {
    display: flex;
    gap: 12px;
    padding: 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    margin-bottom: 8px;
  }
  .permission-step.clickable {
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }
  .permission-step.clickable:hover {
    background: var(--surface-3);
    border-color: var(--border-strong);
  }
  .step-status {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 600;
    flex-shrink: 0;
  }
  .step-status.step-pending {
    background: rgba(234, 179, 8, 0.15);
    color: var(--warning);
  }
  .step-status.step-done {
    background: rgba(74, 222, 128, 0.15);
    color: var(--success);
  }
  .step-content {
    flex: 1;
    min-width: 0;
  }
  .step-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
    margin-bottom: 2px;
  }
  .step-desc {
    font-size: 12px;
    color: var(--text-dim);
    line-height: 1.4;
  }
  .welcome-btn {
    width: 100%;
    padding: 10px;
    border: none;
    border-radius: 8px;
    background: #3b82f6;
    color: white;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    margin-top: 12px;
    transition: background 0.15s;
  }
  .welcome-btn:hover {
    background: #2563eb;
  }
  }</style>