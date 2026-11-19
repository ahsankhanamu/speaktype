import type { EventBus } from './bus';

type Callback = (payload: unknown) => void;

export class FakeBus implements EventBus {
  private listeners = new Map<string, Set<Callback>>();
  private pending: Array<{
    name: string;
    callback: Callback;
    resolve: (unlisten: () => void) => void;
  }> = [];

  liveListeners = 0;
  deferred = false;

  listen<T>(name: string, callback: (payload: T) => void): Promise<() => void> {
    const cb = callback as Callback;
    if (this.deferred) {
      return new Promise((resolve) => this.pending.push({ name, callback: cb, resolve }));
    }
    return Promise.resolve(this.add(name, cb));
  }

  emit(name: string, payload: unknown): void {
    for (const callback of [...(this.listeners.get(name) ?? [])]) callback(payload);
  }

  resolveNext(): void {
    const pending = this.pending.shift();
    if (!pending) throw new Error('No deferred listener');
    pending.resolve(this.add(pending.name, pending.callback));
  }

  get pendingListeners(): number {
    return this.pending.length;
  }

  private add(name: string, callback: Callback): () => void {
    const callbacks = this.listeners.get(name) ?? new Set<Callback>();
    callbacks.add(callback);
    this.listeners.set(name, callbacks);
    this.liveListeners += 1;
    let active = true;
    return () => {
      if (!active) return;
      active = false;
      callbacks.delete(callback);
      this.liveListeners -= 1;
    };
  }
}
