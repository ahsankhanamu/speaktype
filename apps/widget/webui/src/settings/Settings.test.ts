import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, unmount } from 'svelte';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => Promise.resolve(() => {}),
  emit: (...args: unknown[]) => Promise.resolve(),
}));

describe('Settings runtime mount (jsdom)', () => {
  beforeEach(() => {
    invoke.mockReset().mockRejectedValue(new Error('Tauri not available in jsdom'));
  });

  it('mounts the settings shell and renders all tabs', async () => {
    const { default: Settings } = await import('./Settings.svelte');
    const host = document.createElement('div');
    document.body.appendChild(host);
    const app = mount(Settings, { target: host });

    // Wait for async load attempts to drain.
    await new Promise((r) => setTimeout(r, 50));

    const tabs = Array.from(host.querySelectorAll('.tab'));
    expect(tabs.length).toBe(6);
    expect(host.querySelector('.pane-title')?.textContent?.trim()).toBe('Hotkey');
    expect(host.querySelector('#save-btn')).toBeTruthy();
    expect(host.querySelector('#api-url')).toBeTruthy();
    expect(host.querySelector('#input-device-select')).toBeTruthy();

    unmount(app);
    host.remove();
  });

  it('switches tabs on sidebar click', async () => {
    const { default: Settings } = await import('./Settings.svelte');
    const host = document.createElement('div');
    document.body.appendChild(host);
    const app = mount(Settings, { target: host });

    await new Promise((r) => setTimeout(r, 50));

    const tabButtons = Array.from(host.querySelectorAll<HTMLButtonElement>('.tab'));
    const serverTab = tabButtons.find((b) => b.dataset.tab === 'server');
    expect(serverTab).toBeTruthy();
    // click Server tab
    serverTab!.click();
    await new Promise((r) => setTimeout(r, 20));

    expect((host.querySelector('.pane-title') as HTMLElement).textContent?.trim()).toBe('Server');

    unmount(app);
    host.remove();
  });

  it('toggles the input-device info popover', async () => {
    const { default: Settings } = await import('./Settings.svelte');
    const host = document.createElement('div');
    document.body.appendChild(host);
    const app = mount(Settings, { target: host });

    await new Promise((r) => setTimeout(r, 50));

    const info = host.querySelector<HTMLButtonElement>('.info-icon');
    expect(info).toBeTruthy();
    expect(info!.getAttribute('aria-expanded')).toBe('false');

    info!.click();
    await new Promise((r) => setTimeout(r, 20));
    expect(info!.getAttribute('aria-expanded')).toBe('true');
    const popover = host.querySelector('.info-popover');
    expect(popover).toBeTruthy();
    expect(popover!.textContent).toContain('Bluetooth');

    unmount(app);
    host.remove();
  });
});