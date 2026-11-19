import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setBus } from '../lib/data';
import { FakeBus } from '../lib/data/testBus';

const ipc = vi.hoisted(() => ({
  commands: { pasteLast: vi.fn(), openSettings: vi.fn() },
  getSettings: vi.fn(),
  getOnboardingStatus: vi.fn(),
  toggleRecording: vi.fn(),
  cancelRecording: vi.fn(),
  openSettings: vi.fn(),
  openOnboarding: vi.fn(),
  hideWidget: vi.fn(),
  saveWindowPosition: vi.fn(),
  getModels: vi.fn(),
  getInputDevices: vi.fn(),
  saveSettings: vi.fn(),
  loadModel: vi.fn(),
  queueModelDownload: vi.fn(),
  pauseModelDownload: vi.fn(),
  cancelModelDownload: vi.fn(),
}));
const mic = vi.hoisted(() => ({ micLevelsStart: vi.fn(), micLevelsStop: vi.fn() }));
const win = vi.hoisted(() => ({ startDragging: vi.fn() }));
const fit = vi.hoisted(() => ({ layout: vi.fn() }));
vi.mock('../lib/ipc', () => ipc);
vi.mock('../lib/app/micLevels', () => mic);
vi.mock('../lib/app/deleteModel', () => ({ deleteModel: vi.fn() }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => win }));
vi.mock('./windowFit', async (original) => ({ ...(await original<typeof import('./windowFit')>()), createLayout: () => fit.layout }));

const { default: Widget } = await import('./Widget.svelte');

const granted = { microphone: true, accessibility: true, model: true };
const entry = (id: string, over = {}) => ({ id, size_bytes: 0, downloaded: false, ...over });

let bus: FakeBus;
let app: ReturnType<typeof mount> | undefined;

const text = () => document.body.textContent ?? '';
const button = (name: string) =>
  [...document.querySelectorAll<HTMLElement>('button, [role="menuitem"]')].find(
    (b) => b.textContent?.trim() === name || b.getAttribute('aria-label') === name,
  );
const settle = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve();
  flushSync();
};
const start = async (settings = {}, status = {}) => {
  ipc.getSettings.mockResolvedValue({ hotkey: 'Super+Control', widget_size: 'regular', model: 'base', input_device: null, ...settings });
  ipc.getOnboardingStatus.mockResolvedValue({ ...granted, ...status });
  app = mount(Widget, { target: document.body });
  await settle();
};
const send = async (phase: string, kind?: string, message = '') => {
  bus.emit('dictation-state', { phase, notice: kind ? { kind, message } : null });
  await settle();
};
const halo = () => document.querySelector<HTMLElement>('.halo')!.style.boxShadow;
/** The room the window was last asked to keep around the pill, and the pill's edge inset. */
const lastFit = () => {
  const { room, inset } = fit.layout.mock.calls.at(-1)![0];
  return [room.width, room.height, inset];
};

beforeEach(() => {
  bus = new FakeBus();
  setBus(bus);
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  Element.prototype.animate ??= function () {
    return { cancel() {}, set onfinish(done: () => void) { queueMicrotask(done); } } as unknown as Animation;
  };
  window.matchMedia ??= (() => ({ matches: false, addEventListener() {}, removeEventListener() {} })) as unknown as typeof window.matchMedia;
  ipc.getModels.mockResolvedValue({
    available: ['base', 'small', 'large-v3-turbo', 'tiny', 'medium', 'large-v3'].map((id) => entry(id, { downloaded: id === 'base' })),
    downloading: [],
    waiting: [],
    progress: [],
  });
  for (const command of [ipc.openSettings, ipc.openOnboarding, ipc.hideWidget, ipc.cancelRecording, ipc.saveWindowPosition]) {
    command.mockResolvedValue(null);
  }
  ipc.getInputDevices.mockResolvedValue({
    devices: [{ name: 'MacBook Pro Microphone', label: 'MacBook Pro Microphone', is_default: true }],
    default: 'MacBook Pro Microphone',
  });
  ipc.toggleRecording.mockResolvedValue(null);
  ipc.commands.pasteLast.mockResolvedValue(null);
  ipc.commands.openSettings.mockResolvedValue(null);
  fit.layout.mockResolvedValue(undefined);
  win.startDragging.mockResolvedValue(undefined);
});

