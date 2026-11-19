import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setBus } from '../lib/data';
import { FakeBus } from '../lib/data/testBus';

const ipc = vi.hoisted(() => ({
  getOnboardingStatus: vi.fn(),
  getSettings: vi.fn(),
  saveSettings: vi.fn(),
  finishOnboarding: vi.fn(),
  quitApp: vi.fn(),
  openSystemPane: vi.fn(),
  openSettings: vi.fn(),
  requestMicrophoneAccess: vi.fn(),
  getModels: vi.fn(),
  loadModel: vi.fn(),
  queueModelDownload: vi.fn(),
  pauseModelDownload: vi.fn(),
  cancelModelDownload: vi.fn(),
  getInputDevices: vi.fn(),
  reprocessPreview: vi.fn(),
}));
const mic = vi.hoisted(() => ({ micLevelsStart: vi.fn(), micLevelsStop: vi.fn() }));
const motion = vi.hoisted(() => ({ step: vi.fn() }));
vi.mock('../lib/ipc', () => ipc);
vi.mock('../lib/app/micLevels', () => mic);
vi.mock('../lib/app/deleteModel', () => ({ deleteModel: vi.fn() }));
vi.mock('@speaktype/ui/motion', async (original) => {
  const real = await original<typeof import('@speaktype/ui/motion')>();
  motion.step.mockImplementation(real.step);
  return { ...real, step: motion.step };
});

const { default: Onboarding } = await import('./Onboarding.svelte');

const entry = (id: string, over = {}) => ({ id, size_bytes: 0, downloaded: false, ...over });
const catalog = () => ({
  available: ['tiny', 'base', 'small', 'medium', 'large-v3-turbo', 'large-v3'].map((id) => entry(id)),
  downloading: [],
  waiting: [],
  progress: [],
});
const granted = { microphone: true, accessibility: true, model: true, active_model: 'base', exe_path: '', is_dev: false, is_bundled: true, responsible_app: '', responsible_app_path: '' };

let bus: FakeBus;
let app: ReturnType<typeof mount> | undefined;

const text = () => document.body.textContent ?? '';
const button = (name: string) => [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === name) as HTMLButtonElement;
const shown = (name: string) => vi.waitFor(() => expect(button(name)).toBeDefined());
const at = (label: string) => vi.waitFor(() => expect(text()).toContain(label));
const start = (settings = {}, status = {}) => {
  ipc.getSettings.mockResolvedValue({ model: 'base', language: 'auto', input_device: null, hotkey: 'Super+Control', onboarding_step: 0, ...settings });
  ipc.getOnboardingStatus.mockResolvedValue({ ...granted, ...status });
  app = mount(Onboarding, { target: document.body });
};

beforeEach(() => {
  bus = new FakeBus();
  setBus(bus);
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  Element.prototype.animate ??= function () {
    return { cancel() {}, set onfinish(done: () => void) { queueMicrotask(done); } } as unknown as Animation;
  };
  window.matchMedia ??= (() => ({ matches: false, addEventListener() {}, removeEventListener() {} })) as unknown as typeof window.matchMedia;
  ipc.getModels.mockResolvedValue(catalog());
  ipc.getInputDevices.mockResolvedValue({ devices: [{ name: 'MacBook Pro Microphone', label: 'MacBook Pro Microphone', is_default: true }], default: 'MacBook Pro Microphone' });
  ipc.saveSettings.mockResolvedValue(undefined);
  ipc.requestMicrophoneAccess.mockResolvedValue(true);
  ipc.queueModelDownload.mockResolvedValue({ status: 'queued' });
  ipc.loadModel.mockResolvedValue({ status: 'started' });
  ipc.finishOnboarding.mockResolvedValue(null);
});

