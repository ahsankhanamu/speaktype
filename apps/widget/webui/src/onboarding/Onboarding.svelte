<script lang="ts">
  import { onMount } from 'svelte';
  import ModelGrid from '../lib/ModelGrid.svelte';
  import type { OnboardingStatus } from '../lib/ipc';
  import {
    openSystemPane,
    quitApp,
    finishOnboarding,
    getOnboardingStatus,
    requestMicrophoneAccess,
  } from '../lib/ipc';

  const RECOMMENDED = 'small.en';
  const SETTINGS_MODELS_HINT =
    ' You can also download or switch models anytime in Settings (menu bar icon → Models).';

  let status: OnboardingStatus = {
    microphone: false,
    accessibility: false,
    model: false,
    active_model: '',
    exe_path: '',
    is_dev: false,
    is_bundled: false,
    responsible_app: '',
    responsible_app_path: '',
  };
  let lastStatusKey = '';
  let micPromptRequested = false;
  let micSettingsOpened = false;
  let accSettingsOpened = false;
  let modelGridShown = false;
  let descMic = '';
  let descAcc = '';
  let descModel = '';
  let submitting = false;
  let submitDisabled = true;

  let modelGrid: ModelGrid;

  function statusKey(s: OnboardingStatus): string {
    return [
      s.microphone,
      s.accessibility,
      s.model,
      s.active_model || '',
      s.is_dev ? '1' : '0',
      s.is_bundled ? '1' : '0',
      s.exe_path || '',
      s.responsible_app || '',
    ].join('|');
  }

  let modelStepState: 'locked' | 'active' | 'done' = 'locked';

  function render(refreshModels = false) {
    const micDone = status.microphone;
    const accDone = status.accessibility;
    const modelDone = status.model;

    const micEl = document.getElementById('step-mic');
    const accEl = document.getElementById('step-acc');
    if (micEl) setStep(micEl, micDone ? 'done' : 'active');
    if (accEl) setStep(accEl, !micDone ? 'locked' : accDone ? 'done' : 'active');
    modelStepState = modelDone ? 'done' : 'active';

    if (micDone) micSettingsOpened = false;
    descMic = micDone
      ? 'Microphone access granted.'
      : 'Allow microphone access when prompted, or enable SpeakType under Privacy & Security → Microphone in System Settings.';

    if (!micDone && !micPromptRequested) {
      micPromptRequested = true;
      requestMicrophoneAccess().then(() => setTimeout(() => poll(true), 600));
    }

    if (accDone) accSettingsOpened = false;
    descAcc = accDone
      ? 'Accessibility access granted.'
      : status.responsible_app
        ? 'Open System Settings → Privacy & Security → Accessibility and enable "' +
          status.responsible_app +
          '" (then Quit & Reopen).'
        : 'Open System Settings → Privacy & Security → Accessibility and enable the SpeakType entry for this install (then Quit & Reopen).';

    updateRelaunchHints(micDone, accDone);

    if (modelDone) {
      descModel = 'Model "' + status.active_model + '" is ready.' + SETTINGS_MODELS_HINT;
      modelGridShown = false;
    } else {
      descModel =
        'Pick a model to download. ' +
        RECOMMENDED +
        ' is recommended for most Macs.' +
        SETTINGS_MODELS_HINT;
      modelGrid.ensureListeners();
      if (!modelGridShown || refreshModels) {
        modelGridShown = true;
        modelGrid.refresh(true);
      }
    }

    submitDisabled = !(status.microphone && status.accessibility && status.model);
  }

  function updateRelaunchHints(micDone: boolean, accDone: boolean) {
    const showMicRelaunch = !micDone && micSettingsOpened;
    micRelaunchHidden = !showMicRelaunch;
    btnQuitMicHidden = !showMicRelaunch;

    const isDev = !!status.is_dev;
    const unbundled = isDev && !status.is_bundled;
    const showAcc = !accDone;
    accRelaunchHidden = !showAcc || isDev;
    accDevHidden = !showAcc || !isDev;
    accPathHidden = !showAcc || !status.exe_path;
    btnQuitAccHidden = !showAcc;

    if (showAcc && unbundled) {
      const owner = status.responsible_app || 'the terminal app that launched it';
      accDevText =
        'This is a bare executable, so macOS assigns Accessibility to ' +
        owner +
        ' rather than to SpeakType. ' +
        'Run "make dev" instead — it launches a real SpeakType Dev app that holds its own grant.';
    } else if (showAcc && isDev) {
      accDevText =
        'Dev build: enable "' +
        (status.responsible_app || 'SpeakType Dev') +
        '" in the Accessibility list, ' +
        'then Quit & Reopen. Remove any older entry with the same name first.';
    }

    if (showAcc && status.exe_path) {
      const base = status.responsible_app_path
        ? 'Grant to: ' + status.responsible_app_path
        : status.exe_path;
      accPathText = String(base);
    }
  }

  let micRelaunchHidden = true;
  let btnQuitMicHidden = true;
  let accRelaunchHidden = true;
  let accDevHidden = true;
  let accDevText = '';
  let accPathHidden = true;
  let accPathText = '';
  let btnQuitAccHidden = true;

  function setStep(el: HTMLElement, state: string) {
    el.classList.remove('locked', 'active', 'done');
    el.classList.add(state);
  }

  async function poll(forceRender: boolean) {
    try {
      const s = await getOnboardingStatus();
      if (s) {
        const sk = statusKey(s);
        if (forceRender || sk !== lastStatusKey) {
          lastStatusKey = sk;
          status = s;
          render(forceRender);
        }
      }
    } catch (e) {
      /* ignore */
    }
  }

  function openMicSettings() {
    micSettingsOpened = true;
    updateRelaunchHints(status.microphone, status.accessibility);
    openSystemPane('com.apple.preference.security?Privacy_Microphone');
  }

  function openAccSettings() {
    accSettingsOpened = true;
    updateRelaunchHints(status.microphone, status.accessibility);
    openSystemPane('com.apple.preference.security?Privacy_Accessibility');
  }

  function quitForRelaunch() {
    quitApp();
  }

  async function start() {
    submitDisabled = true;
    submitting = true;
    try {
      await finishOnboarding();
    } catch (e) {
      console.error(e);
    }
  }

  let interval: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    poll(true);
    interval = setInterval(() => poll(false), 2000);
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      document.removeEventListener('visibilitychange', onVisibility);
      if (interval) clearInterval(interval);
    };
  });

  function onVisibility() {
    if (!document.hidden) poll(true);
  }
