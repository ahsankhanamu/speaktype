#!/usr/bin/env python3
"""Compose the SpeakType DMG background (1x 600x380).

v5 design: a plain shade of gray with a single slate arrow baked in,
pointing from the app icon over to the Applications folder.  The arrow is
kept smaller than the icons and floats in the spacer gap between them, so
the layout reads cleanly and never suggests a misaligned element.
"""

from __future__ import annotations

import shutil
import subprocess
from io import BytesIO
from pathlib import Path

from PIL import Image, ImageDraw

from dmg_layout import ARROW_W, ARROW_X, ARROW_Y, BG_H, BG_W

ROOT = Path(__file__).resolve().parent
RES = ROOT / "dmg-resources"
OUT = RES / "background.png"
ARROW_SVG = RES / "arrow.svg"

# Vertical variation between two close grays (reads as flat, without enough
# banding to matter).  #cfd0d4 -> #bcbdc2 (a bit darker than before)
GRAY_TOP = (207, 208, 212)
GRAY_BOTTOM = (188, 189, 194)

# Slate arrow colour: soft, clearly visible on the light gray, not harsh black.
ARROW_COLOR = (84, 88, 98)  # #545862


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


def _recolor(image: Image.Image, rgb: tuple[int, int, int]) -> Image.Image:
    """Replace the arrow's black art with a given colour, keeping the alpha."""
    alpha = image.getchannel("A")
    tinted = Image.new("RGBA", image.size, (*rgb, 255))
    tinted.putalpha(alpha)
    return tinted


def compose() -> None:
    if not ARROW_SVG.exists():
        raise SystemExit(f"Missing {ARROW_SVG}")

    bg = Image.new("RGB", (BG_W, BG_H))
    draw = ImageDraw.Draw(bg)
    for y in range(BG_H):
        draw.line([(0, y), (BG_W, y)], fill=_lerp(GRAY_TOP, GRAY_BOTTOM, y / (BG_H - 1)))
    bg = bg.convert("RGBA")

    arrow = _recolor(_render_svg(ARROW_SVG, ARROW_W), ARROW_COLOR)
    bg.alpha_composite(arrow, (round(ARROW_X), round(ARROW_Y)))

    bg.convert("RGB").save(OUT, optimize=True)
    print(f"→ Composed {OUT} ({BG_W}x{BG_H}) with arrow at ({ARROW_X},{ARROW_Y})")


if __name__ == "__main__":
    compose()