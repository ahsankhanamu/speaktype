import { flushSync, mount, unmount, type Component } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Alert } from '@speaktype/ui';
import { setBus } from '../lib/data';
import { FakeBus } from '../lib/data/testBus';
import General from './pages/General.svelte';
import Dictation from './pages/Dictation.svelte';
import Advanced from './pages/Advanced.svelte';
import Storage from './pages/Storage.svelte';
import Providers from './pages/Providers.svelte';
import Permissions from './pages/Permissions.svelte';
import Appearance from './pages/Appearance.svelte';
import History from './pages/History.svelte';

const ipc = vi.hoisted(() => {
  const fn = () => vi.fn();
  return {
    getSettings: fn(), saveSettings: fn(), checkServer: fn(), getServerStatus: fn(), unloadModel: fn(),
    loadServer: fn(), serverInfo: fn(), resetWidgetPosition: fn(), openSystemPane: fn(), openModelsFolder: fn(),
    openRecordingsFolder: fn(), clearHistory: fn(), getHistory: fn(), sessionList: fn(), copyToClipboard: fn(),
    deleteHistoryEntry: fn(), getHistoryAudio: fn(), reprocessHistoryEntry: fn(), getDebugSessions: fn(),
    clearDebugSessions: fn(), deleteDebugSession: fn(), checkPermissions: fn(), openOnboarding: fn(),
    startMicTest: fn(), stopMicTest: fn(), getModels: fn(), getInputDevices: fn(), loadModel: fn(),
    queueModelDownload: fn(), pauseModelDownload: fn(), cancelModelDownload: fn(),
    commands: {
      sttSetProvider: fn(), sttSetApiKey: fn(), sttHasApiKey: fn(), sttRevealApiKey: fn(), sttListModels: fn(), sttTest: fn(),
    },
  };
});
const tauri = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('../lib/ipc', () => ipc);
vi.mock('@tauri-apps/api/core', () => tauri);
vi.mock('@tauri-apps/api/app', () => ({ getVersion: () => Promise.resolve('1.0') }));
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }));
vi.mock('../lib/app/micLevels', () => ({ micLevelsStart: vi.fn(), micLevelsStop: vi.fn() }));
vi.mock('./shortcuts', () => ({ shortcutsGet: () => Promise.resolve([{ action: 'dictation', keys: 'F8' }]) }));
vi.mock('../lib/app/deleteModel', () => ({ deleteModel: vi.fn() }));

const SETTINGS = {
  hotkey: 'Super+Control', api_url: 'http://127.0.0.1:8002/inference', model: 'small', language: 'auto',
  paste_mode: 'active', post_paste_keys: 'enter', save_recordings: false, theme: 'auto', input_device: null,
  hallucination_guard: true, hallucination_retries: 1, quality_no_speech_prob: 0.6, quality_avg_logprob: -1,
  quality_compression_ratio: 2.4, scribe_screenshot_hotkey: 'Super+Option+S', scribe_clipboard_monitor: true,
  stt_provider: null, sounds: true, widget_size: 'regular', launch_at_login: false,
};
const CATALOG = {
  available: ['tiny', 'base', 'small', 'medium', 'large-v3-turbo', 'large-v3'].map((id) => ({ id, size_bytes: 0, downloaded: id === 'small' })),
  downloading: [], waiting: [], progress: [],
};

let bus: FakeBus;
let app: ReturnType<typeof mount> | undefined;

function render(Page: Component) {
  app = mount(Page, { target: document.body });
  flushSync();
}
const text = () => document.body.textContent ?? '';
const labels = () => [...document.querySelectorAll('.st-row .text .label')].map((el) => el.textContent);
const button = (name: string) => [...document.querySelectorAll('button')].find((el) => el.textContent?.trim() === name) as HTMLButtonElement;
const switchFor = (label: string) => document.querySelector(`[role=switch][aria-label="${label}"]`) as HTMLElement;