afterEach(async () => {
  // liveQuery keeps released entries for 30 s; evict them so each test starts from fresh data.
  vi.useRealTimers();
  vi.useFakeTimers({ toFake: ['setTimeout'] });
  if (app) await unmount(app);
  app = undefined;
  vi.advanceTimersByTime(30_000);
  document.body.innerHTML = '';
  vi.clearAllMocks();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('steps', () => {
  it('runs Welcome, Microphone, Accessibility, Speech model, Try it, numbering all but Welcome', async () => {
    start();
    await shown('Get started');
    expect(text()).toContain('Welcome to SpeakType.');
    expect(text()).not.toContain('Step');
    const seen: string[] = [];
    for (const label of ['Step 1 of 4', 'Step 2 of 4', 'Step 3 of 4', 'Step 4 of 4']) {
      button(seen.length ? 'Continue' : 'Get started').click();
      await vi.waitFor(() => expect(text()).toContain(label));
      await vi.waitFor(() => expect(document.querySelectorAll('h1')).toHaveLength(1));
      seen.push(document.querySelector('h1')!.textContent!);
    }
    expect(seen).toEqual(['Let SpeakType hear you.', 'Let SpeakType type for you.', 'Pick how SpeakType listens.', 'Try it once.']);
    expect(button('Done')).toBeDefined();
  });

  it('slides with the direction of travel', async () => {
    start({ onboarding_step: 1 });
    await at('Step 1 of 4');
    button('Continue').click();
    await vi.waitFor(() => expect(motion.step).toHaveBeenCalledWith(expect.anything(), { direction: 1 }, expect.anything()));
    button('Back').click();
    await vi.waitFor(() => expect(motion.step).toHaveBeenCalledWith(expect.anything(), { direction: -1 }, expect.anything()));
  });

  it('resumes at the saved step and saves each change', async () => {
    start({ onboarding_step: 2 });
    await vi.waitFor(() => expect(text()).toContain('Step 2 of 4'));
    button('Continue').click();
    await vi.waitFor(() => expect(ipc.saveSettings).toHaveBeenCalledWith(expect.objectContaining({ onboarding_step: 3 })));
  });

  it('keeps Continue disabled until the permission is granted, and does not leave the picker', async () => {
    start({ onboarding_step: 1 }, { microphone: false });
    await at('Step 1 of 4');
    expect(button('Continue').disabled).toBe(true);
    ipc.getOnboardingStatus.mockResolvedValue(granted);
    bus.emit('sidecar:permissions', {});
    await vi.waitFor(() => expect(button('Continue').disabled).toBe(false));
    expect(text()).toContain('Step 1 of 4');
  });

  it('advances by itself when Accessibility is granted while waiting, not when it already was', async () => {
    start({ onboarding_step: 2 }, { accessibility: false });
    await at('Step 2 of 4');
    expect(button('Continue').disabled).toBe(true);
    ipc.getOnboardingStatus.mockResolvedValue(granted);
    bus.emit('sidecar:permissions', {});
    await vi.waitFor(() => expect(text()).toContain('Step 3 of 4'));
    button('Back').click();
    await vi.waitFor(() => expect(text()).toContain('Step 2 of 4'));
    await new Promise((resolve) => setTimeout(resolve, 100));
    expect(text()).toContain('Step 2 of 4');
  });

  it('polls the status every 2 s', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    start();
    flushSync();
    const before = ipc.getOnboardingStatus.mock.calls.length;
    vi.advanceTimersByTime(2000);
    expect(ipc.getOnboardingStatus.mock.calls.length).toBe(before + 1);
  });
});

describe('recovery paths', () => {
  it('offers Quit SpeakType once the microphone settings were opened', async () => {
    start({ onboarding_step: 1 }, { microphone: false });
    await at('Step 1 of 4');
    await shown('Open System Settings');
    expect(button('Quit SpeakType')).toBeUndefined();
    button('Open System Settings').click();
    expect(ipc.openSystemPane).toHaveBeenCalledWith('com.apple.preference.security?Privacy_Microphone');
    await shown('Quit SpeakType');
    button('Quit SpeakType').click();
    expect(ipc.quitApp).toHaveBeenCalled();
  });

  it('keeps the stale-entry help behind "Not working?" until 15 s of waiting', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout'] });
    start({ onboarding_step: 2 }, { accessibility: false, exe_path: '/Applications/SpeakType.app/Contents/MacOS/speaktype', responsible_app_path: '/Applications/SpeakType.app' });
    for (let i = 0; i < 20; i++) await vi.advanceTimersByTimeAsync(0);
    expect(text()).toContain("Waiting for you to switch it on. We'll continue automatically.");
    await vi.advanceTimersByTimeAsync(14_000);
    expect(button('Not working?')).toBeUndefined();
    expect(text()).not.toContain('Remove SpeakType from the Accessibility list');
    await vi.advanceTimersByTimeAsync(1_000);
    expect(button('Not working?')).toBeDefined();
    expect(text()).not.toContain('Remove SpeakType from the Accessibility list');
    button('Not working?').click();
    await vi.advanceTimersByTimeAsync(0);
    expect(text()).toContain('Remove SpeakType from the Accessibility list');
    expect(text()).toContain('Grant to: /Applications/SpeakType.app');
    button('Quit & Reopen SpeakType').click();
    expect(ipc.quitApp).toHaveBeenCalled();
  });

  it('shows the dev-build hints', async () => {
    start({ onboarding_step: 2 }, { accessibility: false, is_dev: true, is_bundled: false, responsible_app: 'Terminal' });
    await vi.waitFor(() => expect(text()).toContain('This is a bare executable, so macOS assigns Accessibility to Terminal'));
    expect(text()).not.toContain('Remove SpeakType from the Accessibility list');
    expect(button('Not working?')).toBeUndefined();
  });

  it('shows the bundled dev-build hint', async () => {
    start({ onboarding_step: 2 }, { accessibility: false, is_dev: true, is_bundled: true, responsible_app: 'SpeakType Dev' });
    await vi.waitFor(() => expect(text()).toContain('Dev build: enable "SpeakType Dev"'));
  });
});