afterEach(async () => {
  vi.useRealTimers();
  vi.useFakeTimers({ toFake: ['setTimeout'] });
  if (app) await unmount(app);
  app = undefined;
  vi.advanceTimersByTime(30_000);
  vi.useRealTimers();
  document.body.innerHTML = '';
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});

describe('states on the board', () => {
  it('idle: a mic that starts dictation, with no halo', async () => {
    await start();
    expect(halo()).toBe('');
    button('Dictate')!.click();
    expect(ipc.toggleRecording).toHaveBeenCalledWith(false);
  });

  it('recording: waveform, timer, cancel and stop, red halo', async () => {
    await start();
    await send('recording');
    expect(halo()).toBe('var(--st-halo-recording)');
    expect(document.querySelector('[data-waveform]')).not.toBeNull();
    expect(text()).toContain('0:00');
    button('Cancel')!.click();
    expect(ipc.cancelRecording).toHaveBeenCalled();
    button('Stop')!.click();
    expect(ipc.toggleRecording).toHaveBeenCalledWith(true);
  });

  it('transcribing: a quiet pulse, no percentages, no halo', async () => {
    await start();
    await send('transcribing');
    expect(document.querySelector('[aria-label="Transcribing"]')).not.toBeNull();
    expect(halo()).toBe('');
    expect(text()).not.toContain('%');
  });

  it('getting ready: blue halo and the copy', async () => {
    await start();
    await send('preparing');
    expect(text()).toContain('Getting ready…');
    expect(halo()).toBe('var(--st-halo-active)');
  });

  it('no speech: amber halo, nothing to press, fades after 2 s', async () => {
    await start();
    vi.useFakeTimers({ toFake: ['setTimeout'] });
    await send('error', 'noSpeech', "Didn't catch that");
    expect(text()).toContain("Didn't catch that");
    expect(halo()).toBe('var(--st-halo-warning)');
    vi.advanceTimersByTime(1900);
    flushSync();
    expect(text()).toContain("Didn't catch that");
    vi.advanceTimersByTime(200);
    flushSync();
    expect(text()).not.toContain("Didn't catch that");
    expect(button('Dictate')).toBeDefined();
  });

  it('busy reads like no speech', async () => {
    await start();
    await send('error', 'busy', 'Still working on the last one');
    expect(text()).toContain('Still working on the last one');
    expect(halo()).toBe('var(--st-halo-warning)');
  });

  it('pasted: green check for half a second, then idle', async () => {
    await start();
    vi.useFakeTimers({ toFake: ['setTimeout'] });
    await send('done', 'pasted', 'Pasted');
    expect(halo()).toBe('var(--st-halo-success)');
    expect(document.querySelector('[aria-label="Pasted"]')).not.toBeNull();
    vi.advanceTimersByTime(600);
    flushSync();
    expect(document.querySelector('[aria-label="Pasted"]')).toBeNull();
    expect(button('Dictate')).toBeDefined();
  });

  it("couldn't paste: the copy with keys, Paste again calls pasteLast", async () => {
    await start();
    await send('done', 'copiedNotPasted', 'Copied. Press ⌘V to paste');
    expect(document.querySelector('.pill')!.textContent).toMatch(/Copied\. Press\s*⌘\s*V\s*to paste/);
    button('Paste again')!.click();
    expect(ipc.commands.pasteLast).toHaveBeenCalled();
    await settle();
    expect(button('Dictate')).toBeDefined();
  });

  it('microphone unavailable: Choose mic opens the picker, which starts mic levels', async () => {
    await start();
    await send('error', 'micUnavailable', 'Microphone unavailable');
    expect(text()).toContain('Microphone unavailable');
    expect(mic.micLevelsStart).not.toHaveBeenCalled();
    button('Choose mic')!.click();
    await vi.waitFor(() => expect(mic.micLevelsStart).toHaveBeenCalled());
    await vi.waitFor(() => expect(text()).toContain('System default'));
  });

  it('mic fallback: says so while still recording and keeps saying it until Stop', async () => {
    await start();
    vi.useFakeTimers({ toFake: ['setTimeout'] });
    await send('recording');
    const recordingFit = lastFit();
    await send('recording', 'micFallback', 'Switched to MacBook microphone');
    expect(text()).toContain('Switched to MacBook microphone');
    expect(halo()).toBe('var(--st-halo-recording)');
    expect(lastFit()).toEqual(recordingFit);
    vi.advanceTimersByTime(10_000);
    flushSync();
    expect(text()).toContain('Switched to MacBook microphone');
    await send('transcribing');
    expect(button('Stop')).toBeUndefined();
    expect(document.querySelector('[aria-label="Transcribing"]')).not.toBeNull();
  });

  it('provider fallback: the done notice names the service', async () => {
    await start();
    await send('done', 'providerFallback', "Couldn't reach Groq. Used On this Mac.");
    expect(text()).toContain("Couldn't reach Groq. Used On this Mac.");
    expect(halo()).toBe('');
  });

  it('server error: Try again presses the hotkey again', async () => {
    await start();
    await send('error', 'serverError', "Speech model couldn't start");
    expect(text()).toContain("Speech model couldn't start");
    button('Try again')!.click();
    expect(ipc.toggleRecording).toHaveBeenCalledWith(false);
  });

  it('long recording: the notice replaces the dots until the take ends', async () => {
    await start();
    await send('transcribing');
    bus.emit('dictation:long-audio', { duration_secs: 130 });
    await settle();
    expect(text()).toContain('Long recording, this may take a moment');
    await send('done', 'pasted', 'Pasted');
    expect(text()).not.toContain('Long recording');
  });

  it('a new state replaces a fading one at once', async () => {
    await start();
    await send('error', 'noSpeech', "Didn't catch that");
    await send('recording');
    expect(text()).not.toContain("Didn't catch that");
    expect(button('Stop')).toBeDefined();
  });
});

