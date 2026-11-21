import { describe, expect, it } from 'vitest';
import { PAGES } from './router.svelte';
import { filter, index } from './search';

describe('settings search', () => {
  it('finds Keyboard > Draw (hold) for "draw"', () => {
    const hit = filter('draw').find((entry) => entry.label === 'Draw (hold)');
    expect(hit?.page).toBe('keyboard');
  });

  it('matches keywords, case-insensitively', () => {
    expect(filter('KEYCHAIN').map((entry) => entry.page)).toContain('providers');
  });

  it('returns nothing for an empty or unmatched query', () => {
    expect(filter('  ')).toEqual([]);
    expect(filter('zzzz')).toEqual([]);
  });

  it('only points at pages that exist', () => {
    const ids = new Set([...PAGES.map((page) => page.id), 'about']);
    for (const entry of index) expect(ids.has(entry.page)).toBe(true);
  });
});