describe('downloads', () => {
  const loaded = () => ipc.loadModel.mock.calls.map(([id]) => id);

  it('switches to Fast first on a new install, then to the pre-set Balanced', async () => {
    start({ onboarding_step: 1 });
    await vi.waitFor(() => expect(loaded()).toEqual(['base']));
    await at('Step 1 of 4');
    button('Continue').click();
    await at('Step 2 of 4');
    button('Continue').click();
    await vi.waitFor(() => expect(loaded()).toEqual(['base', 'small']));
    expect(ipc.queueModelDownload).not.toHaveBeenCalled();
  });

  it('loads the chosen tier when the slider moves', async () => {
    start({ onboarding_step: 3 });
    const slider = await vi.waitFor(() => {
      const el = document.querySelector('input[type=range]') as HTMLInputElement;
      expect(el).not.toBeNull();
      return el;
    });
    expect(slider.getAttribute('aria-valuetext')).toBe('Balanced');
    await vi.waitFor(() => expect(loaded().filter((id: string) => id === 'small')).toHaveLength(1));
    slider.value = '2';
    slider.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.waitFor(() => expect(loaded()).toContain('large-v3-turbo'));
    expect(ipc.queueModelDownload).not.toHaveBeenCalled();
  });

  it('does not load anything when a download finishes', async () => {
    start({ onboarding_step: 3 });
    await vi.waitFor(() => expect(loaded()).toContain('small'));
    const before = loaded().length;
    await vi.waitFor(() => expect(bus.liveListeners).toBeGreaterThan(0));
    bus.emit('model:progress', { model: 'small', phase: 'done' });
    await new Promise((resolve) => setTimeout(resolve, 100));
    expect(loaded()).toHaveLength(before);
  });

  it('summarises the downloads in the footer', async () => {
    ipc.getModels.mockResolvedValue({
      ...catalog(),
      available: catalog().available.map((m) => (m.id === 'base' ? { ...m, downloaded: true } : m)),
      progress: [{ model: 'small', phase: 'downloading', percent: 64, bytes_downloaded: 1, total_bytes: 2 }],
    });
    start({ onboarding_step: 3 });
    await vi.waitFor(() => expect(text()).toContain('Fast ready · Balanced 64%'));
  });
});

