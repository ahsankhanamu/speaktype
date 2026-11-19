import { emit, listen } from '@tauri-apps/api/event';

export interface EventBus {
  listen<T>(name: string, cb: (payload: T) => void): Promise<() => void>;
}

const tauriBus: EventBus = {
  listen: <T>(name: string, cb: (payload: T) => void) =>
    listen<T>(name, (event) => cb(event.payload)),
};

let currentBus = tauriBus;

export function setBus(bus: EventBus): void {
  currentBus = bus;
}

export function busListen<T>(name: string, cb: (payload: T) => void): Promise<() => void> {
  return currentBus.listen(name, cb);
}

export const emitEvent = <T>(name: string, payload?: T) => emit(name, payload);
