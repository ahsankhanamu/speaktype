import { busListen } from './bus';

interface QueryOptions {
  invalidateOn: string[];
  pollMs?: number;
  refetchOnFocus?: boolean;
}

interface QueryEntry<T> {
  key: string;
  fetcher: () => Promise<T>;
  opts: QueryOptions;
  state: { data: T | undefined; error: unknown; loading: boolean };
  refs: number;
  sequence: number;
  generation: number;
  active: boolean;
  unlistens: Array<() => void>;
  poll?: ReturnType<typeof setInterval>;
  eviction?: ReturnType<typeof setTimeout>;
  onFocus?: () => void;
}

const queries = new Map<string, QueryEntry<unknown>>();

async function refetch<T>(entry: QueryEntry<T>): Promise<void> {
  const sequence = ++entry.sequence;
  entry.state.loading = true;
  try {
    const data = await entry.fetcher();
    if (sequence !== entry.sequence) return;
    entry.state.data = data;
    entry.state.error = undefined;
  } catch (error) {
    if (sequence === entry.sequence) entry.state.error = error;
  } finally {
    if (sequence === entry.sequence) entry.state.loading = false;
  }
}

function retain<T>(entry: QueryEntry<T>): void {
  entry.refs += 1;
  if (entry.refs > 1) return;
  if (entry.eviction !== undefined) clearTimeout(entry.eviction);
  entry.eviction = undefined;
  entry.active = true;
  const generation = ++entry.generation;
  for (const name of entry.opts.invalidateOn) {
    busListen(name, () => void refetch(entry)).then((unlisten) => {
      if (!entry.active || generation !== entry.generation) unlisten();
      else entry.unlistens.push(unlisten);
    });
  }
  if (entry.opts.pollMs !== undefined) {
    entry.poll = setInterval(() => void refetch(entry), entry.opts.pollMs);
  }
  if (entry.opts.refetchOnFocus !== false) {
    entry.onFocus = () => void refetch(entry);
    window.addEventListener('focus', entry.onFocus);
  }
  void refetch(entry);
}

function release<T>(entry: QueryEntry<T>): void {
  entry.refs -= 1;
  if (entry.refs > 0) return;
  entry.active = false;
  entry.generation += 1;
  for (const unlisten of entry.unlistens) unlisten();
  entry.unlistens = [];
  if (entry.poll !== undefined) clearInterval(entry.poll);
  entry.poll = undefined;
  if (entry.onFocus) window.removeEventListener('focus', entry.onFocus);
  entry.onFocus = undefined;
  entry.eviction = setTimeout(() => {
    if (entry.refs === 0) queries.delete(entry.key);
  }, 30_000);
}

export function liveQuery<T>(
  key: string,
  fetcher: () => Promise<T>,
  opts: QueryOptions,
): { readonly data: T | undefined; readonly error: unknown; readonly loading: boolean; refetch(): Promise<void> } {
  let entry = queries.get(key) as QueryEntry<T> | undefined;
  if (!entry) {
    const state: { data: T | undefined; error: unknown; loading: boolean } = $state({
      data: undefined,
      error: undefined,
      loading: false,
    });
    entry = {
      key,
      fetcher,
      opts,
      state,
      refs: 0,
      sequence: 0,
      generation: 0,
      active: false,
      unlistens: [],
    };
    queries.set(key, entry as QueryEntry<unknown>);
  }
  $effect(() => {
    retain(entry);
    return () => release(entry);
  });
  return {
    get data() { return entry.state.data; },
    get error() { return entry.state.error; },
    get loading() { return entry.state.loading; },
    refetch: () => refetch(entry),
  };
}
