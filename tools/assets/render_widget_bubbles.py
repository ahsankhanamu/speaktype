#!/usr/bin/env python3
"""Render SpeakType widget bubbles from the real HTML/CSS via headless Chrome.

Generates high-quality transparent PNGs for each widget state into
assets/source/widgets/. Requires Chrome/Chromium and Pillow (pip install pillow).

Usage:
    python3 scripts/assets/render_widget_bubbles.py
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

SCRIPTS = Path(__file__).resolve().parent
ASSETS = Path(__file__).resolve().parents[2] / "assets"
HTML = SCRIPTS / "widget-bubble.html"
OUT_DIR = ASSETS / "source" / "widgets"

STATES = ("idle", "recording", "transcribing", "success", "error")


def find_chrome() -> str | None:
    env_path = os.environ.get("CHROME_PATH")
    if env_path and Path(env_path).exists():
        return env_path
    candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        shutil.which("google-chrome"),
        shutil.which("chromium"),
        shutil.which("chromium-browser"),
    ]
    for path in candidates:
        if path and Path(path).exists():
            return path
    return None


def crop_transparent(img: Image.Image, pad: int) -> Image.Image:
    if img.mode != "RGBA":
        img = img.convert("RGBA")
    alpha = img.split()[3]
    bbox = alpha.getbbox()
    if not bbox:
        return img
    x1, y1, x2, y2 = bbox
    x1 = max(0, x1 - pad)
    y1 = max(0, y1 - pad)
    x2 = min(img.width, x2 + pad)
    y2 = min(img.height, y2 + pad)
    return img.crop((x1, y1, x2, y2))


def render_state(
    chrome: str,
    state: str,
    out_path: Path,
    *,
    width: int,
    height: int,
    scale: int,
    pad: int,
    crop: bool,
) -> None:
    url = HTML.as_uri() + f"?state={state}"
    out_path.parent.mkdir(parents=True, exist_ok=True)

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        tmp_path = Path(tmp.name)

    try:
        cmd = [
            chrome,
            "--headless=new",
            "--disable-gpu",
            "--hide-scrollbars",
            "--default-background-color=00000000",
            f"--window-size={width},{height}",
            f"--force-device-scale-factor={scale}",
            f"--screenshot={tmp_path}",
            url,
        ]
        subprocess.run(cmd, check=True, capture_output=True)

        img = Image.open(tmp_path)
        if crop:
            img = crop_transparent(img, pad)
        img.save(out_path, optimize=True)
    finally:
        tmp_path.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser(description="Render SpeakType widget bubble PNGs")
    parser.add_argument("--scale", type=int, default=2, help="Device scale factor (default: 2)")
    parser.add_argument("--width", type=int, default=240, help="Viewport width in CSS px")
    parser.add_argument("--height", type=int, default=120, help="Viewport height in CSS px")
    parser.add_argument("--pad", type=int, default=12, help="Padding around crop box in px")
    parser.add_argument("--no-crop", action="store_true", help="Skip transparent margin crop")
    args = parser.parse_args()

    chrome = find_chrome()
    if not chrome:
        print("No Chrome/Chromium found — install Google Chrome or set CHROME_PATH", file=sys.stderr)
        return 1

    for state in STATES:
        out = OUT_DIR / f"{state}.png"
        print(f"Rendering {state} -> {out}")
        render_state(
            chrome,
            state,
            out,
            width=args.width,
            height=args.height,
            scale=args.scale,
            pad=args.pad,
            crop=not args.no_crop,
        )

    print(f"Done — {len(STATES)} bubbles in {OUT_DIR}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
