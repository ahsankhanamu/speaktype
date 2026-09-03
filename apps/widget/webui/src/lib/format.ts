const MIB = 1024 * 1024;
const GIB = MIB * 1024;

function formatScaled(value: number, unit: string): string {
  if (value >= 100 || Math.abs(value - Math.round(value)) < 0.05) {
    return `${Math.round(value)} ${unit}`;
  }
  return `${value.toFixed(1)} ${unit}`;
}

export function formatByteSize(bytes: number): string {
  if (!bytes || bytes <= 0) return '';
  if (bytes >= GIB) {
    return formatScaled(bytes / GIB, 'GB');
  }
  return formatScaled(bytes / MIB, 'MB');
}

export function formatByteSizeApprox(bytes: number): string {
  const label = formatByteSize(bytes);
  return label ? `~${label}` : '';
}

export function formatSpeedMbps(mbps: number): string {
  if (!mbps || mbps <= 0) return '';
  if (mbps >= 100) return `${Math.round(mbps)} MB/s`;
  return `${mbps.toFixed(1)} MB/s`;
}

/** Format seconds as m:ss, or h:mm:ss when over an hour. */
export function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return '0:00';
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) {
    return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  }
  return `${m}:${String(s).padStart(2, '0')}`;
}

export function detectPlatform(): 'macos' | 'windows' | 'linux' | 'unknown' {
  const platform = typeof navigator !== 'undefined' ? navigator.platform || '' : '';
  const ua = typeof navigator !== 'undefined' ? navigator.userAgent || '' : '';
  if (/Mac|iPhone|iPad|iPod/.test(platform) || /Mac OS X/.test(ua)) return 'macos';
  if (/Win/.test(platform) || /Windows/.test(ua)) return 'windows';
  if (/Linux/.test(platform) || /Linux/.test(ua)) return 'linux';
  return 'unknown';
}

export function normalizeHotkey(hotkey: string): string {
  if (!hotkey) return hotkey;
  return hotkey
    .split('+')
    .map((part) => {
      const trimmed = part.trim();
      if (['CmdOrCtrl', 'CommandOrControl', 'Command', 'Meta'].includes(trimmed)) {
        return 'Super';
      }
      return trimmed;
    })
    .join('+');
}

export function isModifierKey(part: string): boolean {
  return ['Super', 'Control', 'Ctrl', 'Alt', 'Option', 'Shift'].includes(part);
}

export function isModifierChordHotkey(parts: string[]): boolean {
  return parts.length === 2 && parts.includes('Super') && parts.includes('Control');
}

export function isAcceptedHotkey(parts: string[]): boolean {
  if (parts.length === 1) {
    return /^F([1-9]|1[0-2])$/i.test(parts[0]);
  }
  if (isModifierChordHotkey(parts)) {
    return true;
  }
  const modifiers = parts.filter(isModifierKey);
  const nonModifiers = parts.filter((part) => !isModifierKey(part));
  return modifiers.length >= 1 && nonModifiers.length >= 1;
}

export function formatHotkeyPart(part: string, os: string): string {
  if (os === 'macos') {
    switch (part) {
      case 'Super':
        return 'Command';
      case 'Control':
      case 'Ctrl':
        return 'Control';
      case 'Alt':
      case 'Option':
        return 'Option';
      case 'Shift':
        return 'Shift';
      case 'Space':
        return 'Space';
      default:
        return part;
    }
  }

  if (os === 'windows' || os === 'linux') {
    switch (part) {
      case 'Super':
        return 'Windows';
      case 'Control':
      case 'Ctrl':
        return 'Control';
      case 'Alt':
      case 'Option':
        return 'Alt';
      case 'Shift':
        return 'Shift';
      case 'Space':
        return 'Space';
      default:
        return part;
    }
  }

  switch (part) {
    case 'Super':
      return 'Windows';
    case 'Control':
    case 'Ctrl':
      return 'Control';
    case 'Alt':
    case 'Option':
      return 'Alt';
    case 'Shift':
      return 'Shift';
    case 'Space':
      return 'Space';
    default:
      return part;
  }
}

export function formatHotkeyCompactPart(part: string, os: string): string {
  if (os === 'macos') {
    switch (part) {
      case 'Super':
        return '⌘';
      case 'Control':
      case 'Ctrl':
        return '⌃';
      case 'Alt':
      case 'Option':
        return '⌥';
      case 'Shift':
        return '⇧';
      case 'Space':
        return 'Space';
      default:
        return part;
    }
  }

  if (os === 'windows' || os === 'linux') {
    switch (part) {
      case 'Super':
        return '⊞';
      case 'Control':
      case 'Ctrl':
        return 'Control';
      case 'Alt':
      case 'Option':
        return 'Alt';
      case 'Shift':
        return 'Shift';
      case 'Space':
        return 'Space';
      default:
        return part;
    }
  }

  return formatHotkeyPart(part, os);
}

export function formatHotkey(
  hotkey: string,
  options: { platform?: string; compact?: boolean; fallback?: string } = {},
): string {
  if (!hotkey) return options.fallback || 'your hotkey';
  const os = options.platform || detectPlatform();
  const rawParts = normalizeHotkey(hotkey)
    .split('+')
    .map((part) => part.trim());
  const formatPart = options.compact ? formatHotkeyCompactPart : formatHotkeyPart;
  const parts = rawParts.map((part) => formatPart(part, os));
  if (options.compact) {
    return parts.join('');
  }
  return parts.join(' + ');
}

export function captureHotkeyModifiers(event: KeyboardEvent): string[] {
  const parts: string[] = [];

  if (event.metaKey) parts.push('Super');
  else if (event.ctrlKey) parts.push('Control');

  if (event.shiftKey) parts.push('Shift');
  if (event.altKey) parts.push('Alt');
  return parts;
}

const CODE_KEY_ALIASES: Record<string, string> = {
  Space: 'Space',
  Enter: 'Enter',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Escape: 'Escape',
  ArrowUp: 'ArrowUp',
  ArrowDown: 'ArrowDown',
  ArrowLeft: 'ArrowLeft',
  ArrowRight: 'ArrowRight',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
};

export function keyFromKeyboardEvent(event: KeyboardEvent): string | null {
  const code = event.code;
  if (code) {
    if (/^Key[A-Z]$/.test(code)) return code.slice(3);
    if (/^Digit\d$/.test(code)) return code.slice(5);
    if (/^F([1-9]|1[0-2])$/.test(code)) return code;
    if (CODE_KEY_ALIASES[code]) return CODE_KEY_ALIASES[code];
    return null;
  }

  const key = event.key;
  if (!key || key === ' ') return 'Space';
  if (key.length === 1) return key.toUpperCase();
  return key;
}