describe('size and window', () => {
  it.each([
    ['small', '30px'],
    ['regular', '40px'],
    ['large', '50px'],
  ])('widget_size %s is a %s pill', async (size, px) => {
    await start({ widget_size: size });
    await vi.waitFor(() => expect(document.querySelector<HTMLElement>('.widget')!.style.getPropertyValue('--size')).toBe(px));
  });

  it('hugs the idle bubble, which may sit right at the screen edge', async () => {
    await start();
    await vi.waitFor(() => expect(lastFit()).toEqual([0, 0, 0]));
  });

  it('keeps an opened pill inside the screen edges', async () => {
    await start();
    await send('recording');
    await vi.waitFor(() => expect(lastFit()).toEqual([0, 0, 8]));
  });

  it('grows while the idle pill is hovered and shrinks back after', async () => {
    await start();
    const stage = document.querySelector('.stage')!;
    stage.dispatchEvent(new MouseEvent('pointerenter'));
    await vi.waitFor(() => expect(lastFit()[1]).toBeGreaterThanOrEqual(120));
    stage.dispatchEvent(new MouseEvent('pointerleave'));
    await vi.waitFor(() => expect(lastFit()).toEqual([0, 0, 0]));
  });

  it('does not grow for hover once recording', async () => {
    await start();
    await send('recording');
    document.querySelector('.stage')!.dispatchEvent(new MouseEvent('pointerenter'));
    await settle();
    expect(lastFit()).toEqual([0, 0, 8]);
  });

  it('grows while the menu is open and returns to rest when it closes', async () => {
    await start();
    document.querySelector('.stage')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await vi.waitFor(() => expect(lastFit()[1]).toBeGreaterThan(500));
    button('Hide widget')!.click();
    await vi.waitFor(() => expect(lastFit()).toEqual([0, 0, 0]));
  });
});

describe('animation frames', () => {
  it('runs only while recording', async () => {
    const raf = vi.spyOn(window, 'requestAnimationFrame').mockImplementation(() => 7);
    const caf = vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(() => {});
    await start();
    await send('transcribing');
    expect(raf).not.toHaveBeenCalled();
    await send('recording');
    expect(raf).toHaveBeenCalled();
    await send('transcribing');
    expect(caf).toHaveBeenCalledWith(7);
    raf.mockClear();
    await send('idle');
    expect(raf).not.toHaveBeenCalled();
  });
});

