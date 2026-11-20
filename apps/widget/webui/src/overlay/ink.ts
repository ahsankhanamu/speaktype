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
