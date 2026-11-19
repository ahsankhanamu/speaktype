import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Alert } from '@speaktype/ui';
import { setBus } from '../lib/data';
import { FakeBus } from '../lib/data/testBus';
import Keyboard from './pages/Keyboard.svelte';
import { filter } from './search';

const api = vi.hoisted(() => ({ shortcutsGet: vi.fn(), shortcutSet: vi.fn(), shortcutsReset: vi.fn() }));
vi.mock('./shortcuts', () => api);

const DEFAULTS = [
  { action: 'dictation', keys: 'F8' },
  { action: 'session', keys: 'Option+Super+K' },
  { action: 'screenshot', keys: 'Option+Super+S' },
  { action: 'drawKey', keys: 'right_option' },
];

let bus: FakeBus;
const mounted: ReturnType<typeof mount>[] = [];
const text = () => document.body.textContent ?? '';
const button = (name: string) => [...document.querySelectorAll('button')].find((el) => el.textContent?.trim() === name) as HTMLButtonElement;
const row = (label: string) => [...document.querySelectorAll('.st-row')].find((el) => el.querySelector('.text .label')?.textContent === label) as HTMLElement;
const caps = (label: string) => [...row(label).querySelectorAll('.st-shortcut kbd')].map((el) => el.textContent?.trim());

function record(label: string, key: string, mods: { ctrlKey?: boolean; altKey?: boolean; metaKey?: boolean } = {}) {
  (row(label).querySelector('[aria-label="Record shortcut"]') as HTMLElement).click();
  flushSync();
  window.dispatchEvent(new KeyboardEvent('keydown', { key, code: key.length === 1 ? `Key${key}` : key, bubbles: true, ...mods }));
  flushSync();
}

async function open() {
  mounted.push(mount(Keyboard, { target: document.body }), mount(Alert, { target: document.body }));
  flushSync();
  await vi.waitFor(() => expect(caps('Start / stop dictation')).toEqual(['F8']));
}

beforeEach(() => {
  bus = new FakeBus();
  setBus(bus);
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} });
  Element.prototype.animate ??= function () {
    return { cancel() {}, set onfinish(done: () => void) { queueMicrotask(done); } } as unknown as Animation;
  };
  window.matchMedia ??= (() => ({ matches: false, addEventListener() {}, removeEventListener() {} })) as unknown as typeof window.matchMedia;
  api.shortcutsGet.mockResolvedValue(DEFAULTS);
  api.shortcutSet.mockResolvedValue(null);
  api.shortcutsReset.mockResolvedValue(DEFAULTS);
});

afterEach(async () => {
  for (const app of mounted.splice(0)) await unmount(app);
  document.body.innerHTML = '';
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});

