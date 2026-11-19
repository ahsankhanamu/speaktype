import { describe, expect, it } from 'vitest';
import { litBars } from './meter';

describe('litBars', () => {
  it('stays dark for silence and room tone', () => {
    expect(litBars(0, 5)).toBe(0);
    expect(litBars(0.001, 5)).toBe(0);
  });

  it('lights about half the bars for normal speech on a laptop mic', () => {
    expect(litBars(0.01, 5)).toBe(2);
    expect(litBars(0.03, 5)).toBe(3);
  });

  it('fills the meter for loud, close speech and never overflows', () => {
    expect(litBars(0.2, 5)).toBe(5);
    expect(litBars(1, 5)).toBe(5);
  });
});