describe('right-click menu', () => {
  const open = async () => {
    await start();
    document.querySelector('.stage')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await vi.waitFor(() => expect(button('Hide widget')).toBeDefined());
  };

  it('lists Microphone, Speech model, History, Settings and Hide widget', async () => {
    await open();
    for (const label of ['Microphone', 'Speech model', 'History', 'Settings', 'Hide widget']) expect(button(label)).toBeDefined();
  });

  it('opens Settings', async () => {
    await open();
    button('Settings')!.click();
    expect(ipc.openSettings).toHaveBeenCalledTimes(1);
  });

  it('opens the History page', async () => {
    await open();
    button('History')!.click();
    expect(ipc.commands.openSettings).toHaveBeenCalledWith('history');
  });

  it('hides the widget', async () => {
    await open();
    button('Hide widget')!.click();
    expect(ipc.hideWidget).toHaveBeenCalled();
  });

  it('Microphone opens the picker', async () => {
    await open();
    button('Microphone')!.click();
    await vi.waitFor(() => expect(mic.micLevelsStart).toHaveBeenCalled());
  });

  it('Speech model opens the model menu', async () => {
    await open();
    button('Speech model')!.click();
    await vi.waitFor(() => expect(document.querySelector('.model-trigger')).not.toBeNull());
  });
});

describe('setup and server', () => {
  it('shows a Finish setup dot while onboarding is unfinished, and the pill opens onboarding', async () => {
    await start({}, { accessibility: false });
    await vi.waitFor(() => expect(document.querySelector('[aria-label="Finish setup"]')).not.toBeNull());
    button('Dictate')!.click();
    expect(ipc.openOnboarding).toHaveBeenCalled();
    expect(ipc.toggleRecording).not.toHaveBeenCalled();
  });

  it('shows no dot once everything is granted', async () => {
    await start();
    await settle();
    expect(document.querySelector('[aria-label="Finish setup"]')).toBeNull();
  });

  it('follows the server: starting, error, then ready fades out', async () => {
    await start();
    vi.useFakeTimers({ toFake: ['setTimeout'] });
    bus.emit('server:starting', {});
    await settle();
    expect(document.querySelector('[aria-label="Server starting…"]')).not.toBeNull();
    bus.emit('server:error', { reason: 'boom' });
    await settle();
    expect(document.querySelector('[aria-label="Server error: boom"]')).not.toBeNull();
    bus.emit('server:ready', { port: 8080 });
    await settle();
    expect(document.querySelector('[aria-label="Server ready"]')).not.toBeNull();
    vi.advanceTimersByTime(3100);
    flushSync();
    expect(document.querySelector('[aria-label^="Server"]')).toBeNull();
  });

  it("shows the You're all set card after onboarding and dismisses it", async () => {
    await start();
    bus.emit('onboarding:finished', { hotkey: 'Super+Control' });
    await settle();
    expect(text()).toContain("You're all set");
    expect(text()).toContain('Click the mic or press ⌃⌘');
    button('Dismiss')!.click();
    await settle();
    expect(text()).not.toContain("You're all set");
  });

  it('presses on the pill that move it are drags, not clicks', async () => {
    await start();
    const stage = document.querySelector('.stage')!;
    stage.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 0, screenX: 10, screenY: 10 }));
    window.dispatchEvent(new MouseEvent('pointermove', { screenX: 30, screenY: 10 }));
    expect(win.startDragging).toHaveBeenCalledTimes(1);
    button('Dictate')!.click();
    expect(ipc.toggleRecording).not.toHaveBeenCalled();
  });
});

describe('lint policy', () => {
  it('the widget does not get a setInterval exemption: polling goes through liveQuery', async () => {
    const { default: config } = await import('../../eslint.config.js');
    const exempted = config.flatMap((entry) => (Array.isArray(entry.files) ? entry.files : []));
    expect(exempted).not.toContain('src/widget/Widget.svelte');
  });
});
