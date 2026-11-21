import { flushSync, mount, unmount } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setBus } from './bus';
import { FakeBus } from './testBus';
import UseEventHarness from './useEvent.test.svelte';

describe('useEvent', () => {
  let bus: FakeBus;

  beforeEach(() => {
    bus = new FakeBus();
    setBus(bus);
  });

  it('releases every listener after 200 mount/unmount cycles', async () => {
    for (let i = 0; i < 200; i += 1) {
      const app = mount(UseEventHarness, {
        target: document.body,
        props: { name: 'session-state', callback: vi.fn() },
      });
      flushSync();
      await Promise.resolve();
      expect(bus.liveListeners).toBe(1);
      await unmount(app);
    }

    expect(bus.liveListeners).toBe(0);
  });

  it('unmount before listen resolves still unlistens', async () => {
    bus.deferred = true;
    const app = mount(UseEventHarness, {
      target: document.body,
      props: { name: 'session-state', callback: vi.fn() },
    });
    flushSync();
    expect(bus.pendingListeners).toBe(1);

    await unmount(app);
    bus.resolveNext();
    await Promise.resolve();
    await Promise.resolve();

    expect(bus.liveListeners).toBe(0);
  });
});
