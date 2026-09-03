<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getSettings,
    saveSettings,
    checkServer,
    getServerStatus,
    stopWhisperServer,
    restartWhisperServer,
    getInputDevices,
    checkPermissions,
    requestAccessibility,
    openSystemPane,
    openModelsFolder,
    openRecordingsFolder,
    resetWidgetPosition,
    openAbout,
    emitEvent,
    listenEvent,
    type Settings,
  } from '../lib/ipc';
  import {
    formatHotkey,
    normalizeHotkey,
    isModifierChordHotkey,
    isModifierKey,
    isAcceptedHotkey,
    captureHotkeyModifiers,
    keyFromKeyboardEvent,
  } from '../lib/format';
  import { applyTheme } from '../lib/theme';
  import { computeDirty, defaultSnapshot, type FormSnapshot, type FormValues } from '../lib/stores';
  import ModelGrid from '../lib/ModelGrid.svelte';
  import InfoPopover from '../lib/InfoPopover.svelte';
  import MicTest from './MicTest.svelte';
import HistoryTab from './HistoryTab.svelte';
import DebugTab from './DebugTab.svelte';
import * as Select from '$lib/ui/select/index.js';
import SettingsFooter from '$lib/SettingsFooter.svelte';

  let historyRef: HistoryTab;
  let debugRef: DebugTab;

  // ---- Tab shell state ----
  type TabName = 'hotkey' | 'server' | 'general' | 'models' | 'history' | 'debug';
  let activeTab = $state<TabName>('hotkey');

  const TABS: { id: TabName; title: string; desc: string }[] = [
    { id: 'hotkey', title: 'Hotkey', desc: 'Choose the key combination that starts and stops dictation.' },
    { id: 'server', title: 'Server', desc: 'Point SpeakType at a transcription endpoint and pick a language.' },
    { id: 'general', title: 'General', desc: 'Appearance, permissions, and what happens after a transcription.' },
    { id: 'models', title: 'Models', desc: 'Download, resume, and switch between Whisper models.' },
    { id: 'history', title: 'History', desc: 'Replay, copy, reprocess, or delete past transcriptions.' },
    { id: 'debug', title: 'Debug', desc: 'Inspect how each recording was chunked and decoded.' },
  ];
  const meta = $derived(TABS.find((t) => t.id === activeTab) || TABS[0]);

  let paneBodyEl: HTMLDivElement;

  // ---- Settings state ----
  let loadedSnapshot: FormSnapshot = defaultSnapshot();
  let saveBtn: HTMLButtonElement;
  let saveLabel = $state('Save');
  let saveBusy = $state(false);
  let resetPositionText = $state('Reset Widget Position');

  // Form bound values (hotkey is special-cased).
  let hotkeyState = $state('');
  let hotkeyCaptured = $state('');
  let capturing = $state(false);
  let superControlPending = false;

  let apiUrl = $state('');
  let themePref = $state<'auto' | 'light' | 'dark'>('auto');
  let languageFilter = $state('');
  let languageValue = $state('auto');
  let pasteMode = $state('active');
  let postPasteKeysSelect = $state('enter');
  let postPasteKeysCustom = $state('');
  let saveRecordings = $state(false);
  let hallucinationGuard = $state(true);
  let inputDevice = $state('');

  // Server
  let serverDotClass = $state('status-dot checking');
  let serverDotTitle = $state('Checking...');
  let serverCheckText = $state('');
  let serverControlsHidden = $state(true);
  let serverHint = $state('');
  let stopDisabled = $state(true);
  let restartLabel = $state('Restart Server');
  let serverBusy = $state(false);
  let checkBusy = $state(false);
  let checkLabel = $state('Check Connection');

  // Permissions
  let permMicClass = $state('permission-status');
  let permMicText = $state('Checking...');
  let openMicHidden = $state(true);
  let permAccClass = $state('permission-status');
  let permAccText = $state('Checking...');
  let requestAccHidden = $state(true);
  let openAccHidden = $state(true);

  // Input devices / mic test
  let inputDevices: { name: string; label: string; is_default: boolean }[] = $state([]);
  let micPermission: boolean | null = $state(null);

  // Models
  let modelGrid: ModelGrid;
  let modelSelectEl = $state<HTMLSelectElement | null>(null);
  let modelStatusHidden = $state(true);
  let modelStatusClass = $state('');
  let modelStatusHtml = $state('');
  let downloadCount = $state(0);
  let modelStatusTimer: number | null = null;

  // Permission polling
  let permissionPollTimer: number | null = null;

  // ---- Languages ----
  const WHISPER_LANGUAGES = [
    { code: 'auto', name: 'Auto-detect' }, { code: 'af', name: 'Afrikaans' }, { code: 'am', name: 'Amharic' },
    { code: 'ar', name: 'Arabic' }, { code: 'as', name: 'Assamese' }, { code: 'az', name: 'Azerbaijani' },
    { code: 'ba', name: 'Bashkir' }, { code: 'be', name: 'Belarusian' }, { code: 'bg', name: 'Bulgarian' },
    { code: 'bn', name: 'Bengali' }, { code: 'bo', name: 'Tibetan' }, { code: 'br', name: 'Breton' },
    { code: 'bs', name: 'Bosnian' }, { code: 'ca', name: 'Catalan' }, { code: 'cs', name: 'Czech' },
    { code: 'cy', name: 'Welsh' }, { code: 'da', name: 'Danish' }, { code: 'de', name: 'German' },
    { code: 'el', name: 'Greek' }, { code: 'en', name: 'English' }, { code: 'es', name: 'Spanish' },
    { code: 'et', name: 'Estonian' }, { code: 'eu', name: 'Basque' }, { code: 'fa', name: 'Persian' },
    { code: 'fi', name: 'Finnish' }, { code: 'fo', name: 'Faroese' }, { code: 'fr', name: 'French' },
    { code: 'gl', name: 'Galician' }, { code: 'gu', name: 'Gujarati' }, { code: 'ha', name: 'Hausa' },
    { code: 'haw', name: 'Hawaiian' }, { code: 'he', name: 'Hebrew' }, { code: 'hi', name: 'Hindi' },
    { code: 'hr', name: 'Croatian' }, { code: 'ht', name: 'Haitian' }, { code: 'hu', name: 'Hungarian' },
    { code: 'hy', name: 'Armenian' }, { code: 'id', name: 'Indonesian' }, { code: 'is', name: 'Icelandic' },
    { code: 'it', name: 'Italian' }, { code: 'ja', name: 'Japanese' }, { code: 'jw', name: 'Javanese' },
    { code: 'ka', name: 'Georgian' }, { code: 'kk', name: 'Kazakh' }, { code: 'km', name: 'Khmer' },
    { code: 'kn', name: 'Kannada' }, { code: 'ko', name: 'Korean' }, { code: 'la', name: 'Latin' },
    { code: 'lb', name: 'Luxembourgish' }, { code: 'ln', name: 'Lingala' }, { code: 'lo', name: 'Lao' },
    { code: 'lt', name: 'Lithuanian' }, { code: 'lv', name: 'Latvian' }, { code: 'mg', name: 'Malagasy' },
    { code: 'mi', name: 'Maori' }, { code: 'mk', name: 'Macedonian' }, { code: 'ml', name: 'Malayalam' },
    { code: 'mn', name: 'Mongolian' }, { code: 'mr', name: 'Marathi' }, { code: 'ms', name: 'Malay' },
    { code: 'mt', name: 'Maltese' }, { code: 'my', name: 'Myanmar (Burmese)' }, { code: 'ne', name: 'Nepali' },
    { code: 'nl', name: 'Dutch' }, { code: 'nn', name: 'Norwegian Nynorsk' }, { code: 'no', name: 'Norwegian' },
    { code: 'oc', name: 'Occitan' }, { code: 'pa', name: 'Punjabi' }, { code: 'pl', name: 'Polish' },
    { code: 'ps', name: 'Pashto' }, { code: 'pt', name: 'Portuguese' }, { code: 'ro', name: 'Romanian' },
    { code: 'ru', name: 'Russian' }, { code: 'sa', name: 'Sanskrit' }, { code: 'sd', name: 'Sindhi' },
    { code: 'si', name: 'Sinhala' }, { code: 'sk', name: 'Slovak' }, { code: 'sl', name: 'Slovenian' },
    { code: 'sn', name: 'Shona' }, { code: 'so', name: 'Somali' }, { code: 'sq', name: 'Albanian' },
    { code: 'sr', name: 'Serbian' }, { code: 'su', name: 'Sundanese' }, { code: 'sv', name: 'Swedish' },
    { code: 'sw', name: 'Swahili' }, { code: 'ta', name: 'Tamil' }, { code: 'te', name: 'Telugu' },
    { code: 'tg', name: 'Tajik' }, { code: 'th', name: 'Thai' }, { code: 'tk', name: 'Turkmen' },
    { code: 'tl', name: 'Tagalog' }, { code: 'tr', name: 'Turkish' }, { code: 'tt', name: 'Tatar' },
    { code: 'uk', name: 'Ukrainian' }, { code: 'ur', name: 'Urdu' }, { code: 'uz', name: 'Uzbek' },
    { code: 'vi', name: 'Vietnamese' }, { code: 'yi', name: 'Yiddish' }, { code: 'yo', name: 'Yoruba' },
    { code: 'zh', name: 'Chinese' },
  ];
  const filteredLanguages = $derived(
    languageFilter ? WHISPER_LANGUAGES.filter((l) => l.name.toLowerCase().includes(languageFilter.toLowerCase()) || l.code.includes(languageFilter.toLowerCase())) : WHISPER_LANGUAGES,
  );
  const languageDisplay = $derived(
    filteredLanguages.map((l) => ({
      ...l,
      label: l.code === 'auto' ? l.name : `${l.name} (${l.code})`,
    })),
  );
  let languageFilterTimer: number | null = null;

  const POST_PASTE_PRESETS = [
    { value: '', label: 'None' },
    { value: 'enter', label: 'Enter' },
    { value: 'tab', label: 'Tab' },
    { value: 'enter+enter', label: 'Enter + Enter' },
    { value: 'space', label: 'Space' },
  ];
  const MODEL_PRESETS = ['tiny.en', 'tiny', 'base.en', 'base', 'small.en', 'small', 'medium.en', 'medium', 'large-v3', 'large-v3-turbo'];

  // ---- Derived current form values ----
  function currentForm(): FormValues {
    return {
      hotkey: hotkeyCaptured || loadedSnapshot.hotkey || '',
      api_url: apiUrl,
      model: modelSelectEl?.value || loadedSnapshot.model || '',
      language: languageValue,
      paste_mode: pasteMode,
      post_paste_keys: getPostPasteKeys(),
      save_recordings: saveRecordings,
      hallucination_guard: hallucinationGuard,
      theme: themePref,
      input_device: inputDevice,
    };
  }

  const dirty = $derived.by(() => computeDirty(currentForm(), loadedSnapshot));

  function getPostPasteKeys(): string {
    const custom = postPasteKeysCustom.trim();
    if (custom) return custom;
    return postPasteKeysSelect;
  }

  function setHotkeyDisplay(text: string) {
    hotkeyState = text || '(not set)';
  }
  function formatCapturedHotkey(hk: string) {
    return hk ? formatHotkey(hk) : '';
  }

  const captureLabel = $derived.by(() => {
    if (capturing) return 'Press keys now...';
    return formatCapturedHotkey(hotkeyCaptured) || 'Click to capture...';
  });

  function updateCaptureFeedback(parts: string[]) {
    if (!capturing || parts.length === 0) return;
    if (isModifierChordHotkey(parts)) {
      hotkeyCapturedByFeedback = formatHotkey(parts.join('+')) + ' (release to confirm)…';
    } else if (parts.every(isModifierKey)) {
      hotkeyCapturedByFeedback = formatHotkey(parts.join('+')) + ' + key…';
    } else {
      hotkeyCapturedByFeedback = formatHotkey(parts.join('+')) + '…';
    }
  }
  let hotkeyCapturedByFeedback = $state('');
  const captureText = $derived(capturing ? hotkeyCapturedByFeedback || 'Press keys now...' : undefined);

  function finishHotkeyCapture(hotkey: string) {
    hotkeyCaptured = normalizeHotkey(hotkey);
    capturing = false;
    superControlPending = false;
    hotkeyCapturedByFeedback = '';
    setHotkeyDisplay(formatCapturedHotkey(hotkeyCaptured));
  }

  function onCaptureClick() {
    if (capturing) {
      capturing = false;
      superControlPending = false;
      hotkeyCapturedByFeedback = '';
      return;
    }
    capturing = true;
    hotkeyCaptured = '';
    superControlPending = false;
    hotkeyCapturedByFeedback = '';
  }

  function onKeyDown(e: KeyboardEvent) {
    if (!capturing) return;
    if (e.key === 'Escape') {
      capturing = false;
      hotkeyCaptured = '';
      superControlPending = false;
      hotkeyCapturedByFeedback = '';
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    const parts = captureHotkeyModifiers(e);
    if (e.metaKey && e.ctrlKey) {
      superControlPending = true;
      updateCaptureFeedback(['Super', 'Control']);
      return;
    }
    superControlPending = false;
    const modKeys = ['Control', 'Shift', 'Alt', 'Meta'];
    if (!modKeys.includes(e.key)) {
      const key = keyFromKeyboardEvent(e);
      if (key) {
        parts.push(key);
        if (isAcceptedHotkey(parts)) {
          finishHotkeyCapture(parts.join('+'));
          return;
        }
      }
    }
    updateCaptureFeedback(parts);
  }

  function onKeyUp(e: KeyboardEvent) {
    if (!capturing || !superControlPending) return;
    e.preventDefault();
    e.stopPropagation();
    if (!e.metaKey && !e.ctrlKey) finishHotkeyCapture('Super+Control');
  }

  function setPreset(hotkey: string) {
    hotkeyCaptured = normalizeHotkey(hotkey);
    setHotkeyDisplay(formatCapturedHotkey(hotkeyCaptured));
    capturing = false;
    hotkeyCapturedByFeedback = '';
  }

  // ---- Server ----
  async function runCheckServer(url?: string) {
    if (checkBusy) return;
    checkBusy = true;
    checkLabel = 'Checking…';
    serverDotClass = 'status-dot checking';
    serverDotTitle = 'Checking...';
    serverCheckText = 'Checking…';
    try {
      const result = await checkServer(url || apiUrl);
      if (result && result.status === 'connected') {
        serverDotClass = 'status-dot connected';
        serverDotTitle = 'Connected';
        serverCheckText = result.message || 'Connected';
      } else {
        serverDotClass = 'status-dot disconnected';
        serverDotTitle = (result && result.message) || 'Disconnected';
        serverCheckText = result?.message || 'Disconnected';
      }
      checkLabel = 'Check Connection';
      return result;
    } catch (e) {
      serverDotClass = 'status-dot disconnected';
      serverDotTitle = 'Error: ' + e;
      serverCheckText = 'Error: ' + e;
      checkLabel = 'Check Connection';
      return null;
    } finally {
      checkBusy = false;
    }
  }

  async function refreshServerControls() {
    try {
      const status = await getServerStatus();
      if (!status || !status.embedded) {
        serverControlsHidden = true;
        serverHint = 'Server controls are only available in builds with the bundled whisper sidecar.';
        return;
      }
      serverControlsHidden = false;
      stopDisabled = !status.running;
      serverHint = status.running
        ? `Whisper server running on port ${status.port}. Stop or restart the sidecar below.`
        : 'Whisper server is stopped. Restart to load the active model.';
      await runCheckServer(apiUrl);
    } catch (e) {
      console.error('Failed to get server status:', e);
    }
  }

  async function onStopServer() {
    stopDisabled = true;
    try {
      await stopWhisperServer();
      await refreshServerControls();
    } catch (e) {
      console.error('Failed to stop server:', e);
      await refreshServerControls();
    }
  }

  async function onRestartServer() {
    serverBusy = true;
    restartLabel = 'Restarting…';
    try {
      const result = await restartWhisperServer();
      if (result && result.api_url) {
        apiUrl = result.api_url;
        loadedSnapshot.api_url = result.api_url;
      }
      await refreshServerControls();
    } catch (e) {
      console.error('Failed to restart server:', e);
      await refreshServerControls();
    } finally {
      serverBusy = false;
      restartLabel = 'Restart Server';
      await refreshServerControls();
    }
  }

  // ---- Input devices ----
  async function populateInputDevices(selected?: string) {
    try {
      const data = await getInputDevices();
      const devices = (data && data.devices) || [];
      const current = inputDevice;
      inputDevices = devices;
      let value = current;
      if (value && !devices.some((d) => d.name === value)) value = '';
      if (!value && selected && devices.some((d) => d.name === selected)) value = selected;
      if (!value && data?.default && devices.some((d) => d.name === data.default)) value = data.default || '';
      inputDevice = value;
    } catch (e) {
      console.error('Failed to load input devices:', e);
    }
  }

  // ---- Permissions ----
  async function runCheckPermissions() {
    try {
      const result = await checkPermissions();
      if (!result) return;
      micPermission = !!result.microphone;
      populateInputDevices();
      if (result.microphone) {
        permMicText = 'Granted';
        permMicClass = 'permission-status granted';
        openMicHidden = true;
      } else {
        permMicText = 'Not Granted';
        permMicClass = 'permission-status denied';
        openMicHidden = false;
      }
      if (result.accessibility) {
        permAccText = 'Granted';
        permAccClass = 'permission-status granted';
        requestAccHidden = true;
        openAccHidden = true;
      } else {
        permAccText = 'Not Granted';
        permAccClass = 'permission-status denied';
        requestAccHidden = false;
        openAccHidden = false;
      }
    } catch (e) {
      console.error('Failed to check permissions:', e);
    }
  }

  function syncPermissionPoll() {
    const wanted = activeTab === 'general' && document.hasFocus() && !document.hidden;
    if (wanted && !permissionPollTimer) {
      permissionPollTimer = window.setInterval(runCheckPermissions, 3000);
    } else if (!wanted && permissionPollTimer) {
      clearInterval(permissionPollTimer);
      permissionPollTimer = null;
    }
  }

  // ---- Tab switching ----
  function switchTab(tab: TabName) {
    activeTab = tab;
    if (paneBodyEl) paneBodyEl.scrollTop = 0;
    syncPermissionPoll();
    if (tab === 'general') runCheckPermissions();
    if (tab === 'server') refreshServerControls();
    if (tab === 'models') {
      modelGrid?.ensureListeners();
      modelGrid?.refresh(true);
    }
  }

  // ---- Save / restore ----
  async function loadSettings() {
    try {
      const settings = await getSettings();
      if (!settings) return;
      setHotkeyDisplay(formatCapturedHotkey(settings.hotkey));
      apiUrl = settings.api_url || '';
      languageValue = settings.language || 'auto';
      pasteMode = settings.paste_mode || 'active';
      const ppk = settings.post_paste_keys || '';
      if (POST_PASTE_PRESETS.some((p) => p.value === ppk)) {
        postPasteKeysSelect = ppk;
        postPasteKeysCustom = '';
      } else if (ppk) {
        postPasteKeysSelect = '';
        postPasteKeysCustom = ppk;
      }
      saveRecordings = !!settings.save_recordings;
      hallucinationGuard = settings.hallucination_guard !== false;
      themePref = settings.theme as 'auto' | 'light' | 'dark';
      applyTheme(themePref);
      modelGrid?.setModelSelect(settings.model);
      await populateInputDevices(settings.input_device || '');
      loadedSnapshot = {
        hotkey: settings.hotkey || '',
        api_url: settings.api_url || '',
        model: settings.model || '',
        language: languageValue,
        paste_mode: pasteMode,
        post_paste_keys: ppk,
        save_recordings: saveRecordings,
        hallucination_guard: hallucinationGuard,
        theme: themePref,
        input_device: inputDevice,
      };
      runCheckServer(settings.api_url);
      refreshServerControls();
    } catch (e) {
      console.error('Failed to load settings:', e);
    }
  }

  async function onSave() {
    if (!dirty || saveBusy) return;
    saveBusy = true;
    try {
      const settings = await getSettings();
      if (!settings) {
        saveBusy = false;
        return;
      }
      if (hotkeyCaptured) settings.hotkey = normalizeHotkey(hotkeyCaptured);
      settings.api_url = apiUrl;
      settings.model = modelSelectEl?.value || '';
      settings.language = languageValue;
      settings.paste_mode = pasteMode;
      settings.post_paste_keys = getPostPasteKeys() || null;
      settings.save_recordings = saveRecordings;
      settings.hallucination_guard = hallucinationGuard;
      settings.theme = themePref;
      settings.input_device = inputDevice || null;

      const hotkeyChanged = settings.hotkey !== loadedSnapshot.hotkey;
      await saveSettings(settings);
      setHotkeyDisplay(formatCapturedHotkey(settings.hotkey));
      applyTheme(settings.theme);

      loadedSnapshot = {
        hotkey: settings.hotkey,
        api_url: settings.api_url,
        model: settings.model,
        language: settings.language,
        paste_mode: settings.paste_mode,
        post_paste_keys: settings.post_paste_keys || '',
        save_recordings: settings.save_recordings,
        hallucination_guard: settings.hallucination_guard,
        theme: settings.theme,
        input_device: settings.input_device || '',
      };
      hotkeyCaptured = '';
      saveLabel = hotkeyChanged ? 'Saved — hotkey active' : 'Saved!';
      setSaveGreen(true);
      setTimeout(() => {
        saveLabel = 'Save';
        setSaveGreen(false);
      }, 1500);
    } catch (e) {
      console.error('Failed to save settings:', e);
      saveLabel = 'Error!';
      setSaveGreen(false, true);
      setTimeout(() => {
        saveLabel = 'Save';
        setSaveGreen(false);
      }, 1500);
    } finally {
      saveBusy = false;
    }
  }

  function setSaveGreen(green: boolean, red = false) {
    if (saveBtn) {
      saveBtn.style.background = green
        ? '#16a34a'
        : red
          ? '#dc2626'
          : '';
      saveBtn.style.borderColor = saveBtn.style.background;
    }
  }

  function restoreDefaults() {
    const defaults = {
      hotkey: 'Super+Control',
      api_url: 'http://127.0.0.1:8002/inference',
      model: 'medium',
      language: 'auto',
      paste_mode: 'active',
      post_paste_keys: 'enter',
      save_recordings: false,
      hallucination_guard: true,
      theme: 'auto' as const,
    };
    hotkeyCaptured = defaults.hotkey;
    setHotkeyDisplay(formatCapturedHotkey(defaults.hotkey));
    apiUrl = defaults.api_url;
    modelGrid?.setModelSelect(defaults.model);
    languageValue = defaults.language;
    pasteMode = defaults.paste_mode;
    postPasteKeysSelect = defaults.post_paste_keys;
    postPasteKeysCustom = '';
    saveRecordings = defaults.save_recordings;
    hallucinationGuard = defaults.hallucination_guard;
    inputDevice = '';
    themePref = defaults.theme;
    applyTheme(defaults.theme);
  }

  // ---- Theme change ----
  function onThemeChange() {
    applyTheme(themePref);
    emitEvent('theme:changed', { theme: themePref });
  }

  // ---- Post-paste keys cross-clearing ----
  function onPostPasteCustomInput() {
    if (postPasteKeysCustom.trim()) postPasteKeysSelect = '';
  }
  function onPostPastePresetChange() {
    if (postPasteKeysSelect) postPasteKeysCustom = '';
  }

  // ---- Reset position ----
  async function onResetPosition() {
    try {
      await resetWidgetPosition();
      resetPositionText = 'Position Reset!';
      setTimeout(() => {
        resetPositionText = 'Reset Widget Position';
      }, 1500);
    } catch (e) {
      console.error('Failed to reset position:', e);
    }
  }

  // ---- Model status ----
  function setStatusLoading(text: string) {
    if (modelStatusTimer) clearTimeout(modelStatusTimer);
    modelStatusHidden = false;
    modelStatusClass = 'loading';
    modelStatusHtml = `<span class="spinner"></span><span class="status-text"></span>`;
    modelBusyText = text;
  }
  let modelBusyText = $state('');
  function showModelStatus(type: string, html: string) {
    modelStatusHidden = false;
    modelStatusClass = type;
    modelStatusHtml = html;
  }
  function hideModelStatus() {
    modelStatusHidden = true;
    modelStatusClass = '';
    if (modelStatusTimer) {
      clearTimeout(modelStatusTimer);
      modelStatusTimer = null;
    }
  }
  function scheduleHideModelStatus(ms: number) {
    if (modelStatusTimer) clearTimeout(modelStatusTimer);
    modelStatusTimer = window.setTimeout(hideModelStatus, ms);
  }

  function onModelSaved(model: string) {
    loadedSnapshot.model = model;
    if (modelSelectEl) modelSelectEl.value = model;
  }

  // ---- onMount wire-up ----
  let unlistenTheme: (() => void) | null = null;

  onMount(() => {
    listenEvent<{ theme?: string }>('theme:changed', (payload) => {
      const theme = payload && payload.theme;
      if (theme) applyTheme(theme);
    }).then((fn) => (unlistenTheme = fn));
    loadSettings();
    modelStatusHidden = true;

    const onFocus = () => {
      if (activeTab === 'general') runCheckPermissions();
      syncPermissionPoll();
    };
    const onBlur = syncPermissionPoll;
    const onVis = syncPermissionPoll;
    window.addEventListener('focus', onFocus);
    window.addEventListener('blur', onBlur);
    document.addEventListener('visibilitychange', onVis);

    return () => {
      unlistenTheme?.();
      if (permissionPollTimer) clearInterval(permissionPollTimer);
      window.removeEventListener('focus', onFocus);
      window.removeEventListener('blur', onBlur);
      document.removeEventListener('visibilitychange', onVis);
    };
  });
</script>

<svelte:head><title>SpeakType Settings</title></svelte:head>

<div class="settings">
  <nav class="sidebar">
    <div class="sidebar-brand">
      <span class="brand-mark">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true">
          <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z"/>
          <path d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z"/>
        </svg>
      </span>
      <span class="brand-text">
        <span class="brand-name">SpeakType</span>
        <span class="brand-version">v0.1.0</span>
      </span>
    </div>

    <div class="tabs">
      {#each TABS as tab}
        <button
          class="tab"
          class:active={activeTab === tab.id}
          data-tab={tab.id}
          title={tab.title}
          onclick={() => switchTab(tab.id)}
        >
          {#if tab.id === 'hotkey'}
            <svg class="tab-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="2" y="6" width="20" height="12" rx="2"/>
              <path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M8 14h8"/>
            </svg>
          {:else if tab.id === 'server'}
            <svg class="tab-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="3" y="4" width="18" height="7" rx="2"/>
              <rect x="3" y="13" width="18" height="7" rx="2"/>
              <path d="M7 7.5h.01M7 16.5h.01"/>
            </svg>
          {:else if tab.id === 'general'}
            <svg class="tab-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <circle cx="12" cy="12" r="3"/>
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-2.82 1.17V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 7.6 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 3.09 14H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 8.6a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.09V4a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1.51 1 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19 9.6V10a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
            </svg>
          {:else if tab.id === 'models'}
            <svg class="tab-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M12 2l9 5-9 5-9-5 9-5z"/>
              <path d="M3 12l9 5 9-5M3 17l9 5 9-5"/>
            </svg>
          {:else if tab.id === 'history'}
            <svg class="tab-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M3 12a9 9 0 1 0 3-6.7L3 8"/>
              <path d="M3 3v5h5M12 7v5l3 2"/>
            </svg>
          {:else if tab.id === 'debug'}
            <svg class="tab-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M8 9l-2-2a1.5 1.5 0 0 1 2.5-1.5L10 7a2 2 0 0 1 4 0l1.5-1.5A1.5 1.5 0 0 1 18 7l-2 2"/>
              <path d="M12 5v14"/>
              <path d="M8 12h8"/>
              <path d="M12 12l-4 6"/>
              <path d="M12 12l4 6"/>
            </svg>
          {/if}
          <span class="tab-label">{tab.title}</span>
          {#if tab.id === 'models'}
            <span class="tab-badge" id="models-download-badge" aria-live="polite" aria-atomic="true" hidden={downloadCount === 0}>
              {downloadCount === 0 ? '' : downloadCount}
            </span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="sidebar-footer">
      <button class="info-btn" id="about-btn" title="About SpeakType" onclick={() => openAbout()}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>
        </svg>
        <span class="tab-label">About</span>
      </button>
    </div>
  </nav>

  <section class="pane">
    <header class="pane-header">
      <h1 class="pane-title" id="pane-title">{meta.title}</h1>
      <p class="pane-desc" id="pane-desc">{meta.desc}</p>
    </header>

    <div class="pane-body" bind:this={paneBodyEl}>
      <!-- HOTKEY -->
      <div class="tab-content" class:active={activeTab === 'hotkey'} id="tab-hotkey">
        <section class="card">
          <h2 class="card-title">Shortcut</h2>
          <div class="field">
            <span class="field-label">Current Hotkey</span>
            <div class="hotkey-display" id="current-hotkey">{hotkeyState}</div>
          </div>
          <div class="field">
            <label for="capture-btn">Press new hotkey combination</label>
            <button class="hotkey-capture" class:capturing={capturing} id="capture-btn" onclick={onCaptureClick}>
              {captureText || captureLabel}
            </button>
            <p class="hint">Any modifier + key combo works (e.g. Command+Option+L). Hold Command+Control and release for a modifier-only chord, or pick a function key preset.</p>
          </div>
          <div class="field">
            <span class="field-label">Or choose a preset</span>
            <div class="presets">
              <button class="preset" data-hotkey="Super+Control" onclick={() => setPreset('Super+Control')}>{formatHotkey('Super+Control')}</button>
              <button class="preset" data-hotkey="F9" onclick={() => setPreset('F9')}>F9</button>
              <button class="preset" data-hotkey="F8" onclick={() => setPreset('F8')}>F8</button>
            </div>
          </div>
        </section>
      </div>

      <!-- SERVER -->
      <div class="tab-content" class:active={activeTab === 'server'} id="tab-server">
        <section class="card">
          <h2 class="card-title">Server</h2>
          <div class="field">
            <label for="api-url">API URL</label>
            <div class="url-row">
              <input type="text" id="api-url" placeholder="http://127.0.0.1:8002/inference" bind:value={apiUrl}>
              <span class={serverDotClass} id="server-status" title={serverDotTitle}></span>
            </div>
            <p class="hint">Whisper-compatible transcription endpoint</p>
          </div>
          <div class="server-connection-row">
            <button class="btn" id="check-server-btn" onclick={() => runCheckServer(apiUrl)} disabled={checkBusy}>
              {#if checkBusy}<span class="btn-spinner" aria-hidden="true"></span>{/if}
              {checkLabel}
            </button>
            {#if serverCheckText}
              <span class="server-check-result" id="server-check-result">{serverCheckText}</span>
            {/if}
          </div>
          <div class="field server-manage">
            <span class="field-label">Local whisper sidecar</span>
            <div class="server-actions" id="server-actions" hidden={serverControlsHidden}>
              <button class="btn secondary" id="restart-server-btn" type="button" disabled={serverBusy} onclick={onRestartServer}>{restartLabel}</button>
              <button class="btn secondary" id="stop-server-btn" type="button" disabled={stopDisabled || serverBusy} onclick={onStopServer}>Stop Server</button>
            </div>
            <p class="hint" id="server-actions-hint">{serverHint}</p>
          </div>
        </section>
        <section class="card">
          <h2 class="card-title">Language</h2>
          <div class="field">
            <input type="text" id="language-search" placeholder="Search language..." class="input-filter" bind:value={languageFilter}>
            <Select.Root bind:value={languageValue}>
              <Select.Trigger aria-label="Select language">
                {languageDisplay.find(l => l.code === languageValue)?.label ?? 'Auto-detect'}
              </Select.Trigger>
              <Select.Content>
                {#each languageDisplay as l (l.code)}
                  <Select.Item value={l.code} label={l.label} />
                {/each}
              </Select.Content>
            </Select.Root>
            <p class="hint">Type to filter. Set to "Auto-detect" for multilingual. Language detection requires a multilingual model (not .en variants).</p>
          </div>
        </section>
      </div>

      <!-- GENERAL -->
      <div class="tab-content" class:active={activeTab === 'general'} id="tab-general">
        <section class="card">
          <h2 class="card-title" id="theme-label">Appearance</h2>
          <div class="field">
            <div class="segmented segmented-icons" role="radiogroup" aria-labelledby="theme-label">
              <input type="radio" name="theme-pref" id="theme-auto" value="auto" bind:group={themePref} onchange={onThemeChange}>
              <label class="segment" for="theme-auto" title="Auto">
                <svg class="theme-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <circle cx="12" cy="12" r="4"/><path d="M12 2a10 10 0 0 1 0 20V12z" fill="currentColor" opacity="0.35"/>
                </svg>
              </label>
              <input type="radio" name="theme-pref" id="theme-light" value="light" bind:group={themePref} onchange={onThemeChange}>
              <label class="segment" for="theme-light" title="Light">
                <svg class="theme-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
                  <circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>
                </svg>
              </label>
              <input type="radio" name="theme-pref" id="theme-dark" value="dark" bind:group={themePref} onchange={onThemeChange}>
              <label class="segment" for="theme-dark" title="Dark">
                <svg class="theme-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <path d="M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8z"/>
                </svg>
              </label>
            </div>
            <p class="hint">Auto follows macOS appearance.</p>
          </div>
        </section>

        <section class="card">
          <h2 class="card-title">Permissions</h2>
          <div class="field">
            <div class="permissions-grid">
              <div class="permission-cell">
                <div class="permission-row-top">
                  <span class="permission-icon" aria-hidden="true">
                    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                      <rect x="9" y="2" width="6" height="12" rx="3"/><path d="M5 10a7 7 0 0 0 14 0"/><line x1="12" y1="17" x2="12" y2="22"/><line x1="8" y1="21" x2="16" y2="21"/>
                    </svg>
                  </span>
                  <span class="permission-label">Microphone</span>
                  <span class={permMicClass} id="perm-mic">{permMicText}</span>
                </div>
                {#if !openMicHidden}
                  <button class="btn secondary btn-small open-settings-btn" id="open-mic-btn" onclick={() => openSystemPane('com.apple.preference.security?Privacy_Microphone')}>Open Settings</button>
                {/if}
              </div>
              <div class="permission-cell">
                <div class="permission-row-top">
                  <span class="permission-icon" aria-hidden="true">
                    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                      <circle cx="12" cy="12" r="9"/><circle cx="12" cy="9" r="1.6"/><path d="M6.5 15.5a6 6 0 0 1 11 0"/><line x1="12" y1="14" x2="12" y2="18.5"/>
                    </svg>
                  </span>
                  <span class="permission-label">Accessibility</span>
                  <span class={permAccClass} id="perm-acc">{permAccText}</span>
                </div>
                {#if !requestAccHidden || !openAccHidden}
                  <div class="permission-actions">
                    {#if !requestAccHidden}
                      <button class="btn secondary btn-small" id="request-acc-btn" onclick={requestAccessibility}>Prompt</button>
                    {/if}
                    {#if !openAccHidden}
                      <button class="btn secondary btn-small open-settings-btn" id="open-acc-btn" onclick={() => openSystemPane('com.apple.preference.security?Privacy_Accessibility')}>Open Settings</button>
                    {/if}
                  </div>
                {/if}
              </div>
            </div>
            <p class="hint">Microphone for recording. Accessibility to paste text into other apps.</p>
          </div>
        </section>

        <section class="card" id="mic-test-card" hidden={!micPermission}>
          <h2 class="card-title">Microphone Check</h2>
          <div class="field">
            <InfoPopover label="Input Device" title="About input devices">
              Pick the microphone to dictate with. Use the built-in mic or a wired input for the most reliable capture — Bluetooth headsets (16 kHz hands-free) are often unstable and can produce only a burst of sound.
            </InfoPopover>
            <Select.Root bind:value={inputDevice}>
              <Select.Trigger aria-label="Select input device">
                {inputDevice ? inputDevices.find(d => d.name === inputDevice)?.label ?? inputDevice : 'System default (follows Display Audio settings)'}
              </Select.Trigger>
              <Select.Content>
                <Select.Item value="" label="System default (follows Display Audio settings)" />
                {#each inputDevices as d (d.name)}
                  <Select.Item
                    value={d.name}
                    label={d.label + (d.is_default ? ' — current default' : '')}
                  />
                {/each}
              </Select.Content>
            </Select.Root>
          </div>
          <div class="field mic-test-field">
            {#if micPermission}
              <MicTest
                permitted={micPermission}
                sectionVisible={activeTab === 'general'}
                inputDeviceValue={inputDevice}
              />
            {/if}
            <p class="hint">Speak normally and watch the bars move. The test uses the input selected above and stops on its own when you leave this section.</p>
          </div>
        </section>

        <section class="card">
          <h2 class="card-title">Paste Behavior</h2>
          <div class="field">
            <label for="paste-mode-select">Destination Window</label>
            <Select.Root bind:value={pasteMode}>
              <Select.Trigger id="paste-mode-select" aria-label="Destination Window">
                {pasteMode === 'active' ? 'Paste to active window' : 'Paste to original window'}
              </Select.Trigger>
              <Select.Content>
                <Select.Item value="original" label="Paste to original window" />
                <Select.Item value="active" label="Paste to active window" />
              </Select.Content>
            </Select.Root>
            <p class="hint">Original: pastes to app you were in when recording started. Active: pastes to whichever app is focused when recording stops.</p>
          </div>
          <div class="field">
            <label for="post-paste-keys-select">Keys After Paste</label>
            <Select.Root bind:value={postPasteKeysSelect} onValueChange={(v) => { if (v) postPasteKeysCustom = ''; }}>
              <Select.Trigger id="post-paste-keys-select" aria-label="Keys After Paste">
                {POST_PASTE_PRESETS.find(p => p.value === postPasteKeysSelect)?.label ?? (postPasteKeysSelect || 'None')}
              </Select.Trigger>
              <Select.Content>
                {#each POST_PASTE_PRESETS as p (p.value)}
                  <Select.Item value={p.value} label={p.label} />
                {/each}
              </Select.Content>
            </Select.Root>
            <input type="text" id="post-paste-keys-custom" placeholder="Or type custom: e.g. enter+tab" style="margin-top: 6px" bind:value={postPasteKeysCustom} oninput={onPostPasteCustomInput}>
            <p class="hint">Keys to press automatically after text is pasted. Use + to chain keys (e.g., enter+tab).</p>
          </div>
        </section>

        <section class="card">
          <h2 class="card-title">Transcription</h2>
          <div class="field">
            <label class="checkbox-label">
              <input type="checkbox" id="save-recordings" bind:checked={saveRecordings}>
              Save recording audio
            </label>
            <p class="hint">Keep WAV files for each transcription so you can reprocess them from History. Stored locally in ~/Library/Application Support/speaktype/recordings/</p>
          </div>
          <div class="field">
            <label class="checkbox-label">
              <input type="checkbox" id="hallucination-guard" bind:checked={hallucinationGuard}>
              Filter hallucinated text
            </label>
            <p class="hint">Scores each transcription for invented text (silence transcribed as speech, repetition loops, low confidence), removes the bad parts, and re-runs the audio when a result looks wrong.</p>
          </div>
        </section>
      </div>

      <!-- MODELS -->
      <div class="tab-content" class:active={activeTab === 'models'} id="tab-models">
        <select id="model-select" style="display:none" bind:this={modelSelectEl}>
          {#each MODEL_PRESETS as m}
            <option value={m}>{m}</option>
          {/each}
        </select>
        <div id="model-status" class="model-status" class:loading={modelStatusClass === 'loading'} hidden={modelStatusHidden}>
          {#if modelStatusClass === 'loading'}
            <span class="spinner"></span><span class="status-text">{modelBusyText}</span>
          {:else if modelStatusHtml}
            <div class="status-inner" id="model-status-inner">{@html modelStatusHtml}</div>
          {/if}
        </div>
        <div class="model-grid-host">
          <ModelGrid
            bind:this={modelGrid}
            mode="settings"
            modelSelect={modelSelectEl}
            getActiveModel={() => modelSelectEl?.value || String(loadedSnapshot.model || '')}
            hooks={{
              onLoading: (text) => setStatusLoading(text),
              onHideStatus: () => hideModelStatus(),
              onStatus: (type, html) => showModelStatus(type, html),
              onScheduleHideStatus: (ms) => scheduleHideModelStatus(ms),
              onModelSaved: (model) => onModelSaved(model),
              onDownloadCount: (count) => { downloadCount = Number(count) || 0; },
            }}
          />
        </div>
      </div>

      <!-- HISTORY -->
      <div class="tab-content" class:active={activeTab === 'history'} id="tab-history">
        <p class="hint recordings-folder-hint">Play and seek saved recordings below each entry. Use ♪⌫ to remove audio; × removes the whole entry. Click twice to confirm deletes.</p>
        <HistoryTab bind:this={historyRef} active={activeTab === 'history'} />
      </div>

      <!-- DEBUG -->
      <div class="tab-content" class:active={activeTab === 'debug'} id="tab-debug">
        <p class="hint">Shows how each recording was split into chunks and how the model was driven per chunk — including every decode pass, retry temperature, and hallucination score.</p>
        <DebugTab bind:this={debugRef} active={activeTab === 'debug'} />
      </div>
    </div>

    <div class="pane-footer-strip">
      <SettingsFooter hidden={activeTab !== 'hotkey'}>
        {#snippet right()}
          <button class="btn secondary" id="restore-defaults-btn" onclick={restoreDefaults}>Restore Defaults</button>
          <button class="btn primary" class:disabled={!dirty} id="save-btn" disabled={!dirty || saveBusy} bind:this={saveBtn} onclick={onSave}>{saveLabel}</button>
        {/snippet}
      </SettingsFooter>

      <SettingsFooter hidden={activeTab !== 'server'}>
        {#snippet right()}
          <button class="btn secondary" id="restore-defaults-btn" onclick={restoreDefaults}>Restore Defaults</button>
          <button class="btn primary" class:disabled={!dirty} id="save-btn" disabled={!dirty || saveBusy} bind:this={saveBtn} onclick={onSave}>{saveLabel}</button>
        {/snippet}
      </SettingsFooter>

      <SettingsFooter hidden={activeTab !== 'general'}>
        {#snippet left()}
          <button class="btn secondary" id="reset-position-btn" onclick={onResetPosition}>{resetPositionText}</button>
        {/snippet}
        {#snippet right()}
          <button class="btn secondary" id="restore-defaults-btn" onclick={restoreDefaults}>Restore Defaults</button>
          <button class="btn primary" class:disabled={!dirty} id="save-btn" disabled={!dirty || saveBusy} bind:this={saveBtn} onclick={onSave}>{saveLabel}</button>
        {/snippet}
      </SettingsFooter>

      <SettingsFooter hidden={activeTab !== 'models'}>
        {#snippet left()}
          <button type="button" class="folder-link-btn" id="open-models-folder-btn" title="Open models folder in Finder" onclick={openModelsFolder}>
            <svg class="folder-link-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/>
            </svg>
            <span class="folder-link-label">Models</span>
          </button>
        {/snippet}
      </SettingsFooter>

      <SettingsFooter hidden={activeTab !== 'history'}>
        {#snippet left()}
          <button type="button" class="folder-link-btn" id="open-recordings-folder-btn" title="Open recordings folder in Finder" onclick={openRecordingsFolder}>
            <svg class="folder-link-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/>
            </svg>
            <span class="folder-link-label">Recordings</span>
          </button>
        {/snippet}
        {#snippet right()}
          <button class="btn secondary" id="clear-history-btn" onclick={() => historyRef?.clearAll()}>Clear All History</button>
        {/snippet}
      </SettingsFooter>

      <SettingsFooter hidden={activeTab !== 'debug'}>
        {#snippet left()}
          <button class="btn secondary" id="refresh-debug-btn" onclick={() => debugRef?.refresh()}>
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true" style="margin-right:6px;vertical-align:-2px">
              <path d="M21 12a9 9 0 1 1-2.64-6.36"/><path d="M21 3v6h-6"/>
            </svg>
            Refresh
          </button>
        {/snippet}
        {#snippet right()}
          <button class="btn secondary" id="clear-debug-btn" onclick={() => debugRef?.clearAll()}>Clear Log</button>
        {/snippet}
      </SettingsFooter>
    </div>
  </section>
</div>