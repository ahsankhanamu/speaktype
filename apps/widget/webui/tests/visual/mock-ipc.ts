type Fixture = Record<string, unknown>;
type Handler = (event: { event: string; id: number; payload: unknown }) => void;

declare global {
  interface Window {
    __TAURI_INTERNALS__: unknown;
    __TAURI_EVENT_PLUGIN_INTERNALS__: unknown;
    __mock: { emit(event: string, payload: unknown): Promise<void>; calls: { cmd: string; args: unknown }[] };
  }
}

type Box = { x: number; y: number; width: number; height: number };
type LayoutArgs = { size: { width: number; height: number }; current: Box | null };

/** The page plays the widget window: the pill sits in its middle, as it does away from the screen edges. */
function layout({ size, current }: LayoutArgs) {
  const to = {
    x: Math.round((innerWidth - size.width) / 2),
    y: Math.round((innerHeight - size.height) / 2),
    ...size,
  };
  return { from: current ?? to, to };
}

/** Stands in for the Tauri runtime: commands answer from the fixture, events fire from `emit`. */
export function installMockIpc(fixture: Fixture) {
  const callbacks = new Map<number, Handler>();
  const listeners = new Map<string, Map<number, number>>();
  let next = 1;
  const calls: { cmd: string; args: unknown }[] = [];

  window.__TAURI_INTERNALS__ = {
    transformCallback(callback: Handler) {
      const id = next++;
      callbacks.set(id, callback);
      return id;
    },
    unregisterCallback: (id: number) => callbacks.delete(id),
    async invoke(cmd: string, args: { event?: string; eventId?: number; handler?: number }) {
      if (cmd === 'plugin:event|listen' && args.event && args.handler) {
        const eventId = next++;
        const forEvent = listeners.get(args.event) ?? new Map<number, number>();
        forEvent.set(eventId, args.handler);
        listeners.set(args.event, forEvent);
        return eventId;
      }
      if (cmd === 'plugin:event|unlisten' && args.event && args.eventId) {
        listeners.get(args.event)?.delete(args.eventId);
        return null;
      }
      calls.push({ cmd, args });
      if (cmd === 'layout_widget') return layout(args as unknown as LayoutArgs);
      return fixture[cmd] ?? null;
    },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };

  window.__mock = {
    calls,
    async emit(event, payload) {
      while (!listeners.get(event)?.size) await new Promise((resolve) => requestAnimationFrame(resolve));
      for (const [eventId, handler] of listeners.get(event) ?? []) callbacks.get(handler)?.({ event, id: eventId, payload });
    },
  };
}
