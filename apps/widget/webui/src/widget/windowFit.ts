import type { ScreenRect } from '../lib/bindings';
import { commands } from '../lib/ipc';

export interface Extent {
  width: number;
  height: number;
}
/** A rect in the widget window's coordinates, in points. */
export interface Box extends Extent {
  x: number;
  y: number;
}

/** How far an opened pill keeps from the screen's edges; the idle bubble may sit right at them. */
export const EDGE_INSET = 8;

export interface Target {
  size: Extent;
  inset: number;
}

// Rust's f64 crosses as `number | null` (NaN); a layout never produces one.
export const toBox = (r: ScreenRect): Box => ({ x: r.x ?? 0, y: r.y ?? 0, width: r.width ?? 0, height: r.height ?? 0 });

/**
 * Asks where each pill goes, the latest target winning. The window is a fixed stage around the
 * bubble, so placing a pill never moves it: `show` only changes where the pill is drawn, and a CSS
 * transition carries the pill there from wherever it is.
 */
export function createLayout(show: (box: Box) => void) {
  let latest: Target | undefined;
  let queue: Promise<void> = Promise.resolve();

  async function run(target: Target) {
    if (target !== latest) return;
    show(toBox(await commands.layoutWidget(target.size, target.inset)));
  }

  return function layout(target: Target): Promise<void> {
    if (latest && latest.size.width === target.size.width && latest.size.height === target.size.height && latest.inset === target.inset) {
      return queue;
    }
    latest = target;
    queue = queue.then(() => run(target)).catch((error) => console.error('[widget] layout failed', error));
    return queue;
  };
}

/** Rects equal to the point, so an unchanged report is not sent again. */
export const sameRegions = (a: Box[], b: Box[]) =>
  a.length === b.length && a.every((r, i) => r.x === b[i].x && r.y === b[i].y && r.width === b[i].width && r.height === b[i].height);