describe('Settings → Keyboard', () => {
  it('lists the four changeable shortcuts, then the fixed ones, in Apple modifier order', async () => {
    await open();
    const labels = [...document.querySelectorAll('.st-row .text .label')].map((el) => el.textContent);
    expect(labels).toEqual([
      'Start / stop dictation', 'Start / stop a session', 'Take a screenshot', 'Draw (hold)',
      'Cancel recording', 'Undo last stroke', 'Copy prompt', 'Open History', 'Open Settings', 'Quit',
    ]);
    expect(caps('Start / stop a session')).toEqual(['⌥', '⌘', 'K']);
    expect([...row('Quit').querySelectorAll('kbd')].map((el) => el.textContent?.trim())).toEqual(['⌘', 'Q']);
  });

  it('saves a recorded shortcut for that action only', async () => {
    await open();
    record('Take a screenshot', 'P', { ctrlKey: true, altKey: true, metaKey: true });
    await vi.waitFor(() => expect(api.shortcutSet).toHaveBeenCalledWith('screenshot', 'Control+Option+Super+P', false));
    expect(api.shortcutSet).toHaveBeenCalledTimes(1);
  });

  it('shows an amber "Already used for" message on a conflict and keeps the old keys', async () => {
    api.shortcutSet.mockRejectedValue({ kind: 'conflict', message: 'Already used for Start / stop a session' });
    await open();
    record('Take a screenshot', 'K', { altKey: true, metaKey: true });
    await vi.waitFor(() => expect(row('Take a screenshot').textContent).toContain('Already used for Start / stop a session'));
    expect(row('Take a screenshot').querySelector('.conflict')).not.toBeNull();
    expect(caps('Take a screenshot')).toEqual(['⌥', '⌘', 'S']);
    expect(button('Use anyway')).toBeUndefined();
  });

  it('asks "Use anyway" for a system shortcut, then resends with force', async () => {
    api.shortcutSet.mockRejectedValueOnce({ kind: 'systemShortcut', message: 'macOS uses this for Spotlight' });
    await open();
    record('Start / stop a session', 'Space', { metaKey: true });
    await vi.waitFor(() => expect(text()).toContain('macOS uses this for Spotlight'));
    expect(api.shortcutSet).toHaveBeenLastCalledWith('session', 'Super+Space', false);
    button('Use anyway').click();
    await vi.waitFor(() => expect(api.shortcutSet).toHaveBeenLastCalledWith('session', 'Super+Space', true));
    await vi.waitFor(() => expect(button('Use anyway')).toBeUndefined());
    expect(text()).not.toContain('macOS uses this for');
  });

  it('keeps the previous keys and shows the message when registration fails', async () => {
    api.shortcutSet.mockRejectedValue({ kind: 'registerFailed', message: "Couldn't use this shortcut. Another app may own it." });
    await open();
    record('Start / stop dictation', 'D', { ctrlKey: true, altKey: true });
    await vi.waitFor(() => expect(row('Start / stop dictation').textContent).toContain("Couldn't use this shortcut. Another app may own it."));
    expect(caps('Start / stop dictation')).toEqual(['F8']);
  });

  it('picks a dictation preset from the pop-up beside the field', async () => {
    await open();
    (document.querySelector('[aria-label="Shortcut presets"]') as HTMLElement).dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    const items = await vi.waitFor(() => {
      const found = [...document.querySelectorAll('[role=menuitemradio], [role=menuitem]')];
      expect(found.length).toBe(4);
      return found;
    });
    expect(items.map((el) => el.textContent?.trim())).toEqual(['⌃⌘ (default)', 'F9', 'F8', 'Record a shortcut…']);
    (items[1] as HTMLElement).click();
    await vi.waitFor(() => expect(api.shortcutSet).toHaveBeenCalledWith('dictation', 'F9', false));
  });

  it('sets the draw key from its pop-up', async () => {
    await open();
    (document.querySelector('[aria-label="Draw (hold)"]') as HTMLElement).dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    const items = await vi.waitFor(() => {
      const found = [...document.querySelectorAll('[role=menuitemradio]')];
      expect(found.length).toBe(3);
      return found;
    });
    expect(items.map((el) => el.textContent?.trim())).toEqual(['Right ⌥', 'Right ⌘', 'Fn']);
    (items[1] as HTMLElement).click();
    await vi.waitFor(() => expect(api.shortcutSet).toHaveBeenCalledWith('drawKey', 'right_command', false));
  });

  it('Restore defaults asks first, and resets only after Restore', async () => {
    await open();
    button('Restore defaults').click();
    await vi.waitFor(() => expect(text()).toContain('Restore default shortcuts?'));
    expect(api.shortcutsReset).not.toHaveBeenCalled();
    button('Cancel').click();
    await vi.waitFor(() => expect(text()).not.toContain('Restore default shortcuts?'));
    expect(api.shortcutsReset).not.toHaveBeenCalled();

    button('Restore defaults').click();
    await vi.waitFor(() => expect(text()).toContain('Restore default shortcuts?'));
    button('Restore').click();
    await vi.waitFor(() => expect(api.shortcutsReset).toHaveBeenCalledTimes(1));
  });

  it('never writes shortcuts on its own', async () => {
    await open();
    bus.emit('shortcuts-changed', null);
    await vi.waitFor(() => expect(api.shortcutsGet).toHaveBeenCalledTimes(2));
    expect(api.shortcutSet).not.toHaveBeenCalled();
    expect(api.shortcutsReset).not.toHaveBeenCalled();
  });

  it('follows shortcuts-changed', async () => {
    await open();
    api.shortcutsGet.mockResolvedValue([{ action: 'dictation', keys: 'F9' }, ...DEFAULTS.slice(1)]);
    bus.emit('shortcuts-changed', null);
    await vi.waitFor(() => expect(caps('Start / stop dictation')).toEqual(['F9']));
  });
});

describe('search', () => {
  it.each([
    ['dictation', 'Start / stop dictation'],
    ['session', 'Start / stop a session'],
    ['screenshot', 'Take a screenshot'],
    ['draw', 'Draw (hold)'],
    ['restore', 'Restore defaults'],
  ])('finds "%s" on Keyboard', (query, label) => {
    expect(filter(query).find((entry) => entry.page === 'keyboard' && entry.label === label)).toBeDefined();
  });
});
