import { describe, expect, it } from 'vitest';
import { grow, strokeWidth } from './ink';

describe('ink', () => {
  it('grows the mark bounds to cover every point', () => {
    let b = grow(null, { x: 10, y: 20 });
    b = grow(b, { x: 40, y: 5 });
    b = grow(b, { x: 25, y: 60 });
    expect(b).toEqual({ x: 10, y: 5, width: 30, height: 55 });
  });

  it('maps the thickness setting to a stroke width, regular by default', () => {
    expect(strokeWidth('thin')).toBeLessThan(strokeWidth('regular'));
    expect(strokeWidth(undefined)).toBe(strokeWidth('regular'));
  });
});
