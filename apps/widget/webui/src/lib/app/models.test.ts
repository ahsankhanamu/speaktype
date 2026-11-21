import { describe, expect, it } from 'vitest';
import { modelState, plainModel, type ModelInfo, type ModelProgress } from './models';

const model = (over: Partial<ModelInfo> = {}): ModelInfo => ({ id: 'small', size_bytes: 0, downloaded: false, ...over });
const state = (m: ModelInfo, p?: ModelProgress, active = 'base') => modelState(m, p, active);

describe('plainModel', () => {
  it.each([
    ['tiny', 'Tiny', 'more', '~75 MB'],
    ['base', 'Fast', 'fast', '~150 MB'],
    ['small', 'Balanced', 'balanced', '~500 MB'],
    ['medium', 'Medium', 'more', '~1.5 GB'],
    ['large-v3-turbo', 'Accurate', 'accurate', '~1.5 GB'],
    ['large-v3', 'Most accurate', 'more', '~3 GB'],
  ])('%s is %s', (id, name, tier, size) => {
    expect(plainModel(id)).toMatchObject({ name, tier, size });
  });

  it('maps English-only variants to the same names', () => {
    for (const id of ['tiny', 'base', 'small', 'medium']) {
      expect(plainModel(`${id}.en`)).toEqual(plainModel(id));
    }
  });

  it('recommends Balanced only', () => {
    expect(plainModel('small').recommended).toBe(true);
    expect(['tiny', 'base', 'medium', 'large-v3-turbo', 'large-v3'].some((id) => plainModel(id).recommended)).toBe(false);
  });

  it('keeps the board trade-off lines', () => {
    expect(plainModel('small').detail).toBe('Small · best for most people');
    expect(plainModel('large-v3').detail).toBe('Large v3 · every language · needs 16 GB of memory');
  });
});

describe('modelState', () => {
  it('marks the downloaded active model In use', () => {
    expect(state(model({ id: 'base', downloaded: true }))).toEqual({ label: 'In use', tone: 'success', action: null });
  });

  it('offers Use for other downloaded models', () => {
    expect(state(model({ downloaded: true }))).toEqual({ label: 'Downloaded', tone: 'neutral', action: 'use' });
  });

  it('offers Download with the size', () => {
    expect(state(model())).toEqual({ label: 'Download · ~500 MB', tone: 'neutral', action: 'download' });
  });

  it('shows the percentage while downloading, with pause', () => {
    const s = state(model(), { phase: 'downloading', percent: 64 });
    expect(s).toMatchObject({ label: 'Downloading · 64%', tone: 'accent', action: 'pause' });
  });

  it('names the model a queued download waits for', () => {
    const s = state(model(), { phase: 'waiting', after: 'Accurate' });
    expect(s).toEqual({ label: 'Waiting · starts after Accurate', tone: 'neutral', action: 'cancel' });
  });

  it('shows paused progress with Resume', () => {
    expect(state(model(), { phase: 'paused', percent: 41 })).toEqual({ label: 'Paused at 41%', tone: 'neutral', action: 'resume' });
  });

  it('treats a leftover partial file as paused', () => {
    const s = state(model({ partial: true, partial_size: 205, expected_size: 500 }));
    expect(s).toEqual({ label: 'Paused at 41%', tone: 'neutral', action: 'resume' });
  });

  it('passes the retrying message through as a warning', () => {
    const message = 'Connection lost — retrying in 8s… (2/5)';
    expect(state(model(), { phase: 'retrying', message })).toEqual({ label: message, tone: 'warning', action: null });
  });

  it('shows the failure with Try again', () => {
    const s = state(model(), { phase: 'error', message: 'Not enough disk space' });
    expect(s).toEqual({ label: 'Not enough disk space', tone: 'danger', action: 'retry' });
  });

  it.each(['preparing', 'stopping', 'starting', 'loading'])('says Getting ready… while %s', (phase) => {
    expect(state(model({ downloaded: true }), { phase })).toEqual({ label: 'Getting ready…', tone: 'accent', action: null });
  });

  it('survives an unknown model with no size', () => {
    expect(state(model({ id: 'mystery' })).label).toBe('Download');
  });

  describe('ETA', () => {
    const bytes = { phase: 'downloading', percent: 38, bytes_downloaded: 580 * 1024 ** 2, total_bytes: 1.5 * 1024 ** 3 } as const;

    it('is hidden before 4 s of samples', () => {
      expect(state(model(), { ...bytes, eta_secs: 120, elapsed_ms: 3999 }).detail).toBe('580 MB of 1.5 GB');
    });

    it('shows minutes after 4 s of samples', () => {
      expect(state(model(), { ...bytes, eta_secs: 120, elapsed_ms: 4000 }).detail).toBe('580 MB of 1.5 GB · 2 min');
    });

    it('shows seconds under a minute', () => {
      expect(state(model(), { ...bytes, eta_secs: 40, elapsed_ms: 9000 }).detail).toBe('580 MB of 1.5 GB · 40 s');
    });
  });
});
