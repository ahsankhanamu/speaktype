// Thin compatibility layer over the generated bindings. The single source of
// truth for commands, events and their types is `bindings.ts` (generated from
// the Rust signatures by tauri-specta). Callers import from here unchanged;
// surfaces migrate to `commands.*`/`events.*` directly as they are rebuilt.
import { invoke } from '@tauri-apps/api/core';
export { emitEvent } from './data/bus';

export * from './bindings';
import { commands, type Settings as GeneratedSettings } from './bindings';

// The Rust `Settings.theme` is a plain string; the UI's picker is the union.
// The old hand-written interface asserted the same.
export type Settings = GeneratedSettings & { theme: 'auto' | 'light' | 'dark' };

export function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args);
}

// ─── Command aliases (existing call sites keep their imports) ────────────────
// Value-returning commands keep their hand-written payload types; the alias
// casts only — the runtime call is still the generated one.

// Recording / widget
export const toggleRecording = commands.toggleRecording;
export const cancelRecording = commands.cancelRecording;
export const hideWidget = commands.hideWidget as unknown as () => Promise<void>;
export const minimizeWidget = commands.minimizeWidget as unknown as () => Promise<void>;
export const quitApp: () => Promise<void> = commands.quitApp;
export const saveWindowPosition = commands.saveWindowPosition;
export const resetWidgetPosition = commands.resetWidgetPosition;

// Windows
export const openAbout = commands.openAbout;
export const openSettings = () => commands.openSettings(null);
export const openOnboarding = commands.openOnboarding;
export const finishOnboarding = commands.finishOnboarding;
export const getOnboardingStatus = commands.getOnboardingStatus as unknown as () =>
  Promise<OnboardingStatus>;
export const requestMicrophoneAccess = commands.requestMicrophoneAccess as unknown as () =>
  Promise<boolean>;

// Clipboard
export const copyToClipboard = commands.copyToClipboard;
export const logFrontend = commands.logFrontend;

// Settings
export const getSettings = commands.getSettings as unknown as () => Promise<Settings>;
export const saveSettings = commands.saveSettings as unknown as (settings: Settings) =>
  Promise<void>;

// Server
export const checkServer = commands.checkServer as unknown as (
  apiUrl: string,
) => Promise<ServerCheckResponse>;
export const getServerStatus = commands.getServerStatus as unknown as () => Promise<ServerStatus>;
export const stopWhisperServer = commands.stopWhisperServer as unknown as () =>
  Promise<ServerCheckResponse>;
export const unloadModel = commands.unloadModel;
export const loadServer = commands.loadServer;
/** Model name and catalog size while it sits in memory; nulls while it runs busy. */
export const serverInfo = commands.serverInfo;
export const restartWhisperServer = commands.restartWhisperServer as unknown as () =>
  Promise<ServerRestartResponse>;

// History
export const getHistory = commands.getHistory;
export const deleteHistoryEntry = commands.deleteHistoryEntry;
export const deleteHistoryAudio = commands.deleteHistoryAudio;
export const clearHistory = commands.clearHistory;
export const reprocessHistoryEntry = commands.reprocessHistoryEntry;
export const reprocessPreview = commands.reprocessPreview;
export const getHistoryAudio = commands.getHistoryAudio;

// Debug
// specta models `#[serde(default)]` scalars as `T | null`; Rust always sends
// these fields (they are not Options), so the nulls are spurious. Genuine
// Option fields (noSpeechProb, avgLogprob) keep their null.
type DefaultedScalar = 'durationSecs' | 'startSecs' | 'endSecs' | 'temperature' | 'score' | 'start' | 'end';
type NoDefaultNull<T> = T extends (infer U)[]
  ? NoDefaultNull<U>[]
  : T extends object
    ? {
        [K in keyof T]: K extends DefaultedScalar
          ? Exclude<T[K], null>
          : K extends 'id'
            ? Exclude<T[K], undefined>
            : NoDefaultNull<T[K]>;
      }
    : T;
