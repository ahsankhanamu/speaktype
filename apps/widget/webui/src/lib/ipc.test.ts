import { describe, it, expect, vi, beforeEach } from 'vitest';

const invoke = vi.fn();
const listen = vi.fn();
const emitFn = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => listen(...args),
  emit: (...args: unknown[]) => emitFn(...args),
}));

describe('ipc.ts command surface', () => {
  beforeEach(() => {
    invoke.mockReset().mockResolvedValue(undefined);
    listen.mockReset().mockResolvedValue(() => {});
    emitFn.mockReset();
  });

  it('statically records every invoked command with expected args', async () => {
    // Holding dynamic imports hoisted; import inside to avoid vi.mock ordering issues.
    const ipc = await import('./ipc');

    await ipc.toggleRecording(true);
    await ipc.cancelRecording();
    await ipc.hideWidget();
    await ipc.minimizeWidget();
    await ipc.quitApp();
    await ipc.saveWindowPosition();
    await ipc.resetWidgetPosition();
    await ipc.openAbout();
    await ipc.openSettings();
    await ipc.openOnboarding();
    await ipc.finishOnboarding();
    await ipc.getOnboardingStatus();
    await ipc.requestMicrophoneAccess();
    await ipc.getSettings();
    await ipc.saveSettings({} as import('./ipc').Settings);
    await ipc.updateHotkey('Super+Control');
    await ipc.checkServer('http://127.0.0.1:8002/inference');
    await ipc.getServerStatus();
    await ipc.stopWhisperServer();
    await ipc.restartWhisperServer();
    await ipc.getHistory();
    await ipc.deleteHistoryEntry(3);
    await ipc.deleteHistoryAudio(1);
    await ipc.clearHistory();
    await ipc.reprocessHistoryEntry(2);
    await ipc.getHistoryAudio(0);
    await ipc.getDebugSessions();
    await ipc.clearDebugSessions();
    await ipc.deleteDebugSession(9);
    await ipc.getDebugAudioSlice(1, 0, 2.5);
    await ipc.checkPermissions();
    await ipc.requestAccessibility();
    await ipc.startMicTest();
    await ipc.stopMicTest();
    await ipc.getAudioInputInfo();
    await ipc.getInputDevices();
    await ipc.openSystemPane('com.apple.preference.security');
    await ipc.openModelsFolder();
    await ipc.openRecordingsFolder();
    await ipc.loadModel('medium');
    await ipc.queueModelDownload('large-v3', true);
    await ipc.restartModelDownload('tiny');
    await ipc.getModels();
    await ipc.getModelProgress();
    await ipc.cancelModelDownload('base');
    await ipc.pauseModelDownload('small');
    await ipc.logFrontend('info', 'hello');

    const names = invoke.mock.calls.map((c) => c[0]);
    const expected = [
      'toggle_recording',
      'cancel_recording',
      'hide_widget',
      'minimize_widget',
      'quit_app',
      'save_window_position',
      'reset_widget_position',
      'open_about',
      'open_settings',
      'open_onboarding',
      'finish_onboarding',
      'get_onboarding_status',
      'request_microphone_access',
      'get_settings',
      'save_settings',
      'update_hotkey',
      'check_server',
      'get_server_status',
      'stop_whisper_server',
      'restart_whisper_server',
      'get_history',
      'delete_history_entry',
      'delete_history_audio',
      'clear_history',
      'reprocess_history_entry',
      'get_history_audio',
      'get_debug_sessions',
      'clear_debug_sessions',
      'delete_debug_session',
      'get_debug_audio_slice',
      'check_permissions',
      'request_accessibility',
      'start_mic_test',
      'stop_mic_test',
      'get_audio_input_info',
      'get_input_devices',
      'open_system_pane',
      'open_models_folder',
      'open_recordings_folder',
      'load_model',
      'queue_model_download',
      'restart_model_download',
      'get_models',
      'get_model_progress',
      'cancel_model_download',
      'pause_model_download',
      'log_frontend',
    ];
    expect(names).toEqual(expected);
  });

  it('passes the right args for parameterized commands', async () => {
    const ipc = await import('./ipc');

    await ipc.deleteHistoryEntry(3);
    expect(invoke.mock.calls[0][1]).toEqual({ index: 3 });

    await ipc.getDebugAudioSlice(1, 0, 2.5);
    expect(invoke.mock.calls[1][1]).toEqual({ sessionId: 1, startSecs: 0, endSecs: 2.5 });

    await ipc.queueModelDownload('large-v3', true);
    expect(invoke.mock.calls[2][1]).toEqual({ model: 'large-v3', restart: true });

    await ipc.checkServer('http://127.0.0.1:8002/inference');
    expect(invoke.mock.calls[3][1]).toEqual({ apiUrl: 'http://127.0.0.1:8002/inference' });

    await ipc.updateHotkey('Super+Control');
    expect(invoke.mock.calls[4][1]).toEqual({ hotkey: 'Super+Control' });

    await ipc.openSystemPane('com.apple.preference.security');
    expect(invoke.mock.calls[5][1]).toEqual({ pane: 'com.apple.preference.security' });
  });

  it('wraps listen and emit for events', async () => {
    const ipc = await import('./ipc');

    const cb = vi.fn();
    const unlisten = await ipc.listenEvent<{ theme: string }>('theme:changed', cb);
    expect(listen).toHaveBeenCalledWith('theme:changed', expect.any(Function));
    expect(typeof unlisten).toBe('function');

    ipc.emitEvent('theme:changed', { theme: 'dark' });
    expect(emitFn).toHaveBeenCalledWith('theme:changed', { theme: 'dark' });
  });
});