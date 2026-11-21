import { beforeEach, describe, expect, it } from 'vitest';
import { PAGES, currentPage, navigate, startRouter } from './router.svelte';

beforeEach(() => {
  localStorage.clear();
  location.hash = '';
});

describe('router', () => {
  it('has a sidebar item for every spec page', () => {
    expect(PAGES.map((page) => page.id)).toEqual([
      'history', 'general', 'keyboard', 'appearance', 'dictation', 'sessions', 'providers', 'permissions', 'storage', 'advanced',
    ]);
  });

  it('every page routes through the hash', () => {
    startRouter();
    for (const { id } of PAGES) {
      navigate(id);
      expect(location.hash).toBe(`#/${id}`);
      expect(currentPage.id).toBe(id);
    }
  });

  it('about is a route without a sidebar item', () => {
    startRouter();
    navigate('about');
    expect(currentPage.id).toBe('about');
  });

  it('opens the section the menu bar asked for as a route', () => {
    startRouter();
    location.hash = '#/history';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(currentPage.id).toBe('history');
  });

  it('follows a hash change', () => {
    startRouter();
    location.hash = '#/providers';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    expect(currentPage.id).toBe('providers');
  });

  it('remembers the last page', () => {
    startRouter();
    navigate('storage');
    location.hash = '';
    startRouter();
    expect(currentPage.id).toBe('storage');
  });

  it('opens on history when nothing is remembered', () => {
    startRouter();
    expect(currentPage.id).toBe('history');
  });

  it('still opens when storage throws', () => {
    const original = Storage.prototype.getItem;
    Storage.prototype.getItem = () => {
      throw new Error('blocked');
    };
    try {
      startRouter();
      expect(currentPage.id).toBe('history');
    } finally {
      Storage.prototype.getItem = original;
    }
  });
});