export type DebugRecord = NoDefaultNull<import('./bindings').DebugSession> & {
  id: number;
};
export const getDebugSessions = commands.getDebugSessions as unknown as () =>
  Promise<DebugRecord[]>;
export const clearDebugSessions = commands.clearDebugSessions;
export const deleteDebugSession = commands.deleteDebugSession;
export const getDebugAudioSlice = commands.getDebugAudioSlice;

// Permissions
export const checkPermissions = commands.checkPermissions as unknown as () =>
  Promise<PermissionsStatus>;
export const requestAccessibility = commands.requestAccessibility as unknown as () =>
  Promise<boolean>;

// Microphone
export const startMicTest = commands.startMicTest as unknown as () => Promise<StartMicTestResult>;
export const stopMicTest = commands.stopMicTest;
export const getAudioInputInfo = commands.getAudioInputInfo as unknown as () =>
  Promise<AudioInputInfo>;
export const getInputDevices = commands.getInputDevices as unknown as () =>
  Promise<InputDevicesResult>;

// System / folders
export const openSystemPane = commands.openSystemPane;
export const openModelsFolder = commands.openModelsFolder as unknown as () => Promise<void>;
export const openRecordingsFolder = commands.openRecordingsFolder as unknown as () =>
  Promise<void>;
export const revealPath = commands.revealPath;

// Models
export const loadModel = commands.loadModel as unknown as (model: string) => Promise<unknown>;
export const queueModelDownload = commands.queueModelDownload as unknown as (
  model: string,
  restart?: boolean,
) => Promise<unknown>;
export const restartModelDownload = commands.restartModelDownload as unknown as (
  model: string,
) => Promise<unknown>;
export const getModels = commands.getModels as unknown as () => Promise<ModelCatalog>;
export const getModelProgress = commands.getModelProgress as unknown as () => Promise<unknown>;
export const cancelModelDownload = commands.cancelModelDownload as unknown as (
  model: string,
) => Promise<unknown>;
export const pauseModelDownload = commands.pauseModelDownload as unknown as (
  model: string,
) => Promise<unknown>;

// Sessions (spec §7.4)
export const sessionStart = commands.sessionStart;
export const sessionStop = commands.sessionStop;
export const sessionActive = commands.sessionActive;
export const sessionSetMute = commands.sessionSetMute;
export const sessionList = commands.sessionList;
export const sessionGet = commands.sessionGet;
export const sessionRename = commands.sessionRename;
export const sessionDelete = commands.sessionDelete;
export const itemUpdate = commands.itemUpdate;
export const itemDelete = commands.itemDelete;
export const itemRestore = commands.itemRestore;
export const itemMove = commands.itemMove;
export const itemSplit = commands.itemSplit;
export const promptRender = commands.promptRender;
export const promptCopy = commands.promptCopy;
export const bundleReveal = commands.bundleReveal;
export const overlayPen = commands.overlayPen;
export const overlayMarkComplete = commands.overlayMarkComplete;

// ─── Hand-written payload types for commands that return raw JSON ────────────

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

export interface ServerStatus {
  embedded: boolean;
  running: boolean;
  /** After an unload: stopped on purpose, so dictation does not reload it. */
  stopped: boolean;
  port: number | null;
}

export interface StartMicTestResult {
  device: string | null;
  sample_rate: number;
  channels: number;
  inputs: number;
  max_ms: number;
}

export interface ModelEntry {
  id: string;
  name?: string;
  size_bytes?: number;
  downloaded?: boolean;
  partial?: boolean;
  partial_size?: number;
  expected_size?: number;
  [key: string]: unknown;
}

/** What `get_models` returns: the catalog plus the live download queue. */
export interface ModelCatalog {
  available: ModelEntry[];
  downloading: string[];
  waiting: string[];
  progress: { model: string; phase: string; percent: number; bytes_downloaded: number; total_bytes: number }[];
}