beforeEach(() => {
  bus = new FakeBus();
  setBus(bus);
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} });
  Element.prototype.animate ??= function () {
    return { cancel() {}, set onfinish(done: () => void) { queueMicrotask(done); } } as unknown as Animation;
  };
  window.matchMedia ??= (() => ({ matches: false, addEventListener() {}, removeEventListener() {} })) as unknown as typeof window.matchMedia;
  Element.prototype.scrollIntoView ??= () => {};
  ipc.getSettings.mockResolvedValue({ ...SETTINGS });
  ipc.saveSettings.mockResolvedValue(null);
  ipc.getModels.mockResolvedValue(CATALOG);
  ipc.getInputDevices.mockResolvedValue({ devices: [], default: null });
  ipc.checkPermissions.mockResolvedValue({ microphone: true, accessibility: true });
  ipc.getServerStatus.mockResolvedValue({ embedded: true, running: true, stopped: false, port: 8002 });
  ipc.serverInfo.mockResolvedValue({ model: null, size: null });
  ipc.getDebugSessions.mockResolvedValue([]);
  ipc.getHistory.mockResolvedValue({ entries: [] });
  ipc.sessionList.mockResolvedValue([]);
  ipc.startMicTest.mockResolvedValue({ device: 'MacBook Pro Microphone' });
  ipc.commands.sttHasApiKey.mockResolvedValue(false);
  ipc.commands.sttSetProvider.mockImplementation(async (config) => config);
  ipc.commands.sttSetApiKey.mockResolvedValue(null);
  ipc.commands.sttListModels.mockResolvedValue([{ id: 'whisper-large-v3-turbo' }, { id: 'whisper-large-v3' }]);
  ipc.commands.sttRevealApiKey.mockResolvedValue('gsk_secret');
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});

