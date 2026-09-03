import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';

export interface Settings {
  hotkey: string;
  api_url: string;
  model: string;
  language: string;
  window_x?: number | null;
  window_y?: number | null;
  onboarding_window_x?: number | null;
  onboarding_window_y?: number | null;
  settings_x?: number | null;
  settings_y?: number | null;
  settings_w?: number | null;
  settings_h?: number | null;
  paste_mode: string;
  post_paste_keys: string | null;
  save_recordings: boolean;
  theme: 'auto' | 'light' | 'dark';
  hallucination_guard: boolean;
  hallucination_retries: number;
  quality_no_speech_prob: number;
  quality_avg_logprob: number;
  quality_compression_ratio: number;
  input_device: string | null;
}

export interface InputDeviceInfo {
  name: string;
  label: string;
  is_default: boolean;
}

export interface InputDevicesResult {
  devices: InputDeviceInfo[];
  default: string | null;
}

export interface AudioInputInfo {
  device: string | null;
  inputs: number;
}

export interface PermissionsStatus {
  accessibility: boolean;
  microphone: boolean;
}

export interface DebugSession {
  id: number;
  text: string;
  timestamp: string;
  duration_secs: number;
  [key: string]: unknown;
}

export interface ModelEntry {
  id: string;
  name?: string;
  size_bytes?: number;
  downloaded?: boolean;
  loading?: boolean;
  [key: string]: unknown;
}

export interface HistoryEntry {
  text: string;
  timestamp: string;
  duration_secs?: number;
  [key: string]: unknown;
}

export function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args);
}

export const logFrontend = (level: string, message: string) =>
  invokeCommand<void>('log_frontend', { level, message });

// Recording / widget
export const toggleRecording = (isRecording: boolean) =>
  invokeCommand('toggle_recording', { isRecording });
export const cancelRecording = () => invokeCommand<void>('cancel_recording');
export const hideWidget = () => invokeCommand<void>('hide_widget');
export const minimizeWidget = () => invokeCommand<void>('minimize_widget');
export const quitApp = () => invokeCommand<void>('quit_app');
export const saveWindowPosition = () => invokeCommand<void>('save_window_position');
export const resetWidgetPosition = () => invokeCommand<void>('reset_widget_position');

// Windows
export const openAbout = () => invokeCommand<void>('open_about');
export const openSettings = () => invokeCommand<void>('open_settings');
export const openOnboarding = () => invokeCommand<void>('open_onboarding');
export const finishOnboarding = () => invokeCommand<void>('finish_onboarding');
export const getOnboardingStatus = () => invokeCommand<OnboardingStatus>('get_onboarding_status');
export const requestMicrophoneAccess = () => invokeCommand<void>('request_microphone_access');

export interface OnboardingStatus {
  microphone: boolean;
  accessibility: boolean;
  model: boolean;
  active_model: string;
  exe_path: string;
  is_dev: boolean;
  is_bundled: boolean;
  responsible_app: string;
  responsible_app_path: string;
}

// Settings
export const getSettings = () => invokeCommand<Settings>('get_settings');
export const saveSettings = (settings: Settings) =>
  invokeCommand<void>('save_settings', { settings });
export const updateHotkey = (hotkey: string) =>
  invokeCommand<void>('update_hotkey', { hotkey });

// Server
export interface ServerCheckResponse {
  status: 'connected' | 'error' | 'disconnected';
  message?: string;
  info?: unknown;
}
export interface ServerRestartResponse {
  status: string;
  port: number;
  api_url: string;
  model: string;
}
export const checkServer = (apiUrl: string) =>
  invokeCommand<ServerCheckResponse>('check_server', { apiUrl });
export interface ServerStatus {
  embedded: boolean;
  running: boolean;
  port: number | null;
}
export const getServerStatus = () => invokeCommand<ServerStatus>('get_server_status');
export const stopWhisperServer = () => invokeCommand<void>('stop_whisper_server');
export const restartWhisperServer = () =>
  invokeCommand<ServerRestartResponse>('restart_whisper_server');

// History
export const getHistory = () => invokeCommand<HistoryEntry[]>('get_history');
export const deleteHistoryEntry = (index: number) =>
  invokeCommand<void>('delete_history_entry', { index });
export const deleteHistoryAudio = (index: number) =>
  invokeCommand<void>('delete_history_audio', { index });
export const clearHistory = () => invokeCommand<void>('clear_history');
export const reprocessHistoryEntry = (index: number) =>
  invokeCommand<void>('reprocess_history_entry', { index });
export const getHistoryAudio = (index: number) =>
  invokeCommand<number[]>('get_history_audio', { index });

// Debug
export const getDebugSessions = () => invokeCommand<DebugSession[]>('get_debug_sessions');
export const clearDebugSessions = () => invokeCommand<void>('clear_debug_sessions');
export const deleteDebugSession = (sessionId: number) =>
  invokeCommand<void>('delete_debug_session', { sessionId });
export const getDebugAudioSlice = (sessionId: number, startSecs: number, endSecs: number) =>
  invokeCommand<number[]>('get_debug_audio_slice', { sessionId, startSecs, endSecs });

// Permissions
export const checkPermissions = () => invokeCommand<PermissionsStatus>('check_permissions');
export const requestAccessibility = () => invokeCommand<void>('request_accessibility');

// Microphone
export interface StartMicTestResult {
  device: string | null;
  sample_rate: number;
  channels: number;
  inputs: number;
  max_ms: number;
}
export const startMicTest = () => invokeCommand<StartMicTestResult>('start_mic_test');
export const stopMicTest = () => invokeCommand<void>('stop_mic_test');
export const getAudioInputInfo = () => invokeCommand<AudioInputInfo>('get_audio_input_info');
export const getInputDevices = () => invokeCommand<InputDevicesResult>('get_input_devices');

// System / folders
export const openSystemPane = (pane: string) =>
  invokeCommand<void>('open_system_pane', { pane });
export const openModelsFolder = () => invokeCommand<void>('open_models_folder');
export const openRecordingsFolder = () => invokeCommand<void>('open_recordings_folder');

// Models
export const loadModel = (model: string) => invokeCommand<void>('load_model', { model });
export const queueModelDownload = (model: string, restart = false) =>
  invokeCommand<void>('queue_model_download', { model, restart });
export const restartModelDownload = (model: string) =>
  invokeCommand<void>('restart_model_download', { model });
export const getModels = () => invokeCommand<ModelEntry[]>('get_models');
export const getModelProgress = () => invokeCommand<unknown>('get_model_progress');
export const cancelModelDownload = (model: string) =>
  invokeCommand<void>('cancel_model_download', { model });
export const pauseModelDownload = (model: string) =>
  invokeCommand<void>('pause_model_download', { model });

// Events
export function listenEvent<T>(
  eventName: string,
  callback: (payload: T) => void,
): Promise<() => void> {
  return listen<T>(eventName, (event) => callback(event.payload));
}

export const emitEvent = <T>(eventName: string, payload?: T) =>
  emit(eventName, payload);