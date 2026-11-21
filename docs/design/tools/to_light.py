"""Derive a light-mode board from a dark board: python3 to_light.py in.dc.html out.dc.html

Colours are mapped by the CSS property they sit in (text vs surface vs shadow),
using Apple's light system colours; elements marked data-keep-theme are left as is.
"""
import re
import sys

ACCENT = {
    '#0a84ff': '#007aff', '#2a93ff': '#1a88ff', '#0070d9': '#0062cc',
    '#4da3ff': '#007aff', '#7cbcff': '#0062cc', '#ff453a': '#ff3b30',
    '#ff6961': '#e0352b', '#30d158': '#34c759', '#ff9f0a': '#ff9500',
}
SURFACE = {
    '#1e1e1e': '#f5f5f7', '#1c1c1e': '#ececf0', '#262628': '#e9e9ed',
    '#2a2a2c': '#ffffff', '#2c2c2e': '#ffffff', '#2c2c2f': '#ffffff',
    '#3a3a3c': '#e5e5ea', '#3c3c3f': '#e5e5ea', '#48484b': '#d8d8dc',
    '#4a4a4d': '#e3e3e8', '#4a4a4e': '#e3e3e8', '#535357': '#ffffff',
    '#404044': '#f1f1f4', '#636366': '#c7c7cc',
}
RGBA = re.compile(r'rgba\((\d+),\s*(\d+),\s*(\d+),\s*([\d.]+)\)')
HEX = re.compile(r'#[0-9a-fA-F]{6}\b|#[0-9a-fA-F]{3}\b')
ON_COLOUR = re.compile(r'background(-color)?:\s*(#0a84ff|#ff453a|#30d158|#ef643c|#ff9f0a)', re.I)


def full_hex(h):
    h = h.lower()
    return '#' + ''.join(c * 2 for c in h[1:]) if len(h) == 4 else h


def text(v):
    v = re.sub(r'#fff(fff)?\b', 'rgba(0,0,0,.88)', v, flags=re.I)
    v = RGBA.sub(lambda m: f'rgba(0,0,0,{m[4]})' if m.groups()[:3] == ('255', '255', '255') else m[0], v)
    return HEX.sub(lambda m: ACCENT.get(full_hex(m[0]), m[0]), v)


def surface(v):
    def rgba(m):
        a = float(m[4])
        if m.groups()[:3] == ('255', '255', '255'):
            return f'rgba(0,0,0,{round(a * .55, 3)})'
        if m.groups()[:3] == ('0', '0', '0'):
            return f'rgba(0,0,0,{round(a * .55, 3)})'
        if m.groups()[:3] == ('40', '40', '42'):
            return 'rgba(255,255,255,.98)'
        if m.groups()[:3] == ('10', '132', '255'):
            return f'rgba(0,122,255,{m[4]})'
        return m[0]
    v = RGBA.sub(rgba, v)
    return HEX.sub(lambda m: SURFACE.get(full_hex(m[0]), ACCENT.get(full_hex(m[0]), m[0])), v)


def shadow(v):
    if 'rgba(255,255,255,.14), 0 0 0 1px rgba(0,0,0,.55)' in v.replace(' ,', ','):
        return '0 0 0 .5px rgba(0,0,0,.12),0 4px 12px rgba(0,0,0,.14)'
    parts = re.split(r',(?![^(]*\))', v)
    out = []
    for p in parts:
        m = RGBA.search(p)
        if m and m.groups()[:3] == ('255', '255', '255') and re.search(r'0 0 0 [\d.]+px', p):
            p = RGBA.sub(f'rgba(0,0,0,{round(float(m[4]) * .8, 3)})', p, count=1)
        elif m and m.groups()[:3] == ('0', '0', '0'):
            p = RGBA.sub(f'rgba(0,0,0,{round(float(m[4]) * .5, 3)})', p, count=1)
        elif m and m.groups()[:3] == ('10', '132', '255'):
            p = RGBA.sub(f'rgba(0,122,255,{m[4]})', p, count=1)
        p = HEX.sub(lambda h: SURFACE.get(full_hex(h[0]), ACCENT.get(full_hex(h[0]), h[0])), p)
        out.append(p)
    return ','.join(out)


def block(decls):
    keep_white = bool(ON_COLOUR.search(decls))
    glyph = bool(re.search(r'(?<![-\w])width:\s*([1-9]|10)px', decls))
    out = []
    for d in re.split(r';(?![^(]*\))', decls):
        if ':' not in d:
            out.append(d)
            continue
        prop, val = d.split(':', 1)
        p = prop.strip().lower()
        if p == 'color' and not keep_white:
            val = text(val)
        elif p in ('color',):
            val = HEX.sub(lambda m: ACCENT.get(full_hex(m[0]), m[0]), val)
        elif p.startswith('background') and glyph and re.search(r'#fff\b', val, re.I):
            val = 'rgba(0,0,0,.75)'
        elif p.startswith('background'):
            val = surface(val)
        elif p == 'box-shadow':
            val = shadow(val)
        elif p in ('border', 'border-color', 'border-top', 'border-bottom', 'outline', 'caret-color'):
            val = text(val)
        out.append(f'{prop}:{val}')
    return ';'.join(out)


def stash_kept(html, keep):
    """Replace each element carrying data-keep-theme (with its subtree) by a placeholder."""
    out, pos = [], 0
    for m in re.finditer(r'<(\w+)[^>]*\bdata-keep-theme\b[^>]*>', html):
        if m.start() < pos:
            continue
        tag, depth, j = m[1], 1, m.end()
        for t in re.finditer(rf'<(/?){tag}\b[^>]*>', html[m.end():]):
            depth += -1 if t[1] else 1
            if depth == 0:
                j = m.end() + t.end()
                break
        out += [html[pos:m.start()], f'\0{len(keep)}\0']
        keep.append(html[m.start():j])
        pos = j
    return ''.join(out) + html[pos:]


def convert(html):
    keep = []
    html = stash_kept(html, keep)
    html = re.sub(r'(<style>)(.*?)(</style>)', lambda m: m[1] + re.sub(r'\{([^{}]*)\}', lambda b: '{' + block(b[1]) + '}', m[2]) + m[3], html, flags=re.S)
    html = re.sub(r'style="([^"]*)"', lambda m: f'style="{block(m[1])}"', html)
    html = re.sub(r'(stroke|fill)="([^"]+)"', lambda m: f'{m[1]}="{text(m[2]) if m[2] not in ("none", "currentColor") else m[2]}"', html)
    return re.sub(r'\0(\d+)\0', lambda m: keep[int(m[1])], html)


if __name__ == '__main__':
    src, dst = sys.argv[1], sys.argv[2]
    out = convert(open(src).read()).replace('</title>', ' (light)</title>', 1)
    open(dst, 'w').write(out)
