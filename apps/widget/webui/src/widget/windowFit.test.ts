import { describe, expect, it } from 'vitest';
import { fitFor } from './windowFit';

const idle = { width: 44, height: 44 };
const recording = { width: 172, height: 40 };
const rest = { hovering: false, menuOpen: false };

describe('fitFor', () => {
  it('hugs an idle pill with 24 px on each side', () => {
    expect(fitFor(idle, rest)).toEqual({ width: 92, height: 92 });
  });

  it('hugs a text or recording pill by its own width', () => {
    expect(fitFor(recording, rest)).toEqual({ width: 220, height: 88 });
  });

  it('grows for the tooltip while hovering, then returns to rest', () => {
    const hover = fitFor(idle, { ...rest, hovering: true });
    expect(hover.width).toBeGreaterThanOrEqual(130);
    expect(hover.height).toBeGreaterThanOrEqual(120);
    expect(fitFor(idle, rest)).toEqual({ width: 92, height: 92 });
  });

  it('never shrinks a wide pill below its content while hovering', () => {
    expect(fitFor(recording, { ...rest, hovering: true }).width).toBe(220);
  });

  it('grows for a menu and returns to the rest size when it closes', () => {
    const open = fitFor(idle, { ...rest, menuOpen: true });
    expect(open.width).toBeGreaterThan(400);
    expect(open.height).toBeGreaterThan(500);
    expect(fitFor(idle, rest)).toEqual({ width: 92, height: 92 });
  });
});
