import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setBus } from '../data';
import { FakeBus } from '../data/testBus';

const ipc = vi.hoisted(() => ({
  getModels: vi.fn(),
  getSettings: vi.fn(),
  saveSettings: vi.fn(),
  getInputDevices: vi.fn(),
  loadModel: vi.fn(),
  queueModelDownload: vi.fn(),
  pauseModelDownload: vi.fn(),
  cancelModelDownload: vi.fn(),
}));
const mic = vi.hoisted(() => ({ micLevelsStart: vi.fn(), micLevelsStop: vi.fn() }));
const remove = vi.hoisted(() => ({ deleteModel: vi.fn() }));
vi.mock('../ipc', () => ipc);
vi.mock('./micLevels', () => mic);
vi.mock('./deleteModel', () => remove);

const { default: MicPicker } = await import('./MicPicker.svelte');
const { default: ModelMenu } = await import('./ModelMenu.svelte');
const { default: SpeechModelsSheet } = await import('./SpeechModelsSheet.svelte');

const entry = (id: string, over = {}) => ({ id, size_bytes: 0, downloaded: false, ...over });
const catalog = {
  available: [
    entry('base', { downloaded: true }),
    entry('small'),
    entry('large-v3-turbo'),
    entry('tiny'),
    entry('medium'),
    entry('large-v3'),
  ],
  downloading: [],
  waiting: [],
  progress: [],
};

let bus: FakeBus;
let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  bus = new FakeBus();
  setBus(bus);
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  Element.prototype.animate ??= function () {
    return { cancel() {}, set onfinish(done: () => void) { queueMicrotask(done); } } as unknown as Animation;
  };
  window.matchMedia ??= (() => ({ matches: false, addEventListener() {}, removeEventListener() {} })) as unknown as typeof window.matchMedia;
  ipc.getModels.mockResolvedValue(catalog);
  ipc.getSettings.mockResolvedValue({ model: 'base', language: 'auto', input_device: null });
  ipc.getInputDevices.mockResolvedValue({
    devices: [
      { name: 'MacBook Pro Microphone', label: 'MacBook Pro Microphone (48 kHz)', is_default: true },
      { name: 'AirPods Pro', label: 'AirPods Pro (Bluetooth hands-free, 16 kHz)', is_default: false },
    ],
    default: 'MacBook Pro Microphone',
  });
  ipc.loadModel.mockResolvedValue({ status: 'started' });
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});

describe('MicPicker', () => {
  it('starts mic levels on mount and calls micLevelsStop on destroy', async () => {
    app = mount(MicPicker, { target: document.body });
    flushSync();
    expect(mic.micLevelsStart).toHaveBeenCalledTimes(1);
    expect(mic.micLevelsStop).not.toHaveBeenCalled();
    await unmount(app);
    app = undefined;
    expect(mic.micLevelsStop).toHaveBeenCalledTimes(1);
    expect(bus.liveListeners).toBe(0);
  });

  it('shows System default, each device, and a per-device level', async () => {
    app = mount(MicPicker, { target: document.body });
    await vi.waitFor(() => expect(document.querySelectorAll('[role=radio]')).toHaveLength(3));
    const names = [...document.querySelectorAll('.name')].map((n) => n.textContent);
    expect(names).toEqual(['System default', 'MacBook Pro Microphone', 'AirPods Pro']);
    expect(document.body.textContent).toContain('Bluetooth · lower quality while in use');

    await vi.waitFor(() => expect(bus.liveListeners).toBeGreaterThan(0));
    bus.emit('mic-levels', { levels: [{ device: 'AirPods Pro', level: 0.5 }] });
    await vi.waitFor(() => {
      const meters = document.querySelectorAll('[role=meter]');
      expect(meters[2].getAttribute('aria-valuenow')).toBe('0.5');
      expect(meters[1].getAttribute('aria-valuenow')).toBe('0');
    });
  });

  it('keeps the levels stream alive every 20 s while open', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    app = mount(MicPicker, { target: document.body });
    flushSync();
    expect(mic.micLevelsStart).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(20_000);
    expect(mic.micLevelsStart).toHaveBeenCalledTimes(2);
    vi.advanceTimersByTime(20_000);
    expect(mic.micLevelsStart).toHaveBeenCalledTimes(3);
    await unmount(app);
    app = undefined;
    vi.advanceTimersByTime(60_000);
    expect(mic.micLevelsStart).toHaveBeenCalledTimes(3);
    vi.useRealTimers();
  });

  it('marks a missing selected device as not connected', async () => {
    ipc.getSettings.mockResolvedValue({ model: 'base', language: 'auto', input_device: 'Studio Display Microphone' });
    app = mount(MicPicker, { target: document.body });
    await vi.waitFor(() => expect(document.body.textContent).toContain('Not connected'));
    expect(document.querySelector('[data-state=checked] .name')?.textContent).toBe('Studio Display Microphone');
  });
});