</script>

<div class="onboarding">
  <div class="header">
    <div class="header-title">
      <div class="logo-wrap">
        <svg class="logo" viewBox="0 0 24 24" fill="currentColor">
          <path
            d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z"
          />
          <path
            d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z"
          />
        </svg>
      </div>
      <div class="header-text">
        <h1>Welcome to SpeakType</h1>
        <p>Three quick steps to get you set up.</p>
      </div>
    </div>
  </div>

  <div class="main">
    <div class="col-left">
      <div class="widget-hint">
        <div class="widget-hint-visual">
          <svg viewBox="0 0 24 24" fill="currentColor">
            <path
              d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z"
            />
            <path
              d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z"
            />
          </svg>
        </div>
        <p class="widget-hint-text">
          The <strong>pulsing orange mic bubble</strong> on your screen is SpeakType. Complete the
          steps below, then click it to start dictating.
        </p>
      </div>

      <div class="steps-scroll" id="steps-scroll">
        <div class="steps">
          <div class="step" id="step-mic">
            <div class="step-body">
              <div class="step-title">
                <svg class="step-check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M4.5 12.75l6 6 9-13.5" /></svg>
                <span>Microphone access</span>
              </div>
              <div class="step-desc" id="desc-mic">{descMic}</div>
              <p class="step-relaunch-hint" id="hint-mic-relaunch" hidden={micRelaunchHidden}>
                Microphone access isn't active yet. If you just enabled it in System Settings, quit and
                reopen SpeakType once.
              </p>
              <div class="step-actions">
                <button class="btn settings" id="btn-mic-settings" onclick={openMicSettings}>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" /><polyline points="15 3 21 3 21 9" /><line x1="10" y1="14" x2="21" y2="3" /></svg>
                  Open System Settings
                </button>
                <button class="btn quit" id="btn-quit-mic" hidden={btnQuitMicHidden} onclick={quitForRelaunch}>
                  Quit SpeakType
                </button>
              </div>
            </div>
          </div>

          <div class="step locked" id="step-acc">
            <div class="step-body">
              <div class="step-title">
                <svg class="step-check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M4.5 12.75l6 6 9-13.5" /></svg>
                <span>Accessibility access</span>
              </div>
              <div class="step-desc" id="desc-acc">{descAcc}</div>
              <p class="step-relaunch-hint" id="hint-acc-relaunch" hidden={accRelaunchHidden}>
                System Settings can show SpeakType ON for an old or debug copy while this app is still
                blocked. Remove SpeakType from the Accessibility list, quit every SpeakType process, open
                only /Applications/SpeakType.app, turn it ON, then use Quit &amp; Reopen once.
              </p>
              <p class="step-relaunch-hint" id="hint-acc-dev" hidden={accDevHidden}>{accDevText}</p>
              <p class="step-exe-path" id="hint-acc-path" hidden={accPathHidden}>{accPathText}</p>
              <div class="step-actions">
                <button class="btn settings" id="btn-acc-settings" onclick={openAccSettings}>
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" /><polyline points="15 3 21 3 21 9" /><line x1="10" y1="14" x2="21" y2="3" /></svg>
                  Open System Settings
                </button>
                <button class="btn quit" id="btn-quit-acc" hidden={btnQuitAccHidden} onclick={quitForRelaunch}>
                  Quit &amp; Reopen SpeakType
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="col-right">
      <div class="model-panel" class:locked={modelStepState === 'locked'} class:done={modelStepState === 'done'}>
        <div class="step-title">
          <svg class="step-check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M4.5 12.75l6 6 9-13.5" /></svg>
          <span>Download a speech model</span>
        </div>
        <div class="step-desc" id="desc-model">{descModel}</div>
        {#if modelStepState === 'done'}
          <div class="panel-done-note">
            <svg class="step-check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M4.5 12.75l6 6 9-13.5" /></svg>
            <span>All set — you’re ready to start.</span>
          </div>
        {:else}
          <div class="model-grid-host scrollable" id="model-grid-host">
            <ModelGrid
              bind:this={modelGrid}
              mode="onboarding"
              recommendedModel={RECOMMENDED}
              getActiveModel={(): string =>
                status.model ? status.active_model || '' : ''
              }
              hooks={{
                onReady: () => poll(true),
                onError: (message) => {
                  descModel =
                    (message || 'Model operation failed') + ' — try again.';
                  modelGrid.refresh(true);
                  render();
                },
                onPaused: () => {
                  modelGrid.refresh(true);
                  poll(true);
                },
              }}
            />
          </div>
        {/if}
      </div>
    </div>
  </div>

  <div class="footer">
    <button
      class="start-btn"
      id="start-btn"
      disabled={submitDisabled}
      onclick={start}
    >
      {#if submitting}<span class="spinner"></span> Starting…{:else}Start Using SpeakType{/if}
    </button>
  </div>
</div>


<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }
  :global(html),
  :global(body) {
    height: 100%;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    background: var(--bg-elevated);
    color: var(--text);
    overflow: hidden;
  }
  :global(#app) {
    height: 100%;
  }

  .onboarding {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 22px 24px 18px;
    box-sizing: border-box;
    overflow: hidden;
  }
  .main {
    display: flex;
    flex-direction: row;
    gap: 16px;
    flex: 1;
    min-height: 0;
  }
  .col-left {
    display: flex;
    flex-direction: column;
    flex: 1.1;
    min-width: 0;
    min-height: 0;
  }
  .col-right {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }

  .header {
    text-align: left;
    margin-bottom: 16px;
    flex-shrink: 0;
    display: flex;
    justify-content: center;
  }
  .header-title {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .header .logo-wrap {
    width: 44px;
    height: 44px;
    flex-shrink: 0;
    background: var(--logo-grad);
    border: 1px solid var(--widget-panel-border);
    border-radius: 11px;
    box-shadow: inset 0 1px 0 var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .header .logo {
    width: 22px;
    height: 22px;
    color: var(--text-strong);
  }
  .header h1 {
    font-size: 18px;
    font-weight: 650;
    letter-spacing: -0.01em;
    color: var(--text);
  }
  .header p {
    font-size: 12.5px;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .widget-hint {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    margin-bottom: 16px;
    background: linear-gradient(135deg, rgba(239, 100, 60, 0.12) 0%, rgba(239, 100, 60, 0.04) 100%);
    border: 1px solid rgba(239, 100, 60, 0.28);
    border-radius: 12px;
    flex-shrink: 0;
    overflow: visible;
  }
  .widget-hint-visual {
    position: relative;
    flex-shrink: 0;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: var(--widget-idle);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 0 14px rgba(255, 110, 50, 0.35);
  }
  .widget-hint-visual::before,
  .widget-hint-visual::after {
    content: '';
    position: absolute;
    inset: -1px;
    border-radius: 50%;
    border: 2px solid rgba(239, 100, 60, 0.65);
    will-change: transform, opacity;
    animation: hintRipple 2.4s cubic-bezier(0.22, 1, 0.36, 1) infinite;
    pointer-events: none;
  }
  .widget-hint-visual::after {
    animation-delay: 1.2s;
  }
  .widget-hint-visual svg {
    position: relative;
    z-index: 1;
    width: 20px;
    height: 20px;
    color: rgba(255, 255, 255, 0.9);
  }
  @keyframes hintRipple {
    0% {
      transform: scale(1);
      opacity: 0.8;
    }
    100% {
      transform: scale(1.75);
      opacity: 0;
    }
  }
  .widget-hint-text {
    font-size: 13px;
    line-height: 1.45;
    color: #c9b0a4;
  }
  .widget-hint-text strong {
    color: #ffe8dc;
    font-weight: 600;
  }

  .steps-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    -webkit-overflow-scrolling: touch;
    margin: 0 -4px;
    padding: 0 4px;
  }
  :global(.steps-scroll::-webkit-scrollbar) {
    width: 6px;
  }
  :global(.steps-scroll::-webkit-scrollbar-track) {
    background: transparent;
  }
  :global(.steps-scroll::-webkit-scrollbar-thumb) {
    background: var(--scrollbar);
    border-radius: 3px;
  }
  :global(.steps-scroll::-webkit-scrollbar-thumb:hover) {
    background: var(--scrollbar-hover);
  }

  .steps {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  :global(.step) {
    background: var(--surface);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    padding: 14px;
    flex-shrink: 0;
    transition: opacity 0.2s, border-color 0.2s, background 0.2s;
  }
  :global(.step.active) {
    border-color: var(--accent-border);
    background: linear-gradient(160deg, var(--surface-3) 0%, var(--surface) 100%);
    box-shadow: inset 0 1px 0 var(--border-subtle);
  }
  :global(.step.locked) {
    opacity: 0.42;
    pointer-events: none;
  }
  :global(.step.done) {
    border-color: var(--success-border);
    pointer-events: none;
  }

  .step-body {
    min-width: 0;
  }
  :global(.step-title) {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 600;
  }
  :global(.step-check) {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
    display: none;
    color: var(--success);
  }
  :global(.step.done .step-check) {
    display: block;
  }
  :global(.step-desc) {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 2px;
    line-height: 1.45;
  }
  :global(.step.done .step-desc) {
    color: var(--success-alt);
  }

  :global(.step-actions) {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
  }
  :global(.step.done .step-actions),
  :global(.step.locked .step-actions) {
    display: none;
  }

  :global(.btn) {
    border: none;
    border-radius: 7px;
    padding: 7px 13px;
    font-size: 12.5px;
    font-weight: 550;
    cursor: pointer;
    font-family: inherit;
    transition: background 0.15s;
  }
  :global(.btn.settings) {
    width: 100%;
    background: var(--accent);
    color: var(--on-accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }
  :global(.btn.settings:hover) {
    background: var(--accent);
    filter: brightness(1.08);
  }
  :global(.btn.settings svg) {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
    opacity: 0.95;
  }

  :global(.step-relaunch-hint) {
    font-size: 11.5px;
    color: var(--warning);
    margin-top: 8px;
    line-height: 1.45;
    padding: 8px 10px;
    background: var(--warning-soft);
    border: 1px solid var(--warning-alt);
    border-radius: 7px;
  }

  :global(.step-exe-path) {
    font-size: 10.5px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: var(--text-muted);
    margin-top: 6px;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-all;
    user-select: text;
    -webkit-user-select: text;
  }

  :global(.btn.quit) {
    width: 100%;
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
  }
  :global(.btn.quit:hover) {
    background: var(--widget-btn-bg);
    color: var(--text);
  }

  .model-panel {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    padding: 16px;
    transition: opacity 0.2s, border-color 0.2s, background 0.2s;
  }
  .model-panel.locked {
    opacity: 0.6;
  }
  .model-panel.done {
    border-color: var(--success-border);
  }
  .model-panel .step-title {
    font-size: 15px;
  }
  .model-panel.done .step-title .step-check {
    display: block;
    color: var(--success);
  }
  .model-panel .step-desc {
    margin-top: 4px;
    margin-bottom: 12px;
  }
  .model-panel .model-grid-host {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    -webkit-overflow-scrolling: touch;
    padding-right: 2px;
    margin: 0 -4px;
    padding-left: 4px;
  }
  .model-panel .model-grid-host::-webkit-scrollbar {
    width: 6px;
  }
  .model-panel .model-grid-host::-webkit-scrollbar-track {
    background: transparent;
  }
  .model-panel .model-grid-host::-webkit-scrollbar-thumb {
    background: var(--scrollbar);
    border-radius: 3px;
  }
  .model-panel .model-grid-host::-webkit-scrollbar-thumb:hover {
    background: var(--scrollbar-hover);
  }
  .panel-done-note {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 14px;
    padding: 12px 14px;
    background: var(--success-soft);
    border: 1px solid var(--success-border);
    border-radius: 8px;
    color: var(--success-alt);
    font-size: 12.5px;
  }
  .panel-done-note svg {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
    color: var(--success);
    display: block;
  }

  .footer {
    margin-top: 16px;
    flex-shrink: 0;
  }
  .start-btn {
    width: 100%;
    padding: 12px;
    border: none;
    border-radius: 9px;
    background: var(--widget-btn-bg);
    color: #666;
    font-size: 14px;
    font-weight: 650;
    cursor: default;
    font-family: inherit;
  }
  .start-btn:not(:disabled) {
    background: var(--accent);
    color: var(--on-accent);
    cursor: pointer;
  }
  .start-btn:not(:disabled):hover {
    background: var(--accent);
    filter: brightness(1.08);
  }

  .spinner {
    width: 13px;
    height: 13px;
    border: 2px solid rgba(255, 255, 255, 0.25);
    border-top-color: #fff;
    border-radius: 50%;
    display: inline-block;
    animation: spin 0.7s linear infinite;
    vertical-align: -2px;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>