import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setBus } from './bus';
import LiveQueryHarness from './liveQuery.test.svelte';
import { FakeBus } from './testBus';

interface QueryHandle {
  readonly data: unknown;
  readonly error: unknown;
  readonly loading: boolean;
  refetch(): Promise<void>;
}

function mountQuery(
  queryKey: string,
  fetcher: () => Promise<unknown>,
  options: { invalidateOn?: string[]; pollMs?: number; refetchOnFocus?: boolean } = {},
) {
  let query!: QueryHandle;
  const app = mount(LiveQueryHarness, {
    target: document.body,
    props: {
      queryKey,
      fetcher,
      ...options,
      ready: (value: QueryHandle) => (query = value),
    },
  });
  flushSync();
  return { app, query: () => query };
}

describe('liveQuery', () => {
  let bus: FakeBus;

  beforeEach(() => {
    bus = new FakeBus();
    setBus(bus);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('refetches when an invalidating event arrives', async () => {
    let fetchCount = 0;
    const { app, query } = mountQuery('event-query', async () => {
      fetchCount += 1;
      return `result-${fetchCount}`;
    }, { invalidateOn: ['sessions-changed'] });

    await vi.waitFor(() => expect(query().data).toBe('result-1'));
    bus.emit('sessions-changed', null);
    await vi.waitFor(() => expect(query().data).toBe('result-2'));
    expect(fetchCount).toBe(2);
    await unmount(app);
  });

  it('refetches when remounted after every user had unmounted', async () => {
    let fetchCount = 0;
    const fetcher = async () => `result-${++fetchCount}`;
    const first = mountQuery('remount-query', fetcher, { invalidateOn: ['sessions-changed'] });
    await vi.waitFor(() => expect(first.query().data).toBe('result-1'));
    await unmount(first.app);
    bus.emit('sessions-changed', null);
    const second = mountQuery('remount-query', fetcher, { invalidateOn: ['sessions-changed'] });
    await vi.waitFor(() => expect(second.query().data).toBe('result-2'));
    await unmount(second.app);
  });

  it('a slow earlier response never overwrites a newer one', async () => {
    const resolvers: Array<(value: string) => void> = [];
    const { app, query } = mountQuery(
      'race-query',
      () => new Promise<string>((resolve) => resolvers.push(resolve)),
    );
    expect(resolvers).toHaveLength(1);

    const second = query().refetch();
    expect(resolvers).toHaveLength(2);
    resolvers[1]('newer');
    await second;
    expect(query().data).toBe('newer');

    resolvers[0]('older');
    await Promise.resolve();
    await Promise.resolve();
    expect(query().data).toBe('newer');
    await unmount(app);
  });

  it('two components with the same key share one fetch', async () => {
    let fetchCount = 0;
    const fetcher = async () => {
      fetchCount += 1;
      return 'shared';
    };
    const first = mountQuery('shared-query', fetcher);
    const second = mountQuery('shared-query', fetcher);

    await vi.waitFor(() => {
      expect(first.query().data).toBe('shared');
      expect(second.query().data).toBe('shared');
    });
    expect(fetchCount).toBe(1);
    await unmount(first.app);
    await unmount(second.app);
  });

  it('stops polling and unlistens when the last user unmounts', async () => {
    vi.useFakeTimers();
    let fetchCount = 0;
    const { app } = mountQuery('polling-query', async () => {
      fetchCount += 1;
      return fetchCount;
    }, { invalidateOn: ['sessions-changed'], pollMs: 100 });

    await vi.waitFor(() => expect(fetchCount).toBe(1));
    await vi.advanceTimersByTimeAsync(100);
    expect(fetchCount).toBe(2);
    expect(bus.liveListeners).toBe(1);

    await unmount(app);
    await vi.advanceTimersByTimeAsync(1000);
    bus.emit('sessions-changed', null);

    expect(fetchCount).toBe(2);
    expect(bus.liveListeners).toBe(0);
  });
});
