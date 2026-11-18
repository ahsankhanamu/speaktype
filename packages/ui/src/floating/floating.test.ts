import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { alignedOffset } from './aligned';
import { hint } from './hint.svelte';
import { truncate } from './truncate';
import Combobox from './Combobox/Combobox.svelte';
import MenuFixture from './Menu/menu.fixture.svelte';

const rect = (top: number) => ({ top }) as DOMRect;

describe('alignedOffset', () => {
  test('puts the selected label over the trigger label', () => {
    expect(alignedOffset(rect(300), 120, 260, 800)).toBe(180);
  });

  test('clamps to the bottom edge with an 8px margin', () => {
    expect(alignedOffset(rect(600), 20, 260, 640)).toBe(640 - 260 - 8);
  });

  test('clamps to the top edge with an 8px margin', () => {
    expect(alignedOffset(rect(40), 120, 260, 800)).toBe(8);
  });
});

describe('truncate', () => {
  let observers: { disconnect: () => void }[];

  beforeEach(() => {
    vi.useFakeTimers();
    observers = [];
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe() {}
        disconnect = vi.fn();
        constructor() {
          observers.push(this);
        }
      },
    );
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    hint.anchor = null;
  });

  function makeNode(scrollWidth: number, clientWidth: number) {
    const node = document.createElement('span');
    Object.defineProperty(node, 'scrollWidth', { value: scrollWidth, configurable: true });
    Object.defineProperty(node, 'clientWidth', { value: clientWidth, configurable: true });
    return node;
  }

  test('attaches no tooltip for text that fits', () => {
    const node = makeNode(80, 100);
    truncate(node, { mode: 'tail', full: 'Short' });
    node.dispatchEvent(new Event('mouseenter'));
    vi.advanceTimersByTime(600);
    expect(hint.anchor).toBeNull();
  });

  test('attaches one after 500 ms for overflowing text', () => {
    const node = makeNode(200, 100);
    truncate(node, { mode: 'tail', full: 'A very long transcript line' });
    node.dispatchEvent(new Event('mouseenter'));
    vi.advanceTimersByTime(499);
    expect(hint.anchor).toBeNull();
    vi.advanceTimersByTime(1);
    expect(hint.anchor).toBe(node);
    expect(hint.text).toBe('A very long transcript line');
    node.dispatchEvent(new Event('mouseleave'));
    expect(hint.anchor).toBeNull();
  });

  test('re-entering restarts the delay instead of stacking timers', () => {
    const node = makeNode(200, 100);
    truncate(node, { mode: 'tail', full: 'A very long transcript line' });
    node.dispatchEvent(new Event('mouseenter'));
    vi.advanceTimersByTime(300);
    node.dispatchEvent(new Event('mouseenter'));
    vi.advanceTimersByTime(300);
    expect(hint.anchor).toBeNull();
    vi.advanceTimersByTime(200);
    expect(hint.anchor).toBe(node);
  });

  test('removes every listener and the observer on destroy', () => {
    const node = makeNode(200, 100);
    const add = vi.spyOn(node, 'addEventListener');
    const remove = vi.spyOn(node, 'removeEventListener');
    const action = truncate(node, { mode: 'tail', full: 'x' });
    action.destroy();
    expect(remove.mock.calls.map(([type]) => type).sort()).toEqual(add.mock.calls.map(([type]) => type).sort());
    expect(observers[0].disconnect).toHaveBeenCalled();
  });

  test('destroy cancels a pending tooltip', () => {
    const node = makeNode(200, 100);
    const action = truncate(node, { mode: 'tail', full: 'x' });
    node.dispatchEvent(new Event('mouseenter'));
    action.destroy();
    vi.advanceTimersByTime(600);
    expect(hint.anchor).toBeNull();
  });
});

describe('MenuItem', () => {
  test('an item without an icon keeps the icon column', () => {
    render(MenuFixture);
    const plain = screen.getByRole('menuitem', { name: 'Reset' });
    const icon = plain.querySelector('.icon') as HTMLElement;
    expect(icon).not.toBeNull();
    expect(icon.children.length).toBe(0);
    expect(plain.querySelector('.check')).not.toBeNull();
  });

  test('the checked radio item shows its check', () => {
    render(MenuFixture);
    const checked = screen.getByRole('menuitemradio', { name: 'Right' });
    expect(checked.querySelector('.check svg')).not.toBeNull();
    expect(screen.getByRole('menuitemradio', { name: 'Left' }).querySelector('.check svg')).toBeNull();
  });
});

describe('Combobox', () => {
  const items = [
    { value: 'pt', label: 'Portuguese' },
    { value: 'pt-br', label: 'Portuguese (Brazil)' },
    { value: 'es', label: 'Spanish' },
  ];

  test('typing filters, bolds the match and reports the count', async () => {
    render(Combobox, { items, value: 'pt-br', label: 'Language' });
    const input = screen.getByRole('combobox', { name: 'Language' }) as HTMLInputElement;
    await fireEvent.keyDown(input, { key: 'p' });
    await fireEvent.input(input, { target: { value: 'port' } });
    const options = await screen.findAllByRole('option');
    expect(options).toHaveLength(2);
    expect(options[0].querySelector('b')!.textContent).toBe('Port');
    expect(screen.getByText('2')).toBeTruthy();
  });

  test('no matches shows the empty message', async () => {
    render(Combobox, { items, value: 'pt', label: 'Language', emptyMessage: (q: string) => `No languages match “${q}”` });
    const input = screen.getByRole('combobox', { name: 'Language' });
    await fireEvent.keyDown(input, { key: 'x' });
    await fireEvent.input(input, { target: { value: 'xyz' } });
    expect(await screen.findByText('No languages match “xyz”')).toBeTruthy();
  });
});
