// IPC wrappers for Tauri v2 communication
// Tauri v2: API at window.__TAURI__.core.invoke / window.__TAURI__.event.listen
// Note: "ipc" is a reserved global in Tauri's WKWebView, so we use "ttipc"

function getInvoke() {
  if (window.__TAURI__ && window.__TAURI__.core) return window.__TAURI__.core.invoke;
  if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) return window.__TAURI_INTERNALS__.invoke;
  return null;
}

function getListen() {
  if (window.__TAURI__ && window.__TAURI__.event) return window.__TAURI__.event.listen;
  if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.listen) return window.__TAURI_INTERNALS__.listen;
  return null;
}

function getEmit() {
  if (window.__TAURI__ && window.__TAURI__.event) return window.__TAURI__.event.emit;
  if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.emit) return window.__TAURI_INTERNALS__.emit;
  return null;
}

const ttipc = {
  invoke(cmd, args) {
    const fn = getInvoke();
    if (!fn) return Promise.resolve();
    return fn(cmd, args);
  },

  listen(eventName, callback) {
    const fn = getListen();
    if (!fn) return Promise.resolve();
    return fn(eventName, callback);
  },

  emit(eventName, payload) {
    const fn = getEmit();
    if (!fn) return Promise.resolve();
    return fn(eventName, payload);
  },

  toggleRecording(isRecording) {
    return this.invoke('toggle_recording', { isRecording });
  },

  cancelRecording() {
    return this.invoke('cancel_recording');
  },

  getSettings() {
    return this.invoke('get_settings');
  },

  saveSettings(settings) {
    return this.invoke('save_settings', { settings });
  },

  updateHotkey(hotkey) {
    return this.invoke('update_hotkey', { hotkey });
  },

  checkServer(apiUrl) {
    return this.invoke('check_server', { apiUrl });
  },

  getServerStatus() {
    return this.invoke('get_server_status');
  },

  stopWhisperServer() {
    return this.invoke('stop_whisper_server');
  },

  restartWhisperServer() {
    return this.invoke('restart_whisper_server');
  },

  openAbout() {
    return this.invoke('open_about');
  },

  openSettings() {
    return this.invoke('open_settings');
  },

  hideWidget() {
    return this.invoke('hide_widget');
  },

  minimizeWidget() {
    return this.invoke('minimize_widget');
  },

  quitApp() {
    return this.invoke('quit_app');
  },

  saveWindowPosition() {
    return this.invoke('save_window_position');
  },

  resetWidgetPosition() {
    return this.invoke('reset_widget_position');
  },

  getHistory() {
    return this.invoke('get_history');
  },

  deleteHistoryEntry(index) {
    return this.invoke('delete_history_entry', { index });
  },

  deleteHistoryAudio(index) {
    return this.invoke('delete_history_audio', { index });
  },

  clearHistory() {
    return this.invoke('clear_history');
  },

  reprocessHistoryEntry(index) {
    return this.invoke('reprocess_history_entry', { index });
  },

  getHistoryAudio(index) {
    return this.invoke('get_history_audio', { index });
  },

  checkPermissions() {
    return this.invoke('check_permissions');
  },

  requestAccessibility() {
    return this.invoke('request_accessibility');
  },

  openSystemPane(pane) {
    return this.invoke('open_system_pane', { pane });
  },

  openModelsFolder() {
    return this.invoke('open_models_folder');
  },

  openRecordingsFolder() {
    return this.invoke('open_recordings_folder');
  },

  loadModel(model) {
    return this.invoke('load_model', { model });
  },

  queueModelDownload(model, restart = false) {
    return this.invoke('queue_model_download', { model, restart });
  },

  restartModelDownload(model) {
    return this.invoke('restart_model_download', { model });
  },

  getModels() {
    return this.invoke('get_models');
  },

  getModelProgress() {
    return this.invoke('get_model_progress');
  },

  cancelModelDownload(model) {
    return this.invoke('cancel_model_download', { model });
  },

  pauseModelDownload(model) {
    return this.invoke('pause_model_download', { model });
  },
};

window.ttipc = ttipc;
