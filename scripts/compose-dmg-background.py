#!/usr/bin/env python3
"""Compose SpeakType DMG background (@2x 1320×800 @144dpi → 660×400pt window)."""

from __future__ import annotations

import shutil
import subprocess
from io import BytesIO
from pathlib import Path

from PIL import Image, ImageDraw

from dmg_layout import (
    APP_SLOT_RIGHT,
    APPS_VISUAL_LEFT,
    ARROW_W,
    ARROW_Y_OFFSET,
    ICON_CENTER_Y,
    BG_H,
    BG_W,
    ICON_SIZE,
    ICON_Y,
    SCALE,
)

ROOT = Path(__file__).resolve().parent
RES = ROOT / "dmg-resources"
OUT = RES / "background.png"
ARROW_SVG = RES / "arrow.svg"


def _light_background() -> Image.Image:
    img = Image.new("RGB", (BG_W, BG_H))
    draw = ImageDraw.Draw(img)
    for y in range(BG_H):
        t = y / (BG_H - 1)
        if t < 0.72:
            c = _lerp((247, 247, 248), (240, 240, 242), t / 0.72)
        else:
            c = _lerp((240, 240, 242), (228, 228, 232), (t - 0.72) / 0.28)
        draw.line([(0, y), (BG_W, y)], fill=c)
    return img


def _lerp(a: tuple[int, int, int], b: tuple[int, int, int], t: float) -> tuple[int, int, int]:
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def _render_svg(svg: Path, width: int) -> Image.Image:
    if not shutil.which("rsvg-convert"):
        raise SystemExit("Need rsvg-convert (brew install librsvg)")
    result = subprocess.run(
        ["rsvg-convert", "-w", str(width), str(svg)],
        check=True,
        capture_output=True,
    )
    return Image.open(BytesIO(result.stdout)).convert("RGBA")


def _arrow_position(arrow: Image.Image) -> tuple[int, int]:
    gap_left = APP_SLOT_RIGHT * SCALE
    gap_right = APPS_VISUAL_LEFT * SCALE
    x = (gap_left + gap_right - arrow.width) // 2
    anchor_y = ICON_CENTER_Y * SCALE
    y = anchor_y - arrow.height // 2 - ARROW_Y_OFFSET * SCALE
    return x, y


def compose() -> None:
    if not ARROW_SVG.exists():
        raise SystemExit(f"Missing {ARROW_SVG}")

    bg = _light_background().convert("RGBA")
    arrow = _render_svg(ARROW_SVG, ARROW_W)
    bg.alpha_composite(arrow, _arrow_position(arrow))
    bg.convert("RGB").save(OUT, optimize=True)
    print(f"→ Composed {OUT} ({BG_W}x{BG_H})")


if __name__ == "__main__":
    compose()
