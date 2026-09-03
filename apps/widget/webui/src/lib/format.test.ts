import { describe, it, expect } from 'vitest';
import {
  formatHotkey,
  normalizeHotkey,
  isModifierChordHotkey,
  isAcceptedHotkey,
  formatDuration,
  formatByteSize,
  keyFromKeyboardEvent,
  detectPlatform,
} from './format';

describe('normalizeHotkey', () => {
  it('maps Command/CommandOrControl/Meta to Super', () => {
    expect(normalizeHotkey('Command+Shift+A')).toBe('Super+Shift+A');
    expect(normalizeHotkey('CmdOrCtrl+Alt+K')).toBe('Super+Alt+K');
    expect(normalizeHotkey('Meta+Option+P')).toBe('Super+Option+P');
  });
  it('leaves plain keys untouched', () => {
    expect(normalizeHotkey('F9')).toBe('F9');
    expect(normalizeHotkey('Super+Control')).toBe('Super+Control');
  });
});

describe('isModifierChordHotkey', () => {
  it('matches Super+Control only (2 parts)', () => {
    expect(isModifierChordHotkey(['Super', 'Control'])).toBe(true);
    expect(isModifierChordHotkey(['Control', 'Super'])).toBe(true);
    expect(isModifierChordHotkey(['Super', 'Control', 'A'])).toBe(false);
    expect(isModifierChordHotkey(['Super', 'Shift'])).toBe(false);
  });
});

describe('isAcceptedHotkey', () => {
  it('accepts a lone function key', () => {
    expect(isAcceptedHotkey(['F1'])).toBe(true);
    expect(isAcceptedHotkey(['F12'])).toBe(true);
    expect(isAcceptedHotkey(['F13'])).toBe(false);
    expect(isAcceptedHotkey(['A'])).toBe(false);
  });
  it('accepts the Super+Control chord', () => {
    expect(isAcceptedHotkey(['Super', 'Control'])).toBe(true);
  });
  it('requires a modifier plus a key', () => {
    expect(isAcceptedHotkey(['Super', 'L'])).toBe(true);
    expect(isAcceptedHotkey(['Shift', 'Enter'])).toBe(true);
    expect(isAcceptedHotkey(['L', 'Super'])).toBe(true);
    expect(isAcceptedHotkey(['A', 'B'])).toBe(false);
  });
});

describe('formatHotkey', () => {
  it('formats for macOS', () => {
    expect(formatHotkey('Super+L', { platform: 'macos' })).toBe('Command + L');
    expect(formatHotkey('Super+Control', { platform: 'macos' })).toBe('Command + Control');
    expect(formatHotkey('Alt+K', { platform: 'macos' })).toBe('Option + K');
    expect(formatHotkey('Super+L', { platform: 'macos', compact: true })).toBe('⌘L');
  });
  it('formats for windows/linux', () => {
    expect(formatHotkey('Super+L', { platform: 'windows' })).toBe('Windows + L');
    expect(formatHotkey('Super+Alt+T', { platform: 'linux' })).toBe('Windows + Alt + T');
  });
  it('falls back for empty hotkey', () => {
    expect(formatHotkey('', { fallback: 'your hotkey' })).toBe('your hotkey');
  });
});

describe('formatDuration', () => {
  it('formats m:ss', () => {
    expect(formatDuration(0)).toBe('0:00');
    expect(formatDuration(65)).toBe('1:05');
    expect(formatDuration(3599)).toBe('59:59');
  });
  it('formats h:mm:ss', () => {
    expect(formatDuration(3600)).toBe('1:00:00');
    expect(formatDuration(3661)).toBe('1:01:01');
  });
  it('handles invalid input', () => {
    expect(formatDuration(-1)).toBe('0:00');
    expect(formatDuration(Number.NaN)).toBe('0:00');
    expect(formatDuration(Number.POSITIVE_INFINITY)).toBe('0:00');
  });
});

describe('formatByteSize', () => {
  it('returns empty for <= 0', () => {
    expect(formatByteSize(0)).toBe('');
    expect(formatByteSize(-1)).toBe('');
  });
  it('formats MB', () => {
    expect(formatByteSize(1024 * 1024)).toBe('1 MB');
  });
  it('formats GB', () => {
    const oneGiB = 1024 * 1024 * 1024;
    expect(formatByteSize(oneGiB)).toBe('1 GB');
    expect(formatByteSize(Math.round(1.5 * oneGiB))).toBe('1.5 GB');
    expect(formatByteSize(Math.round(2 * oneGiB))).toBe('2 GB');
  });
});

describe('keyFromKeyboardEvent', () => {
  it('maps Key/ digit/ function codes', () => {
    expect(keyFromKeyboardEvent({ code: 'KeyA' } as KeyboardEvent)).toBe('A');
    expect(keyFromKeyboardEvent({ code: 'Digit7' } as KeyboardEvent)).toBe('7');
    expect(keyFromKeyboardEvent({ code: 'F5' } as KeyboardEvent)).toBe('F5');
  });
  it('maps aliases', () => {
    expect(keyFromKeyboardEvent({ code: 'Space' } as KeyboardEvent)).toBe('Space');
    expect(keyFromKeyboardEvent({ code: 'ArrowUp' } as KeyboardEvent)).toBe('ArrowUp');
  });
  it('falls back to event.key', () => {
    expect(keyFromKeyboardEvent({ code: '', key: 'x' } as KeyboardEvent)).toBe('X');
    expect(keyFromKeyboardEvent({ code: '', key: ' ' } as KeyboardEvent)).toBe('Space');
  });
});

describe('detectPlatform', () => {
  it('detects macOS from userAgent', () => {
    expect(detectPlatform()).toBe('unknown'); // jsdom default UA is not mac
  });
});