describe('settings pages', () => {
  it('General holds launch at login, sounds and reset widget position', async () => {
    render(General);
    await vi.waitFor(() => expect(labels()).toEqual(['Launch at login', 'Sounds', 'Reset widget position']));
    switchFor('Sounds').click();
    await vi.waitFor(() => expect(ipc.saveSettings).toHaveBeenCalledWith(expect.objectContaining({ sounds: false, model: 'small' })));
    button('Reset widget position').click();
    await vi.waitFor(() => expect(ipc.resetWidgetPosition).toHaveBeenCalled());
  });

  it('Launch at login goes only through the autostart command', async () => {
    render(General);
    await vi.waitFor(() => expect(switchFor('Launch at login')).not.toBeNull());
    switchFor('Launch at login').click();
    await vi.waitFor(() =>
      expect(tauri.invoke.mock.calls.some(([cmd, args]) => cmd === 'set_launch_at_login' && (args as { on: boolean }).on)).toBe(true),
    );
    expect(ipc.saveSettings).not.toHaveBeenCalledWith(expect.objectContaining({ launch_at_login: true }));
  });

  it('Dictation carries every field from the spec', async () => {
    ipc.getModels.mockResolvedValue({
      ...CATALOG,
      available: CATALOG.available.map((model) => (model.id === 'small' ? { ...model, downloaded: false } : model)),
    });
    render(Dictation);
    await vi.waitFor(() => expect(text()).toContain('Manage models…'));
    expect(labels()).toEqual(
      expect.arrayContaining(['Start / stop dictation', 'Language', 'Microphone', 'Paste into', 'After pasting, press', 'Save recording audio']),
    );
    expect(text()).toContain('Balanced');
    switchFor('Save recording audio').click();
    await vi.waitFor(() => expect(ipc.saveSettings).toHaveBeenCalledWith(expect.objectContaining({ save_recordings: true })));
  });

  it('Dictation shows the hotkey as a link to Keyboard instead of a second field', async () => {
    location.hash = '';
    render(Dictation);
    await vi.waitFor(() => expect(document.querySelector('[aria-label="Change in Keyboard"]')).not.toBeNull());
    expect(document.querySelector('.st-shortcut')).toBeNull();
    expect([...document.querySelectorAll('.st-row')].find((row) => row.textContent?.includes('Start / stop dictation'))?.textContent).toContain('F8');
    (document.querySelector('[aria-label="Change in Keyboard"]') as HTMLElement).click();
    expect(location.hash).toBe('#/keyboard');
  });

  it('Advanced keeps the hallucination filter, server controls and API URL check', async () => {
    ipc.getDebugSessions.mockResolvedValue([{ id: 3, startedAt: '2026-11-19T10:00:00Z', model: 'small', durationSecs: 9, finalChars: 40, chunks: [{}], passes: [] }]);
    ipc.checkServer.mockResolvedValue({ status: 'connected' });
    render(Advanced);
    await vi.waitFor(() => expect(text()).toContain('Running on port 8002'));
    expect(labels()).toEqual(
      expect.arrayContaining(['Hallucination filter', 'Retries', 'Silence threshold', 'Confidence threshold', 'Repetition threshold', 'Transcription server', 'API URL']),
    );
    expect(text()).toContain('Recent dictation debug records');
    await vi.waitFor(() => expect(text()).toContain('1 chunks'));
    button('Check').click();
    await vi.waitFor(() => expect(ipc.checkServer).toHaveBeenCalledWith('http://127.0.0.1:8002/inference'));
    await vi.waitFor(() => expect(document.querySelector('.caption.success')?.textContent).toBe('Connected'));
  });

  it('a debug record opens to show its text and how each chunk was decoded', async () => {
    ipc.getDebugSessions.mockResolvedValue([
      {
        id: 3,
        startedAt: '2026-11-19T10:00:00Z',
        model: 'small',
        durationSecs: 9,
        finalChars: 11,
        finalText: 'hello there',
        chunks: [{ idx: 0, startSecs: 0, endSecs: 9, status: 'ok', chars: 11 }],
        passes: [{ chunkIdx: 0, pass: 1, rawChars: 11, score: 0.82 }],
      },
    ]);
    render(Advanced);
    const toggle = await vi.waitFor(() => document.querySelector<HTMLButtonElement>('[aria-label="Show details"]')!);
    toggle.click();
    await vi.waitFor(() => expect(text()).toContain('hello there'));
    expect(text()).toContain('Chunk 1 · 0:00–0:09');
    expect(text()).toContain('pass 1 · 11 chars · score 0.82');
  });

  it('Advanced unloads the model with the size the menu bar quotes, then loads it now', async () => {
    let running = true;
    ipc.getServerStatus.mockImplementation(async () => ({ embedded: true, running, stopped: !running, port: 8002 }));
    ipc.serverInfo.mockImplementation(async () =>
      running ? { model: 'small', size: '~500 MB' } : { model: null, size: null },
    );
    ipc.unloadModel.mockImplementation(async () => {
      running = false;
      return null;
    });
    ipc.loadServer.mockImplementation(async () => {
      running = true;
      return null;
    });
    render(Advanced);
    await vi.waitFor(() => expect(text()).toContain('Running · ~500 MB in memory'));
    button('Unload model').click();
    await vi.waitFor(() => expect(ipc.unloadModel).toHaveBeenCalled());
    await vi.waitFor(() => expect(button('Load now')).toBeTruthy());
    expect(text()).toContain('Not loaded');
    expect(text()).toContain('loads again on the next dictation or when you press Load now');
    button('Load now').click();
    await vi.waitFor(() => expect(ipc.loadServer).toHaveBeenCalled());
    await vi.waitFor(() => expect(button('Unload model')).toBeTruthy());
  });

  it('Storage maps the retention choice to history_limit, null meaning forever', async () => {
    ipc.getSettings.mockResolvedValue({ ...SETTINGS, history_limit: null });
    render(Storage);
    await vi.waitFor(() => expect(text()).toContain('Forever'));
  });

  it('Storage shows an age choice when History keeps dictations by age', async () => {
    ipc.getSettings.mockResolvedValue({ ...SETTINGS, history_limit: null, history_max_days: 90 });
    render(Storage);
    await vi.waitFor(() => expect(text()).toContain('3 months'));
  });
});

