import { describe, expect, test } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const css = readFileSync(resolve(__dirname, 'tokens.css'), 'utf8');

type Vars = Record<string, string>;

const blocks = new Map<string, Vars>();
for (const [, selector, body] of css.matchAll(/([^{}@]+)\{([^{}]*)\}/g)) {
  const vars = blocks.get(selector.trim()) ?? {};
  for (const [, name, value] of body.matchAll(/(--[\w-]+):\s*([^;]+);/g)) vars[name] = value.trim().replace(/\s+/g, ' ');
  blocks.set(selector.trim(), vars);
}

const dark = blocks.get(':root')!;
const light = blocks.get(':root[data-theme="light"]')!;
const systemLight = blocks.get(':root:not([data-theme="dark"])')!;

const THEMED = Object.keys(light);

type RGBA = [number, number, number, number];

function parse(value: string): RGBA {
  if (value.startsWith('#')) {
    const n = parseInt(value.slice(1), 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 1];
  }
  const [r, g, b, a = 1] = value.match(/[\d.]+/g)!.map(Number);
  return [r, g, b, a];
}

function blend(fg: string, bg: string): RGBA {
  const [r, g, b, a] = parse(fg);
  const [br, bgG, bb] = parse(bg);
  return [r * a + br * (1 - a), g * a + bgG * (1 - a), b * a + bb * (1 - a), 1];
}

function luminance([r, g, b]: RGBA) {
  const [lr, lg, lb] = [r, g, b].map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
}

function contrast(fg: RGBA, bg: string) {
  const [a, b] = [luminance(fg), luminance(parse(bg))];
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

function hueSaturation([r, g, b]: RGBA) {
  const [rn, gn, bn] = [r / 255, g / 255, b / 255];
  const max = Math.max(rn, gn, bn);
  const d = max - Math.min(rn, gn, bn);
  if (d === 0) return { hue: 0, saturation: 0 };
  const hue = max === rn ? ((gn - bn) / d) % 6 : max === gn ? (bn - rn) / d + 2 : (rn - gn) / d + 4;
  return { hue: (hue * 60 + 360) % 360, saturation: d / max };
}

describe('tokens', () => {
  test('every themed token is defined for dark and light', () => {
    for (const name of THEMED) {
      expect(dark[name], name).toBeDefined();
      expect(light[name], name).toBeDefined();
    }
  });

  test('the system-light block repeats the explicit light block', () => {
    expect(systemLight).toEqual(light);
  });

  test('body text meets WCAG AA on window and group in both themes', () => {
    for (const theme of [dark, light])
      for (const bg of ['--st-window', '--st-group'])
        expect(contrast(blend(theme['--st-text'], theme[bg]), theme[bg]), bg).toBeGreaterThanOrEqual(4.5);
  });

  test('secondary text meets 4.5:1 on group in both themes', () => {
    for (const theme of [dark, light])
      expect(contrast(blend(theme['--st-text-secondary'], theme['--st-group']), theme['--st-group'])).toBeGreaterThanOrEqual(4.5);
  });

  test('no yellow outside the highlighter ink', () => {
    const isYellow = (value: string) => {
      const { hue, saturation } = hueSaturation(parse(value));
      return hue >= 45 && hue <= 65 && saturation > 0.6;
    };
    for (const theme of [dark, light])
      for (const [name, value] of Object.entries(theme))
        if (/^(#|rgb)/.test(value) && name !== '--st-ink-highlighter') expect(isYellow(value), name).toBe(false);
    expect(isYellow(dark['--st-ink-highlighter'])).toBe(true);
  });
});
