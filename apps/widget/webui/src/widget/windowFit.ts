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

const TOOLTIP = { width: 150, height: 124 };
const MENU = { width: 460, height: 560 };
const NONE = { width: 0, height: 0 };

/** How far an opened pill keeps from the screen's edges; the idle bubble may sit right at them. */
export const EDGE_INSET = 8;

/**
 * The least window the pill needs around it. A transparent window still takes clicks, so it grows
 * only while the tooltip or a menu needs room, and returns to hugging the pill after.
 */
export function roomFor({ hovering, menuOpen }: { hovering: boolean; menuOpen: boolean }): Extent {
  return menuOpen ? MENU : hovering ? TOOLTIP : NONE;
}

export interface Target {
  size: Extent;
  room: Extent;
  inset: number;
}

/** Draws the pill at `box`; with `animate`, resolves once the morph has landed. */
export type Show = (box: Box, animate: boolean) => Promise<void>;

const same = (a: Target, b: Target) =>
  a.size.width === b.size.width &&
  a.size.height === b.size.height &&
  a.room.width === b.room.width &&
  a.room.height === b.room.height &&
  a.inset === b.inset;

// Rust's f64 crosses as `number | null` (NaN); a layout never produces one.
const box = (r: ScreenRect): Box => ({ x: r.x ?? 0, y: r.y ?? 0, width: r.width ?? 0, height: r.height ?? 0 });

/**
 * Lays the widget out for each target in turn, the latest winning. The window first grows to hold
 * the pill where it is drawn now and where it is going, the pill morphs between the two, and then
 * the window hugs the pill again. A new target cuts a morph short and carries on from wherever the
 * pill has got to.
 */
export function createLayout(show: Show, current: () => Box | null) {
  let latest: Target | undefined;
  let interrupt = () => {};
  let queue: Promise<void> = Promise.resolve();

  async function run(target: Target) {
    if (target !== latest) return;
    const { size, room, inset } = target;
    const superseded = new Promise<void>((resolve) => (interrupt = resolve));
    const grown = await commands.layoutWidget(size, room, inset, current());
    await show(box(grown.from), false);
    await Promise.race([show(box(grown.to), true), superseded]);
    if (target !== latest) return;
    const hugged = await commands.layoutWidget(size, room, inset, null);
    await show(box(hugged.to), false);
  }

  return function layout(target: Target): Promise<void> {
    if (latest && same(target, latest)) return queue;
    latest = target;
    interrupt();
    queue = queue.then(() => run(target)).catch((error) => console.error('[widget] layout failed', error));
    return queue;
  };
}