describe('Providers', () => {
  const cloud = { kind: 'cloud', service: 'groq', baseUrl: 'https://api.groq.com/openai/v1', model: '', fallbackToLocal: true };

  it('shows the three cards and the cloud steps in dependency order', async () => {
    ipc.getSettings.mockResolvedValue({ ...SETTINGS, stt_provider: cloud });
    render(Providers);
    await vi.waitFor(() => expect(labels()).toContain('1. Service'));
    expect([...document.querySelectorAll('.provider-card .title')].map((el) => el.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'On this Mac Recommended', 'Your own server', 'Cloud service',
    ]);
    expect(labels().filter((label) => /^\d\./.test(label ?? ''))).toEqual(['1. Service', '2. Address', '3. API key', '4. Model', ]);
  });

  it('sends the key to the Keychain, connects, then enables the model list', async () => {
    ipc.getSettings.mockResolvedValue({ ...SETTINGS, stt_provider: { ...cloud, model: '' } });
    ipc.commands.sttListModels.mockRejectedValueOnce({ message: 'Unauthorized' });
    render(Providers);
    await vi.waitFor(() => expect(document.querySelector('input[aria-label="Model"]')).not.toBeNull());
    expect((document.querySelector('input[aria-label="Model"]') as HTMLInputElement).disabled).toBe(true);
    await vi.waitFor(() => expect(text()).toContain('Unauthorized'));

    const key = document.querySelector('input[aria-label="API key"]') as HTMLInputElement;
    key.value = 'gsk_new';
    key.dispatchEvent(new Event('input', { bubbles: true }));
    flushSync();
    button('Connect').click();

    await vi.waitFor(() => expect(ipc.commands.sttSetApiKey).toHaveBeenCalledWith('groq', 'gsk_new'));
    await vi.waitFor(() => expect(text()).toContain('Connected · 2 speech models found'));
    expect(document.querySelector('input[aria-label="Model"][disabled]')).toBeNull();
    expect(ipc.commands.sttSetApiKey.mock.invocationCallOrder[0]).toBeLessThan(ipc.commands.sttListModels.mock.invocationCallOrder.at(-1)!);
  });

  it('reveals the saved key through the authenticated command, never the settings file', async () => {
    ipc.getSettings.mockResolvedValue({ ...SETTINGS, stt_provider: cloud });
    ipc.commands.sttHasApiKey.mockResolvedValue(true);
    render(Providers);
    await vi.waitFor(() => expect(document.querySelector('[aria-label="Show"]')).not.toBeNull());
    (document.querySelector('[aria-label="Show"]') as HTMLElement).click();
    await vi.waitFor(() => expect(ipc.commands.sttRevealApiKey).toHaveBeenCalledWith('groq'));
    await vi.waitFor(() => expect((document.querySelector('input[aria-label="API key"]') as HTMLInputElement).value).toBe('gsk_secret'));
  });

  it('shows the voice disclosure with the service name', async () => {
    ipc.getSettings.mockResolvedValue({ ...SETTINGS, stt_provider: cloud });
    render(Providers);
    await vi.waitFor(() => expect(text()).toContain('Your voice will be sent to Groq'));
  });
});

