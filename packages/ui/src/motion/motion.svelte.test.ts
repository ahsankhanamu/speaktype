import { flushSync } from 'svelte';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { duration, easeOut, pop, reducedMotion, sheet, step } from './index';

const node = document.createElement('div');

describe('transitions', () => {
  test('step with reduced:true has no transform and a 120 ms duration', () => {
    const t = step(node, { direction: 1, reduced: true });
    expect(t.duration).toBe(120);
    expect(t.css!(0.5, 0.5)).not.toContain('transform');
  });

  test('step slides 24px in its direction', () => {
    expect(step(node, { direction: -1, reduced: false }).css!(0, 1)).toContain('translateX(-24px)');
  });

  test('sheet lasts 200 ms and starts at scale(0.96)', () => {
    const t = sheet(node, { reduced: false });
    expect(t.duration).toBe(200);
    expect(t.css!(0, 1)).toContain('scale(0.96)');
  });

  test('pop slides 4px when placed below and only fades when aligned', () => {
    expect(pop(node, { reduced: false, placement: 'below' }).css!(0, 1)).toContain('translateY(4px)');
    expect(pop(node, { reduced: false, placement: 'aligned' }).css!(0, 1)).not.toContain('transform');
  });

  test('easeOut follows cubic-bezier(.2,0,0,1)', () => {
    expect(easeOut(0)).toBe(0);
    expect(easeOut(1)).toBe(1);
    expect(easeOut(0.5)).toBeGreaterThan(0.8);
  });

  test('durations', () => expect(duration).toEqual({ fast: 120, base: 160, slow: 200, ink: 250 }));
});

describe('reducedMotion', () => {
  afterEach(() => vi.unstubAllGlobals());

  test('an omitted reduced option follows the media query', () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true }));
    expect(sheet(node).css!(0, 1)).not.toContain('transform');
  });

  test('attaches one listener for many readers and removes it after the last cleanup', async () => {
    let live = 0;
    let added = 0;
    vi.stubGlobal('matchMedia', () => ({
      matches: false,
      addEventListener: () => (live++, added++),
      removeEventListener: () => live--,
    }));
    const stops = Array.from({ length: 5 }, () =>
      $effect.root(() => {
        $effect(() => {
          reducedMotion.current;
        });
      }),
    );
    flushSync();
    expect(added).toBe(1);
    expect(live).toBe(1);
    stops.forEach((stop) => stop());
    await Promise.resolve();
    expect(live).toBe(0);
  });
});
