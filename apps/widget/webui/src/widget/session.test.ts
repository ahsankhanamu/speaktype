import { describe, expect, it } from 'vitest';
import { elapsed } from './session.svelte';

describe('elapsed', () => {
  it('reads as m:ss from the session start', () => {
    expect(elapsed(0, 0)).toBe('0:00');
    expect(elapsed(0, 9_400)).toBe('0:09');
    expect(elapsed(0, 134_000)).toBe('2:14');
  });

  it('never goes negative when clocks disagree', () => {
    expect(elapsed(5_000, 0)).toBe('0:00');
  });
});
