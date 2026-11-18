const SYMBOLS: Record<string, string> = { Control: '⌃', Option: '⌥', Alt: '⌥', Shift: '⇧', Super: '⌘' };
const NAMED: Record<string, string> = { Enter: 'return', Return: 'return', Backspace: 'delete', Delete: 'delete', Tab: 'tab', Escape: 'escape' };
const MODIFIER_ORDER = '⌃⌥⇧⌘';

const rank = (key: string) => MODIFIER_ORDER.indexOf(key);

export function formatShortcut(keys: string[]): string[] {
  const display = keys.map((key) => SYMBOLS[key] ?? NAMED[key] ?? (key.length === 1 ? key.toUpperCase() : key));
  const modifiers = display.filter((key) => rank(key) >= 0).sort((a, b) => rank(a) - rank(b));
  return [...modifiers, ...display.filter((key) => rank(key) < 0)];
}
