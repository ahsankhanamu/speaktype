import { fireEvent, render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { Search } from '@lucide/svelte';
import Button from './Button.svelte';
import IconButton from './IconButton.svelte';
import Keycap from './Keycap.svelte';
import SecureField from './SecureField.svelte';
import Checkbox from './Checkbox.svelte';
import SearchField from './SearchField.svelte';
import Segmented from './Segmented.svelte';
import ShortcutField from './ShortcutField.svelte';
import Switch from './Switch.svelte';
import TextField from './TextField.svelte';
import { formatShortcut } from './shortcut';

const text = (value: string) => createRawSnippet(() => ({ render: () => `<span>${value}</span>` }));

afterEach(() => vi.useRealTimers());

describe('formatShortcut', () => {
  test('orders modifiers ⌃ ⌥ ⇧ ⌘', () => {
    expect(formatShortcut(['Super', 'Control'])).toEqual(['⌃', '⌘']);
    expect(formatShortcut(['Shift', 'Super', 'Option', 'k'])).toEqual(['⌥', '⇧', '⌘', 'K']);
  });

  test('maps Alt to ⌥ and Enter to return', () => {
    expect(formatShortcut(['Alt', 'Enter'])).toEqual(['⌥', 'return']);
  });
});

describe('Keycap', () => {
  test('return renders an svg glyph and no ↩ text', () => {
    const { container } = render(Keycap, { key: 'return' });
    expect(container.querySelector('svg')).not.toBeNull();
    expect(container.textContent).not.toContain('↩');
  });

  test('modifiers render as text', () => {
    render(Keycap, { key: '⌘' });
    expect(screen.getByText('⌘')).toBeTruthy();
  });
});

describe('Button', () => {
  test('a disabled button ignores clicks and is aria-disabled', async () => {
    const onclick = vi.fn();
    render(Button, { children: text('Copy'), disabled: true, onclick });
    const button = screen.getByRole('button');
    await fireEvent.click(button);
    expect(onclick).not.toHaveBeenCalled();
    expect(button.getAttribute('aria-disabled')).toBe('true');
  });

  test('a busy button keeps its label, is aria-busy and ignores clicks', async () => {
    const onclick = vi.fn();
    render(Button, { children: text('Copy'), busy: true, onclick });
    const button = screen.getByRole('button');
    expect(button.getAttribute('aria-busy')).toBe('true');
    expect(screen.getByText('Copy')).toBeTruthy();
    await fireEvent.click(button);
    expect(onclick).not.toHaveBeenCalled();
  });

  test('a destructive button is never the default', () => {
    render(Button, { children: text('Delete'), variant: 'destructive' });
    expect(screen.getByRole('button').classList.contains('is-default')).toBe(false);
  });

  test('shortcut keys render as keycaps', () => {
    const { container } = render(Button, { children: text('Copy'), shortcut: ['Super', 'C'] });
    expect([...container.querySelectorAll('kbd')].map((k) => k.textContent)).toEqual(['⌘', 'C']);
  });
});

describe('IconButton', () => {
  test('its accessible name is the label', () => {
    render(IconButton, { label: 'Search', icon: Search });
    expect(screen.getByRole('button', { name: 'Search' })).toBeTruthy();
  });
});

describe('Segmented', () => {
  test('ArrowRight moves the value to the next item', async () => {
    const items = [
      { value: 'timeline', label: 'Timeline' },
      { value: 'result', label: 'Result' },
    ];
    const { component } = render(Segmented, { items, value: 'timeline' });
    const first = screen.getByRole('radio', { name: 'Timeline' });
    first.focus();
    await fireEvent.keyDown(first, { key: 'ArrowRight' });
    expect(screen.getByRole('radio', { name: 'Result' }).getAttribute('aria-checked')).toBe('true');
    expect(component).toBeTruthy();
  });
});

describe('SecureField', () => {
  const field = () => screen.getByLabelText('API key') as HTMLInputElement;

  test('re-masks after 30 s', async () => {
    vi.useFakeTimers();
    render(SecureField, { value: 'sk-1', revealed: true, 'aria-label': 'API key' });
    expect(field().type).toBe('text');
    await vi.advanceTimersByTimeAsync(30_000);
    expect(field().type).toBe('password');
  });

  test('re-masks when the window loses focus', async () => {
    render(SecureField, { value: 'sk-1', revealed: true, 'aria-label': 'API key' });
    await fireEvent.blur(window);
    expect(field().type).toBe('password');
  });

  test('reveal waits for onrequestreveal', async () => {
    const onrequestreveal = vi.fn().mockResolvedValue(false);
    render(SecureField, { value: 'sk-1', revealed: false, onrequestreveal, 'aria-label': 'API key' });
    await fireEvent.click(screen.getByRole('button', { name: 'Show' }));
    expect(onrequestreveal).toHaveBeenCalled();
    expect(field().type).toBe('password');
  });

  test('accepts paste while masked', async () => {
    render(SecureField, { value: '', revealed: false, 'aria-label': 'API key' });
    await fireEvent.input(field(), { target: { value: 'pasted' } });
    expect(field().value).toBe('pasted');
  });
});

describe('ShortcutField', () => {
  test('recording captures ⌥⌘K', async () => {
    const { component } = render(ShortcutField, { value: [] });
    await fireEvent.click(screen.getByRole('button', { name: 'Record shortcut' }));
    const recorder = screen.getByRole('button', { name: /press a shortcut/i });
    await fireEvent.keyDown(recorder, { key: '˚', code: 'KeyK', altKey: true, metaKey: true });
    expect([...document.querySelectorAll('kbd')].map((k) => k.textContent)).toEqual(['⌥', '⌘', 'K']);
    expect(component).toBeTruthy();
  });

  test('modifier-only presses keep recording', async () => {
    render(ShortcutField, { value: [] });
    await fireEvent.click(screen.getByRole('button', { name: 'Record shortcut' }));
    const recorder = screen.getByRole('button', { name: /press a shortcut/i });
    await fireEvent.keyDown(recorder, { key: 'Meta', code: 'MetaLeft', metaKey: true });
    expect(screen.getByRole('button', { name: /press a shortcut/i })).toBeTruthy();
  });

  test('Escape cancels recording and the clear button empties the value', async () => {
    render(ShortcutField, { value: ['Super', 'K'] });
    await fireEvent.click(screen.getByRole('button', { name: 'Record shortcut' }));
    await fireEvent.keyDown(screen.getByRole('button', { name: /press a shortcut/i }), { key: 'Escape', code: 'Escape' });
    expect(document.querySelectorAll('kbd').length).toBe(2);
    await fireEvent.click(screen.getByRole('button', { name: 'Clear shortcut' }));
    expect(document.querySelectorAll('kbd').length).toBe(0);
  });

  test('a conflict is announced under the field', () => {
    render(ShortcutField, { value: ['Option', 'Super', 'S'], conflict: 'Already used for screenshots' });
    expect(screen.getByText('Already used for screenshots')).toBeTruthy();
  });
});

describe('toggles and fields', () => {
  test('a switch toggles on click', async () => {
    render(Switch, { checked: false, label: 'Launch at login' });
    const toggle = screen.getByRole('switch', { name: 'Launch at login' });
    await fireEvent.click(toggle);
    expect(toggle.getAttribute('aria-checked')).toBe('true');
  });

  test('clicking a mixed checkbox checks it', async () => {
    render(Checkbox, { checked: 'mixed', label: 'Everything' });
    const box = screen.getByRole('checkbox', { name: 'Everything' });
    expect(box.getAttribute('aria-checked')).toBe('mixed');
    await fireEvent.click(box);
    expect(box.getAttribute('aria-checked')).toBe('true');
  });

  test('a text field links its error message', () => {
    render(TextField, { value: '~/missing', error: "That folder doesn't exist", 'aria-label': 'Folder' });
    const input = screen.getByLabelText('Folder');
    expect(input.getAttribute('aria-invalid')).toBe('true');
    expect(document.getElementById(input.getAttribute('aria-describedby')!)!.textContent).toBe("That folder doesn't exist");
  });

  test('the search field clears itself', async () => {
    render(SearchField, { value: 'port' });
    await fireEvent.click(screen.getByRole('button', { name: 'Clear search' }));
    expect((screen.getByRole('searchbox') as HTMLInputElement).value).toBe('');
  });
});
