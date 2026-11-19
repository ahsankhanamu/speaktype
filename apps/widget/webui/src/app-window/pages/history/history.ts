import type { HistoryEntry, SessionSummary } from '../../../lib/ipc';
import type { HistoryEntryV2 } from '../../pending';

export interface Dictation {
  kind: 'dictation';
  key: string;
  /** Position in the history file, which the delete and reprocess commands take. */
  index: number;
  at: Date;
  entry: HistoryEntry & Pick<HistoryEntryV2, 'outcome'>;
}

export interface SessionRow {
  kind: 'session';
  key: string;
  at: Date;
  session: SessionSummary;
}

export type Entry = Dictation | SessionRow;
export type Header = { kind: 'header'; key: string; label: string };
export type Row = Header | Entry;
export type Filter = 'all' | 'sessions' | 'dictations';

const DAY_MS = 86_400_000;
const TITLE_CHARS = 48;

const startOfDay = (date: Date) => new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();

export function dayLabel(date: Date, now: Date): string {
  const days = Math.round((startOfDay(now) - startOfDay(date)) / DAY_MS);
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days < 7) return date.toLocaleDateString('en-GB', { weekday: 'long' });
  return date.toLocaleDateString('en-GB', { day: 'numeric', month: 'long' });
}

const OUTCOMES = { pasted: 'pasted', copied: 'copied', pasteFailed: 'paste failed' } as const;

export function outcomeLabel(outcome: HistoryEntryV2['outcome']): string {
  return outcome ? OUTCOMES[outcome] : '';
}

export function titleOf(text: string): string {
  const clean = text.trim();
  if (!clean) return 'Empty dictation';
  if (clean.length <= TITLE_CHARS) return clean;
  const cut = clean.slice(0, TITLE_CHARS);
  const space = cut.lastIndexOf(' ');
  return `${cut.slice(0, space > 0 ? space : TITLE_CHARS).trimEnd()}…`;
}

const haystack = (item: Entry) => (item.kind === 'dictation' ? item.entry.text : `${item.session.name} ${item.session.snippet ?? ''}`);

/** Entries grouped by day, newest first, with a header row before each day. */
export function buildRows(items: Entry[], filter: Filter, query: string, now: Date): Row[] {
  const needle = query.trim().toLowerCase();
  const wanted = items
    .filter((item) => filter === 'all' || item.kind === (filter === 'sessions' ? 'session' : 'dictation'))
    .filter((item) => !needle || haystack(item).toLowerCase().includes(needle))
    .sort((a, b) => b.at.getTime() - a.at.getTime());

  const rows: Row[] = [];
  let lastDay = -1;
  for (const item of wanted) {
    const day = startOfDay(item.at);
    if (day !== lastDay) rows.push({ kind: 'header', key: `h${day}`, label: dayLabel(item.at, now) });
    lastDay = day;
    rows.push(item);
  }
  return rows;
}

export const spoken = (seconds: number) => (seconds < 60 ? `${Math.max(1, Math.round(seconds))} s` : `${Math.round(seconds / 60)} min`);

const timeOf = (date: Date) => date.toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' });

const parts = (item: Entry): string[] => {
  if (item.kind === 'dictation') {
    const { duration_secs, outcome } = item.entry;
    return [duration_secs ? spoken(duration_secs) : '', outcomeLabel(outcome)];
  }
  const { startedAtMs, endedAtMs, markCount } = item.session;
  return [endedAtMs ? spoken((endedAtMs - startedAtMs) / 1000) : '', `${markCount} ${markCount === 1 ? 'mark' : 'marks'}`];
};

/** "10:52 · 9 s · pasted" */
export const metaOf = (item: Entry) => [timeOf(item.at), ...parts(item)].filter(Boolean).join(' · ');

/** "Today at 10:52 · 9 s · pasted" */
export const detailMetaOf = (item: Entry, now: Date) =>
  [`${dayLabel(item.at, now)} at ${timeOf(item.at)}`, ...parts(item)].filter(Boolean).join(' · ');