describe('Permissions', () => {
  it('has no test running until asked, and only one at a time', async () => {
    render(Permissions);
    await vi.waitFor(() => expect(button('Test')).toBeDefined());
    expect(ipc.startMicTest).not.toHaveBeenCalled();
    expect(button('Stop test')).toBeUndefined();

    button('Test').click();
    await vi.waitFor(() => expect(button('Stop test')).toBeDefined());
    expect(ipc.startMicTest).toHaveBeenCalledTimes(1);

    button('Test typing').click();
    await vi.waitFor(() => expect(ipc.stopMicTest).toHaveBeenCalled());
    await vi.waitFor(() => expect(button('Done')).toBeDefined());
    expect(button('Stop test')).toBeUndefined();
    expect(document.querySelector('textarea')?.placeholder).toBe('SpeakType test');
  });

  it('reports why typing failed', async () => {
    tauri.invoke.mockRejectedValue({ message: 'Accessibility is off' });
    render(Permissions);
    await vi.waitFor(() => expect(button('Test typing')).toBeDefined());
    button('Test typing').click();
    await vi.waitFor(() => expect(text()).toContain('Accessibility is off'));
  });

  it('points at System Settings when a permission is missing', async () => {
    ipc.checkPermissions.mockResolvedValue({ microphone: false, accessibility: false });
    render(Permissions);
    await vi.waitFor(() => expect(text()).toContain('Not allowed'));
    expect(button('Test')).toBeUndefined();
  });
});

describe('Review focus 4: the widget changes a setting while the window is open', () => {
  it('Appearance follows settings-changed without reopening', async () => {
    render(Appearance);
    const checked = () => document.querySelector('[aria-label="Widget size"] [data-state=checked]')?.textContent?.trim();
    await vi.waitFor(() => expect(checked()).toBe('Regular'));
    ipc.getSettings.mockResolvedValue({ ...SETTINGS, widget_size: 'large' });
    await vi.waitFor(() => expect(bus.liveListeners).toBeGreaterThan(0));
    bus.emit('settings-changed', null);
    await vi.waitFor(() => expect(checked()).toBe('Large'));
  });
});

describe('History', () => {
  const entries = Array.from({ length: 1000 }, (_, i) => ({
    id: String(i), text: `Dictation ${i}`, timestamp: new Date(Date.now() - i * 3_600_000).toISOString(), duration_secs: 4,
  }));

  it('renders a window of 1,000 entries, not all of them', async () => {
    ipc.getHistory.mockResolvedValue({ entries });
    render(History);
    await vi.waitFor(() => expect(document.querySelectorAll('[role=listitem]').length).toBeGreaterThan(0));
    expect(document.querySelectorAll('[role=listitem]').length).toBeLessThan(40);
  });

  it('teaches the shortcuts when empty', async () => {
    render(History);
    await vi.waitFor(() => expect(text()).toContain('Nothing here yet'));
  });

  it('lists sessions as rows with a coming-soon Timeline', async () => {
    ipc.sessionList.mockResolvedValue([{ id: 1, name: 'Checkout bug', startedAtMs: Date.now(), endedAtMs: null, markCount: 2, clipCount: 0, snippet: null }]);
    render(History);
    await vi.waitFor(() => expect(text()).toContain('Timeline for this session is coming soon'));
  });

  it('deletes the selected dictation by its position in the history file', async () => {
    ipc.getHistory.mockResolvedValue({ entries: entries.slice(0, 3) });
    render(History);
    const alert = mount(Alert, { target: document.body });
    await vi.waitFor(() => expect(document.querySelector('[aria-label="More"]')).not.toBeNull());
    const more = document.querySelector('[aria-label="More"]') as HTMLElement;
    more.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    const del = await vi.waitFor(() => {
      const item = [...document.querySelectorAll('[role=menuitem]')].find((el) => el.textContent?.trim() === 'Delete');
      expect(item).toBeDefined();
      return item as HTMLElement;
    });
    del.click();
    await vi.waitFor(() => expect(document.body.textContent).toContain('Delete this dictation?'));
    (document.querySelector('[data-alert-dialog-action]') as HTMLElement).click();
    await vi.waitFor(() => expect(ipc.deleteHistoryEntry).toHaveBeenCalledWith(0));
    await unmount(alert);
  });
});
