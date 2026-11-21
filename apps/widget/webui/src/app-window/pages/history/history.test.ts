import { describe, expect, it } from 'vitest';
import { buildRows, dayLabel, detailMetaOf, metaOf, outcomeLabel, spoken, titleOf, type Dictation, type SessionRow } from './history';

const NOW = new Date('2026-11-19T12:00:00');

const entry = (index: number, timestamp: string, text = 'Hello world', extra = {}): Dictation => ({
  kind: 'dictation',
  key: `d${index}`,
  index,
  at: new Date(timestamp),
  entry: { text, timestamp, ...extra },
});
const session = (id: number, timestamp: string, name = 'Checkout bug', snippet: string | null = null): SessionRow => ({
  kind: 'session',
  key: `s${id}`,
  at: new Date(timestamp),
  session: { id, name, startedAtMs: new Date(timestamp).getTime(), endedAtMs: null, markCount: 2, clipCount: 0, snippet },
});

describe('dayLabel', () => {
  it('names today, yesterday, this week and older days', () => {
    expect(dayLabel(new Date('2026-11-19T08:00:00'), NOW)).toBe('Today');
    expect(dayLabel(new Date('2026-11-18T23:00:00'), NOW)).toBe('Yesterday');
    expect(dayLabel(new Date('2026-11-16T10:00:00'), NOW)).toBe('Monday');
    expect(dayLabel(new Date('2026-11-02T10:00:00'), NOW)).toBe('2 November');
  });
});

describe('buildRows', () => {
  const items = [
    entry(0, '2026-11-19T10:52:00', 'Pushed the checkout fix'),
    session(1, '2026-11-19T10:41:00', 'Checkout overlaps total'),
    entry(1, '2026-11-18T16:05:00', 'Review copy'),
  ];

  it('groups newest first under day headers', () => {
    const rows = buildRows(items, 'all', '', NOW);
    expect(rows.map((row) => (row.kind === 'header' ? row.label : row.key))).toEqual(['Today', 'd0', 's1', 'Yesterday', 'd1']);
  });

  it('filters sessions and dictations', () => {
    expect(buildRows(items, 'sessions', '', NOW).filter((row) => row.kind !== 'header')).toHaveLength(1);
    expect(buildRows(items, 'dictations', '', NOW).filter((row) => row.kind !== 'header')).toHaveLength(2);
  });

  it('searches text and session names, case-insensitively', () => {
    const keys = (query: string) => buildRows(items, 'all', query, NOW).flatMap((row) => (row.kind === 'header' ? [] : [row.key]));
    expect(keys('CHECKOUT')).toEqual(['d0', 's1']);
    expect(keys('nothing')).toEqual([]);
  });
});

describe('labels', () => {
  it('words each outcome', () => {
    expect(outcomeLabel('pasted')).toBe('pasted');
    expect(outcomeLabel('copied')).toBe('copied');
    expect(outcomeLabel('pasteFailed')).toBe('paste failed');
    expect(outcomeLabel(null)).toBe('');
    expect(outcomeLabel(undefined)).toBe('');
  });

  it('titles an entry with its first words', () => {
    expect(titleOf('Pushed the checkout fix, can you retest this on staging when you get a moment please')).toBe(
      'Pushed the checkout fix, can you retest this on…',
    );
    expect(titleOf('Short one')).toBe('Short one');
    expect(titleOf('')).toBe('Empty dictation');
  });
});

describe('metaOf', () => {
  it('reads like the board: time, length, outcome', () => {
    expect(metaOf(entry(0, '2026-11-19T10:52:00', 'x', { duration_secs: 9, outcome: 'pasted' }))).toBe('10:52 · 9 s · pasted');
    expect(metaOf(entry(0, '2026-11-19T10:52:00'))).toBe('10:52');
    expect(spoken(134)).toBe('2 min');
  });

  it('spells out the day in the detail header', () => {
    expect(detailMetaOf(entry(0, '2026-11-19T10:52:00', 'x', { duration_secs: 9, outcome: 'pasted' }), NOW)).toBe('Today at 10:52 · 9 s · pasted');
  });

  it("counts a session's marks", () => {
    expect(metaOf(session(1, '2026-11-19T16:05:00'))).toBe('16:05 · 2 marks');
  });
});
