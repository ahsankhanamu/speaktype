const MARGIN = 8;
const FOCUS_RING = 2;

export function alignedOffset(
  triggerLabelRect: DOMRect,
  itemLabelOffsetTop: number,
  contentHeight: number,
  viewportHeight: number,
  margin = MARGIN,
): number {
  return Math.max(margin, Math.min(triggerLabelRect.top - itemLabelOffsetTop, viewportHeight - contentHeight - margin));
}

const labelRect = (el: Element) => (el.querySelector('[data-st-label]') ?? el).getBoundingClientRect();

/**
 * Places a fixed-position popup so its selected item's label sits over the trigger's label (macOS pop-up buttons).
 * The popup appears under the pointer, so the release of the click that opened it must not pick the item below.
 */
export function alignTo(node: HTMLElement, trigger: HTMLElement | null) {
  let origin: { x: number; y: number } | null = null;
  let moved = false;
  const track = (e: PointerEvent) => {
    origin ??= { x: e.clientX, y: e.clientY };
    moved ||= Math.hypot(e.clientX - origin.x, e.clientY - origin.y) > 4;
  };
  const guard = (e: PointerEvent) => {
    if (!moved) e.stopPropagation();
    moved = true;
  };
  node.addEventListener('pointermove', track);
  node.addEventListener('pointerup', guard, true);
  place(node, trigger);
  return {
    destroy() {
      node.removeEventListener('pointermove', track);
      node.removeEventListener('pointerup', guard, true);
    },
  };
}

function place(node: HTMLElement, trigger: HTMLElement | null) {
  if (!trigger) return;
  const item = node.querySelector('[data-selected], [data-state="checked"]') ?? node.querySelector('[role="option"], [role="menuitemradio"]');
  const box = node.getBoundingClientRect();
  const itemLabel = item ? labelRect(item) : box;
  const triggerLabel = labelRect(trigger);
  const left = triggerLabel.left - (itemLabel.left - box.left);
  const width = Math.max(box.width, trigger.getBoundingClientRect().right + FOCUS_RING - left);
  node.style.minWidth = `${width}px`;
  node.style.top = `${alignedOffset(triggerLabel, itemLabel.top - box.top, box.height, innerHeight)}px`;
  node.style.left = `${Math.max(MARGIN, Math.min(left, innerWidth - width - MARGIN))}px`;
  node.style.visibility = 'visible';
}
