import { busListen } from './bus';

export function useEvent<T>(name: string, cb: (payload: T) => void): void {
  $effect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    busListen<T>(name, cb).then((off) => {
      if (disposed) off();
      else unlisten = off;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  });
}
