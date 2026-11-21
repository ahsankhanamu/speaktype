import { readdirSync, readFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, test, vi } from 'vitest';
import Alert from './Alert.svelte';
import { confirmAlert } from './confirm.svelte';
import Row from './Row.svelte';
import SheetFixture from './sheet.fixture.svelte';
import Toast from './Toast.svelte';
import { toast, toasts } from './toasts.svelte';

afterEach(() => {
  toasts.length = 0;
  vi.useRealTimers();
});

describe('Sheet', () => {
  test('Escape closes it and focus returns to the opener', async () => {
    render(SheetFixture);
    const opener = screen.getByRole('button', { name: 'Open' });
    opener.focus();
    await fireEvent.click(opener);
    const dialog = await screen.findByRole('dialog', { name: 'Speech models' });
    await fireEvent.keyDown(dialog, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(opener));
  });

  test('Tab cycles inside the sheet', async () => {
    render(SheetFixture);
    await fireEvent.click(screen.getByRole('button', { name: 'Open' }));
    const done = await screen.findByRole('button', { name: 'Done' });
    done.focus();
    await fireEvent.keyDown(done, { key: 'Tab' });
    const dialog = screen.getByRole('dialog');
    expect(dialog.contains(document.activeElement)).toBe(true);
  });
});

describe('confirmAlert', () => {
  test('Cancel is the default, so Return resolves false', async () => {
    render(Alert);
    const answer = confirmAlert({ title: 'Delete this session?', confirmLabel: 'Delete', destructive: true });
    const cancel = await screen.findByRole('button', { name: 'Cancel' });
    await waitFor(() => expect(document.activeElement).toBe(cancel));
    await fireEvent.click(document.activeElement!);
    expect(await answer).toBe(false);
  });

  test('the confirm button resolves true', async () => {
    render(Alert);
    const answer = confirmAlert({ title: 'Reset permissions?', confirmLabel: 'Reset' });
    await fireEvent.click(await screen.findByRole('button', { name: 'Reset' }));
    expect(await answer).toBe(true);
  });
});

describe('toast', () => {
  test('four toasts leave three visible', async () => {
    render(Toast);
    for (let i = 1; i <= 4; i++) toast.show({ message: `Message ${i}` });
    await waitFor(() => expect(screen.getAllByRole('status')).toHaveLength(3));
  });

  test('clicking Undo runs the action', async () => {
    const run = vi.fn();
    render(Toast);
    toast.show({ message: 'Mark deleted', action: { label: 'Undo', run } });
    await fireEvent.click(await screen.findByRole('button', { name: 'Undo' }));
    expect(run).toHaveBeenCalledOnce();
  });

  test('a toast leaves after its duration and the returned function dismisses early', async () => {
    vi.useFakeTimers();
    render(Toast);
    toast.show({ message: 'Copied', durationMs: 1000 });
    const dismiss = toast.show({ message: 'Saved' });
    await vi.advanceTimersByTimeAsync(1000);
    expect(toasts.map((t) => t.message)).toEqual(['Saved']);
    dismiss();
    expect(toasts).toHaveLength(0);
  });
});

describe('Row', () => {
  test('an info row uses the info row height', () => {
    const { container } = render(Row, { kind: 'info', label: 'Storage' });
    expect((container.firstElementChild as HTMLElement).style.minHeight).toBe('var(--st-row-info)');
  });
});

describe('halo tokens', () => {
  test('only Halo.svelte and tokens.css use --st-halo-', () => {
    const root = resolve(__dirname, '..');
    const users: string[] = [];
    const walk = (dir: string) => {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const path = join(dir, entry.name);
        if (entry.isDirectory()) walk(path);
        else if (/\.(svelte|ts|css)$/.test(entry.name) && !entry.name.endsWith('.test.ts') && readFileSync(path, 'utf8').includes('--st-halo-'))
          users.push(relative(root, path));
      }
    };
    walk(root);
    expect(users.sort()).toEqual(['containers/Halo.svelte', 'tokens/tokens.css']);
  });
});
