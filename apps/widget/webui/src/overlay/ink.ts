/** How long after the draw key is released the ink stays, then how long it takes to fade. */
export const HOLD_MS = 900;
export const FADE_MS = 250;

const WIDTHS: Record<string, number> = { thin: 2, regular: 4, bold: 7 };

/** Stroke width in points for the Appearance → Ink thickness setting. */
export const strokeWidth = (thickness: string | undefined) => WIDTHS[thickness ?? 'regular'] ?? WIDTHS.regular;

/** The CSS colour for the Appearance → Ink colour setting (a `--st-ink-*` token). */
export const inkColor = (name: string | undefined) => `var(--st-ink-${name ?? 'orange'}, var(--st-ink-orange))`;

/** The midpoint between two points: strokes are drawn as quadratic curves through midpoints, so they come out smooth. */
export const mid = (a: { x: number; y: number }, b: { x: number; y: number }) => ({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 });

export type Bounds = { x: number; y: number; width: number; height: number };

/** `bounds` grown to include `point`. */
export function grow(bounds: Bounds | null, point: { x: number; y: number }): Bounds {
  if (!bounds) return { x: point.x, y: point.y, width: 0, height: 0 };
  const x = Math.min(bounds.x, point.x);
  const y = Math.min(bounds.y, point.y);
  return {
    x,
    y,
    width: Math.max(bounds.x + bounds.width, point.x) - x,
    height: Math.max(bounds.y + bounds.height, point.y) - y,
  };
}
