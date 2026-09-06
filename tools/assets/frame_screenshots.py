#!/usr/bin/env python3
"""Wrap raw headless-HTML captures in a polished macOS-style window mockup.

Renders the window chrome (title bar, traffic lights, rounded corners, border)
at a supersampled resolution, then downscales so the rounded corners and circles
are perfectly antialiased — no jagged "clip" edges.

The output PNG is exactly the window bounds (RGBA, corners transparent) so it
composites cleanly onto any background with no surplus padding.

Usage:
    .venv/bin/python3 tools/assets/frame_screenshots.py
"""

from __future__ import annotations

import glob
import os
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
RAW = ROOT / "assets" / "source" / "screenshots" / "raw"
FRAMED = ROOT / "assets" / "source" / "screenshots"

SUPERSAMPLE = 4  # render chrome at this scale, then downscale for crisp antialiased edges

# Reference chrome geometry measured from the original macOS screenshot of a window.
# REF_WIDTH scales only the corner radius / left inset proportionally per capture.
# Title-bar height, traffic-light size and their spacing are ABSOLUTE (fixed),
# like a real macOS title bar — they do not grow with window width.
REF_WIDTH = 1039
TITLEBAR_H = 80          # absolute title-bar height px (fixed, ~75% of the old scaled 107px)
DOT_SIZE = 34            # absolute traffic-light diameter px (75% of the old 46px render)
DOT_SPACING = 60         # absolute center-to-center spacing px
REF_INSET = 18           # window-edge to first dot LEFT edge
REF_RADIUS = 26          # window corner radius
DOT_COLORS = ((255, 95, 86), (255, 189, 46), (40, 201, 64))  # close / minim rest / max
TITLEBAR_COLOR = (44, 44, 48)
BORDER_COLOR = (46, 46, 51)
TITLE_TEXT_COLOR = (172, 172, 178)

# Per-window title-bar text, keyed by the screenshot name suffix
# (e.g. screenshot-06-about.png -> "about").
WINDOW_TITLES = {
    "general": "SpeakType Settings",
    "server": "SpeakType Settings",
    "models": "SpeakType Settings",
    "hotkey": "SpeakType Settings",
    "history": "SpeakType Settings",
    "debug": "SpeakType Settings",
    "about": "About SpeakType",
    "onboarding": "SpeakType",
}


def load_font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = [
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/SFNSText.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
    ]
    if bold:
        candidates = [
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/SFNSText.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ]
    for path in candidates:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size=size)
            except OSError:
                continue
    return ImageFont.load_default()


def frame_window(raw: Image.Image, title: str = "SpeakType Settings") -> Image.Image:
    cw, ch = raw.size
    unit = cw / REF_WIDTH
    tb = TITLEBAR_H
    dot = DOT_SIZE
    spacing = DOT_SPACING
    pad_left = max(1, round(REF_INSET * unit))
    radius = max(1, round(REF_RADIUS * unit))
    border = max(1, round(1.5 * unit))
    font_scale = tb * 0.34

    W, H = cw, ch + tb

    # Supersampled working canvas (all vector drawing lives at this resolution so the
    # final downscale antialiases every edge — corners, circles and text).
    s = SUPERSAMPLE
    sw, sh = W * s, H * s
    scr = radius * s
    stb = tb * s

    canvas = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))

    # 1. Content (app screenshot), clipped to the rounded window bounds.
    content = raw.convert("RGBA").resize((sw, ch * s), Image.Resampling.LANCZOS)
    content_layer = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
    content_layer.paste(content, (0, stb))

    mask = Image.new("L", (sw, sh), 0)
    dm = ImageDraw.Draw(mask)
    dm.rounded_rectangle((0, 0, sw - 1, sh - 1), radius=scr, fill=255)
    content_layer.putalpha(mask)
    canvas.paste(content_layer, (0, 0), content_layer)

    # 2. Title bar (rounded top, flat where it meets the content).
    titlebar = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
    td = ImageDraw.Draw(titlebar)
    td.rounded_rectangle((0, 0, sw - 1, stb), radius=scr, fill=TITLEBAR_COLOR + (255,))
    td.rectangle((0, stb - scr, sw - 1, stb), fill=TITLEBAR_COLOR + (255,))
    canvas.paste(titlebar, (0, 0), titlebar)

    # 3. Traffic lights (supersampled so they are perfect circles).
    draw = ImageDraw.Draw(canvas)
    r = (dot * s) // 2
    cy = (tb * s) // 2
    cx_first = (pad_left * s) + r
    for i, color in enumerate(DOT_COLORS):
        cx = cx_first + i * spacing * s
        draw.ellipse((cx - r, cy - r, cx + r, cy + r), fill=color + (255,))

    # 4. Centred window title.
    font = load_font(int(font_scale * s), bold=True)
    tw = draw.textlength(title, font=font)
    tx = (sw - tw) // 2 + (pad_left * s) // 2
    ty = cy - font.size // 2
    draw.text((tx, ty), title, fill=TITLE_TEXT_COLOR + (255,), font=font)

    # 5. Outer border.
    draw.rounded_rectangle(
        (0, 0, sw - 1, sh - 1),
        radius=scr,
        outline=BORDER_COLOR + (255,),
        width=max(1, border * s),
    )

    # Downscale: this produces perfectly smooth corners/circles.
    return canvas.resize((W, H), Image.Resampling.LANCZOS)


def main() -> None:
    RAW.mkdir(parents=True, exist_ok=True)
    FRAMED.mkdir(parents=True, exist_ok=True)
    files = sorted(glob.glob(str(RAW / "screenshot-*.png")))
    if not files:
        raise SystemExit(f"No raw screenshots found in {RAW}")
    for path in files:
        name = Path(path).stem  # e.g. screenshot-06-about
        key = name.split("-", 1)[1] if "-" in name else name  # -> "06-about" -> "about"
        key = key.split("-", 1)[-1]
        title = WINDOW_TITLES.get(key, "SpeakType Settings")
        img = Image.open(path).convert("RGBA")
        framed = frame_window(img, title)
        framed.save(FRAMED / f"{name}.png")
        print(f"Framed {name}.png -> {framed.size[0]}x{framed.size[1]} ({framed.mode}) [{title}]")


if __name__ == "__main__":
    main()