describe('ModelMenu', () => {
  async function open() {
    app = mount(ModelMenu, { target: document.body, props: { trigger: 'row', onopenproviders: vi.fn() } });
    await vi.waitFor(() => expect(document.querySelector('.model-trigger')?.textContent).toContain('Fast · In use'));
    const trigger = document.querySelector('.model-trigger') as HTMLElement;
    trigger.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    await vi.waitFor(() => expect(document.querySelector('[role=listbox]')).not.toBeNull());
  }
  const options = () => [...document.querySelectorAll('[role=option]')];
  const choose = (el: Element) => {
    for (const clientX of [0, 20]) el.dispatchEvent(new MouseEvent('pointermove', { bubbles: true, clientX }));
    el.dispatchEvent(new MouseEvent('pointerup', { bubbles: true }));
  };
  const items = () => options().map((el) => el.textContent?.replace(/\s+/g, ' ').trim());

  it('groups models, server and cloud with their actions', async () => {
    await open();
    const headings = [...document.querySelectorAll('.heading')].map((h) => h.textContent);
    expect(headings).toEqual(['On this Mac', 'Your server', 'Cloud']);
    expect(items()).toEqual([
      'Fast In use',
      'Balanced Recommended 500 MB',
      'Accurate 1.5 GB',
      'More Whisper models… Tiny, Medium, Most accurate',
      'Connect a whisper.cpp server… by URL',
      'Add a cloud service… OpenAI, Groq, Deepgram',
    ]);
  });

  it('queues a not-downloaded model to load when ready', async () => {
    await open();
    choose(options()[1]);
    await vi.waitFor(() => expect(ipc.loadModel).toHaveBeenCalledWith('small'));
    expect(ipc.queueModelDownload).not.toHaveBeenCalled();
  });

  it('opens the Speech models sheet from More Whisper models', async () => {
    await open();
    choose(options()[3]);
    await vi.waitFor(() => expect(document.querySelector('[role=dialog]')?.textContent ?? '').toContain('Speech models'));
  });
});

describe('Speech models sheet data', () => {
  const progress = { model: 'large-v3-turbo', phase: 'downloading', percent: 38, bytes_downloaded: 1, total_bytes: 2 };

  it('does not refetch the catalog on download progress, only when a download ends', async () => {
    app = mount(SpeechModelsSheet, { target: document.body, props: { open: false } });
    await vi.waitFor(() => expect(bus.liveListeners).toBeGreaterThanOrEqual(3));
    await vi.waitFor(() => expect(ipc.getModels).toHaveBeenCalledTimes(1));

    for (let i = 0; i < 20; i++) bus.emit('model:progress', progress);
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(ipc.getModels).toHaveBeenCalledTimes(1);

    bus.emit('model:progress', { ...progress, phase: 'done' });
    await vi.waitFor(() => expect(ipc.getModels).toHaveBeenCalledTimes(2));
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(ipc.getModels).toHaveBeenCalledTimes(2);
  });

  it('deletes a downloaded model from its row menu', async () => {
    ipc.getSettings.mockResolvedValue({ model: 'small', language: 'auto', input_device: null });
    app = mount(SpeechModelsSheet, { target: document.body, props: { open: true } });
    const options = await vi.waitFor(() => {
      const button = document.querySelector('[aria-label="Fast model options"]');
      expect(button).not.toBeNull();
      return button as HTMLElement;
    });
    options.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    const del = await vi.waitFor(() => {
      const item = [...document.querySelectorAll('[role=menuitem]')].find((el) => el.textContent === 'Delete');
      expect(item).toBeDefined();
      return item as HTMLElement;
    });
    del.click();
    await vi.waitFor(() => expect(remove.deleteModel).toHaveBeenCalledWith('base'));
  });
});
