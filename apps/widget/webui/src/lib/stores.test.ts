import { describe, it, expect } from 'vitest';
import { computeDirty, defaultSnapshot } from './stores';

describe('computeDirty', () => {
  it('is not dirty when form matches snapshot', () => {
    const snapshot = defaultSnapshot();
    const current = { ...snapshot };
    expect(computeDirty(current, snapshot)).toBe(false);
  });

  it('is dirty when a field diverges', () => {
    const snapshot = defaultSnapshot();
    const current = { ...snapshot, language: 'en' };
    expect(computeDirty(current, snapshot)).toBe(true);
  });

  it('treats post_paste_keys default as empty string snapshot', () => {
    const snapshot = { ...defaultSnapshot(), post_paste_keys: 'enter' };
    const current = { ...snapshot, post_paste_keys: 'enter' };
    expect(computeDirty(current, snapshot)).toBe(false);

    // Snapshot value '' vs form 'enter' diverges
    const s2 = { ...defaultSnapshot(), post_paste_keys: '' };
    expect(computeDirty({ ...s2, post_paste_keys: 'enter' }, s2)).toBe(true);
  });

  it('ignores keys not part of the persisted field set', () => {
    const snapshot = defaultSnapshot();
    const current = { ...snapshot, some_other: 'x' as unknown };
    expect(computeDirty(current, snapshot)).toBe(false);
  });
});

describe('defaultSnapshot', () => {
  it('returns the legacy defaults', () => {
    const d = defaultSnapshot();
    expect(d.hotkey).toBe('Super+Control');
    expect(d.api_url).toBe('http://127.0.0.1:8002/inference');
    expect(d.model).toBe('medium');
    expect(d.language).toBe('auto');
    expect(d.paste_mode).toBe('active');
    expect(d.post_paste_keys).toBe('enter');
    expect(d.save_recordings).toBe(false);
    expect(d.hallucination_guard).toBe(true);
    expect(d.theme).toBe('auto');
    expect(d.input_device).toBe('');
  });
});