describe('Try it', () => {
  it('shows the configured hotkey as keycaps', async () => {
    start({ onboarding_step: 4, hotkey: 'Super+Control' });
    await vi.waitFor(() => expect([...document.querySelectorAll('kbd .glyph')].map((k) => k.textContent?.trim())).toEqual(['⌃', '⌘']));
    expect([...document.querySelectorAll('kbd .word')].map((k) => k.textContent)).toEqual(['control', 'command']);
  });

  it('celebrates the first dictation and finishes on Done', async () => {
    start({ onboarding_step: 4 });
    const field = await vi.waitFor(() => {
      const el = document.querySelector('textarea') as HTMLTextAreaElement;
      expect(el).not.toBeNull();
      return el;
    });
    expect(text()).not.toContain('That worked');
    field.value = 'Hi, this is my first note';
    field.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.waitFor(() => expect(text()).toContain('That worked'));
    button('Done').click();
    await vi.waitFor(() => expect(ipc.finishOnboarding).toHaveBeenCalledTimes(1));
  });

  it('keeps Done disabled until a model is ready', async () => {
    start({ onboarding_step: 4 }, { model: false });
    await shown('Done');
    expect(button('Done').disabled).toBe(true);
  });

  describe('re-run with another model', () => {
    const downloadedFast = () => ({
      ...catalog(),
      available: catalog().available.map((m) => (['base', 'small'].includes(m.id) ? { ...m, downloaded: true } : m)),
    });
    const dictate = async () => {
      const field = await vi.waitFor(() => {
        const el = document.querySelector('textarea') as HTMLTextAreaElement;
        expect(el).not.toBeNull();
        return el;
      });
      field.value = 'Hi, this is my first note';
      field.dispatchEvent(new Event('input', { bubbles: true }));
      await at('That worked');
    };
    const switchTo = async (model: string) => {
      ipc.getSettings.mockResolvedValue({ model, language: 'auto', input_device: null, hotkey: 'Super+Control', onboarding_step: 4 });
      bus.emit('settings-changed', {});
    };

    beforeEach(() => {
      ipc.getModels.mockResolvedValue(downloadedFast());
      ipc.reprocessPreview.mockResolvedValue('Hi, this is my first note with SpeakType.');
    });

    it('does nothing before the first dictation', async () => {
      start({ onboarding_step: 4 });
      await at('Try it once.');
      await vi.waitFor(() => expect(bus.liveListeners).toBeGreaterThan(0));
      await switchTo('small');
      await new Promise((resolve) => setTimeout(resolve, 100));
      expect(ipc.reprocessPreview).not.toHaveBeenCalled();
    });

    it('shows Getting ready… while the other model loads', async () => {
      start({ onboarding_step: 4 });
      await dictate();
      ipc.getModels.mockResolvedValue({ ...downloadedFast(), progress: [{ model: 'small', phase: 'loading' }] });
      bus.emit('model:progress', { model: 'small', phase: 'done' });
      await at('Getting ready…');
      expect(ipc.reprocessPreview).not.toHaveBeenCalled();
    });

    it('re-runs the practice entry once the model is in use and offers Keep or Back', async () => {
      start({ onboarding_step: 4 });
      await dictate();
      await switchTo('small');
      await vi.waitFor(() => expect(ipc.reprocessPreview).toHaveBeenCalledWith(0));
      await at('Hi, this is my first note with SpeakType.');
      expect(button('Keep Balanced')).toBeDefined();
      expect(button('Back to Fast')).toBeDefined();
    });

    it('goes back to the previous model with loadModel', async () => {
      start({ onboarding_step: 4 });
      await dictate();
      await switchTo('small');
      await shown('Back to Fast');
      button('Back to Fast').click();
      expect(ipc.loadModel).toHaveBeenCalledWith('base');
      await vi.waitFor(() => expect(button('Keep Balanced')).toBeUndefined());
    });

    it('keeps the new model and closes the comparison', async () => {
      start({ onboarding_step: 4 });
      await dictate();
      await switchTo('small');
      await shown('Keep Balanced');
      button('Keep Balanced').click();
      await vi.waitFor(() => expect(button('Keep Balanced')).toBeUndefined());
      expect(ipc.loadModel).not.toHaveBeenCalled();
    });
  });
});
