import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface Fit {
  width: number;
  height: number;
  /** Distance from the window's left edge to the pill's centre, which stays where it is on screen. */
  pivot: number;
}

/** Room around the pill for its halo; the window is never larger than this at rest. */
const MARGIN = 24;
const TOOLTIP = { width: 150, height: 124 };
const MENU = { width: 460, height: 560 };

/**
 * The window size for a pill (or pill and card) of the given size. A transparent window still takes
 * clicks, so it grows only while the tooltip or a menu needs room, and returns to the rest size after.
 */
export function fitFor(
  content: { width: number; height: number },
  { hovering, menuOpen }: { hovering: boolean; menuOpen: boolean },
): { width: number; height: number } {
  const rest = { width: content.width + 2 * MARGIN, height: content.height + 2 * MARGIN };
  const room = menuOpen ? MENU : hovering ? TOOLTIP : { width: 0, height: 0 };
  return { width: Math.max(rest.width, room.width), height: Math.max(rest.height, room.height) };
}

let current: Fit | undefined;
let latest: Fit | undefined;
let queue: Promise<void> = Promise.resolve();

async function apply(next: Fit) {
  if (next !== latest) return;
  const window = getCurrentWindow();
  const scale = await window.scaleFactor();
  if (!current) {
    const size = await window.innerSize();
    const width = size.width / scale;
    current = { width, height: size.height / scale, pivot: width / 2 };
  }
  if (next.width === current.width && next.height === current.height && next.pivot === current.pivot) return;
  const position = await window.outerPosition();
  await window.setPosition(
    new PhysicalPosition(
      Math.round(position.x + (current.pivot - next.pivot) * scale),
      Math.round(position.y + ((current.height - next.height) / 2) * scale),
    ),
  );
  await window.setSize(new LogicalSize(next.width, next.height));
  current = next;
}

/** Resizes the widget window to its content, growing and shrinking around the pill's centre. */
export function fitWindow(width: number, height: number, pivot: number): Promise<void> {
  const next = { width, height, pivot };
  latest = next;
  queue = queue.then(() => apply(next)).catch((error) => console.error('[widget] resize failed', error));
  return queue;
}
