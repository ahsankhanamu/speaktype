import type { TransitionConfig } from 'svelte/transition';
import { reducedMotion } from './reduced.svelte';

export { reducedMotion };

export const duration = { fast: 120, base: 160, slow: 200, ink: 250, morph: 320 } as const;

const bezier = (a: number, b: number, t: number) => 3 * a * (1 - t) ** 2 * t + 3 * b * (1 - t) * t ** 2 + t ** 3;

/** cubic-bezier(.2, 0, 0, 1) */
export function easeOut(x: number): number {
  if (x <= 0 || x >= 1) return Math.min(Math.max(x, 0), 1);
  let [lo, hi] = [0, 1];
  for (let i = 0; i < 24; i++) {
    const mid = (lo + hi) / 2;
    if (bezier(0.2, 0, mid) < x) lo = mid;
    else hi = mid;
  }
  return bezier(0, 1, (lo + hi) / 2);
}

type Options = { reduced?: boolean };

const isReduced = (p?: Options) => p?.reduced ?? reducedMotion.current;

const fade: TransitionConfig = { duration: duration.fast, easing: easeOut, css: (t) => `opacity:${t}` };

export function sheet(_node: Element, p?: Options): TransitionConfig {
  if (isReduced(p)) return fade;
  return { duration: duration.slow, easing: easeOut, css: (t) => `opacity:${t};transform:scale(${0.96 + 0.04 * t})` };
}

export function pop(_node: Element, p?: Options & { placement?: 'below' | 'aligned' }): TransitionConfig {
  if (p?.placement === 'aligned' || isReduced(p)) return fade;
  return { duration: duration.fast, easing: easeOut, css: (t, u) => `opacity:${t};transform:translateY(${4 * u}px)` };
}

export function step(_node: Element, p: Options & { direction: 1 | -1 }): TransitionConfig {
  if (isReduced(p)) return fade;
  return {
    duration: duration.slow,
    easing: easeOut,
    css: (t, u) => `opacity:${t};transform:translateX(${24 * p.direction * u}px)`,
  };
}
