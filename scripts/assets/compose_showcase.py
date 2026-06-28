#!/usr/bin/env python3
"""Compose README showcase slides and demo animation from source assets.

Source files (inputs):
    assets/source/screenshots/screenshot-*.png
    assets/source/widgets/{idle,recording,transcribing,success,error}.png

Outputs (composed):
    assets/showcase-*.png, assets/demo.webp, assets/demo.gif

Usage:
    python3 scripts/assets/compose_showcase.py
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = Path(__file__).resolve().parent
ASSETS = ROOT / "assets"
SOURCE_SCREENSHOTS = ASSETS / "source" / "screenshots"
SOURCE_WIDGETS = ASSETS / "source" / "widgets"
OUTPUT = ASSETS
CANVAS = (1280, 800)
BG = (0, 0, 0)
PANEL = (245, 245, 245)
TITLE_COLOR = (255, 255, 255)
SUB_COLOR = (160, 160, 160)
DOT_ACTIVE = (59, 130, 246)
DOT_IDLE = (80, 80, 80)

# Fixed white panel — identical size on every slide; only inner content changes
PANEL_MARGIN_X = 48
PANEL_TOP = 36
BOTTOM_MARGIN = 15
WHITE_TO_FOOTER_GAP = 15

FOOTER_TITLE_SIZE = 42
FOOTER_SUB_SIZE = 22
FOOTER_TITLE_TO_SUB = 10
FOOTER_SUB_TO_DOTS = 18
FOOTER_DOT_R = 5

# Footer stacked upward from the floor
FOOTER_DOTS_Y = CANVAS[1] - BOTTOM_MARGIN - FOOTER_DOT_R
FOOTER_SUB_Y = FOOTER_DOTS_Y - FOOTER_DOT_R - FOOTER_SUB_TO_DOTS - FOOTER_SUB_SIZE
FOOTER_TITLE_Y = FOOTER_SUB_Y - FOOTER_TITLE_TO_SUB - FOOTER_TITLE_SIZE
PANEL_BOTTOM = FOOTER_TITLE_Y - WHITE_TO_FOOTER_GAP
PANEL_BOX = (PANEL_MARGIN_X, PANEL_TOP, CANVAS[0] - PANEL_MARGIN_X, PANEL_BOTTOM)

SCREENSHOT_PAD = 10
WIDGET_GAP = 20
CONTENT_FILL = 0.94

WIDGET_DISPLAY_SCALE = 0.6
WIDGET_MAX_WIDTH = {
    "idle": 104,
    "success": 104,
    "error": 104,
    "recording": 178,
    "transcribing": 178,
}

SCREENSHOTS = {
    "general": SOURCE_SCREENSHOTS / "screenshot-01-general.png",
    "server": SOURCE_SCREENSHOTS / "screenshot-02-server.png",
    "models": SOURCE_SCREENSHOTS / "screenshot-03-models.png",
    "hotkey": SOURCE_SCREENSHOTS / "screenshot-04-hotkey.png",
    "history": SOURCE_SCREENSHOTS / "screenshot-05-history.png",
    "about": SOURCE_SCREENSHOTS / "screenshot-06-about.png",
}


def screenshot_box() -> tuple[int, int, int, int]:
    x1, y1, x2, y2 = PANEL_BOX
    return (x1 + SCREENSHOT_PAD, y1 + SCREENSHOT_PAD, x2 - SCREENSHOT_PAD, y2 - SCREENSHOT_PAD)


def require_screenshots() -> None:
    missing = [name for name, path in SCREENSHOTS.items() if not path.exists()]
    if missing:
        lines = "\n".join(f"  {SCREENSHOTS[n]}" for n in missing)
        raise FileNotFoundError(f"Missing screenshots:\n{lines}")


def load_font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = [
        "/System/Library/Fonts/SFNSText.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf" if bold else "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
    ]
    for path in candidates:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size=size)
            except OSError:
                continue
    return ImageFont.load_default()


def rounded_panel(size: tuple[int, int], radius: int = 24) -> Image.Image:
    w, h = size
    panel = Image.new("RGBA", size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(panel)
    draw.rounded_rectangle((0, 0, w - 1, h - 1), radius=radius, fill=PANEL + (255,))
    return panel


def render_widget_bubbles() -> None:
    script = SCRIPTS / "render_widget_bubbles.py"
    result = subprocess.run([sys.executable, str(script)], capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(
            "Failed to render widget bubbles:\n"
            + (result.stderr or result.stdout or "unknown error")
        )
    print(result.stdout.strip())


def load_widget(kind: str) -> Image.Image:
    path = SOURCE_WIDGETS / f"{kind}.png"
    if not path.exists():
        raise FileNotFoundError(f"Missing widget source: {path}")
    return Image.open(path).convert("RGBA")


def fit_image(img: Image.Image, box: tuple[int, int, int, int], max_height: int | None = None) -> Image.Image:
    x1, y1, x2, y2 = box
    max_w, max_h = x2 - x1, y2 - y1
    if max_height is not None:
        max_h = min(max_h, max_height)
    scale = min(max_w / img.width, max_h / img.height)
    new_size = (max(1, int(img.width * scale)), max(1, int(img.height * scale)))
    return img.resize(new_size, Image.Resampling.LANCZOS)


def paste_centered(base: Image.Image, overlay: Image.Image, box: tuple[int, int, int, int]) -> None:
    x1, y1, x2, y2 = box
    ox = x1 + ((x2 - x1) - overlay.width) // 2
    oy = y1 + ((y2 - y1) - overlay.height) // 2
    base.paste(overlay, (ox, oy), overlay if overlay.mode == "RGBA" else None)


def paste_in_box(
    base: Image.Image,
    overlay: Image.Image,
    box: tuple[int, int, int, int],
    *,
    align: str = "center",
    pad: int = 8,
) -> None:
    x1, y1, x2, y2 = box
    if align == "left":
        ox = x1 + pad
    else:
        ox = x1 + ((x2 - x1) - overlay.width) // 2
    oy = y1 + ((y2 - y1) - overlay.height) // 2
    base.paste(overlay, (ox, oy), overlay if overlay.mode == "RGBA" else None)


def prepare_widget(kind: str) -> Image.Image:
    widget = load_widget(kind)
    w, h = widget.size
    scaled = widget.resize(
        (max(1, int(w * WIDGET_DISPLAY_SCALE)), max(1, int(h * WIDGET_DISPLAY_SCALE))),
        Image.Resampling.LANCZOS,
    )
    max_w = WIDGET_MAX_WIDTH.get(kind)
    if max_w and scaled.width > max_w:
        ratio = max_w / scaled.width
        scaled = scaled.resize(
            (max(1, int(scaled.width * ratio)), max(1, int(scaled.height * ratio))),
            Image.Resampling.LANCZOS,
        )
    elif max_w and scaled.width < max_w and kind in ("idle", "success", "error"):
        ratio = max_w / scaled.width
        scaled = scaled.resize(
            (max(1, int(scaled.width * ratio)), max(1, int(scaled.height * ratio))),
            Image.Resampling.LANCZOS,
        )
    return scaled


def draw_fixed_panel(canvas: Image.Image) -> None:
    px1, py1, px2, py2 = PANEL_BOX
    panel = rounded_panel((px2 - px1, py2 - py1))
    canvas.paste(panel, (px1, py1), panel)


def layout_content(
    shot: Image.Image,
    widget_kind: str,
) -> tuple[Image.Image, Image.Image, int, int]:
    """Fit screenshot + widget, scale up together to fill the white panel."""
    box = screenshot_box()
    bx1, by1, bx2, by2 = box
    box_w, box_h = bx2 - bx1, by2 - by1

    fitted = fit_image(shot, box)
    widget = prepare_widget(widget_kind)

    group_w = fitted.width + WIDGET_GAP + widget.width
    scale = min(
        box_w * CONTENT_FILL / group_w,
        box_h * CONTENT_FILL / fitted.height,
        1.5,
    )
    if scale > 1.005:
        fitted = fitted.resize(
            (max(1, int(fitted.width * scale)), max(1, int(fitted.height * scale))),
            Image.Resampling.LANCZOS,
        )
        widget = widget.resize(
            (max(1, int(widget.width * scale)), max(1, int(widget.height * scale))),
            Image.Resampling.LANCZOS,
        )

    group_w = fitted.width + WIDGET_GAP + widget.width
    shot_x = bx1 + (box_w - group_w) // 2
    shot_y = by1 + (box_h - fitted.height) // 2
    return fitted, widget, shot_x, shot_y


def paste_content(
    canvas: Image.Image,
    fitted: Image.Image,
    widget: Image.Image,
    shot_x: int,
    shot_y: int,
) -> None:
    canvas.paste(fitted, (shot_x, shot_y), fitted)
    ox = shot_x + fitted.width + WIDGET_GAP
    oy = shot_y + (fitted.height - widget.height) // 2
    canvas.paste(widget, (ox, oy), widget)


def draw_footer(draw: ImageDraw.ImageDraw, title: str, subtitle: str, active_dot: int, dot_count: int = 6) -> None:
    title_font = load_font(42, bold=True)
    sub_font = load_font(22)
    tw = draw.textlength(title, font=title_font)
    draw.text(((CANVAS[0] - tw) / 2, FOOTER_TITLE_Y), title, fill=TITLE_COLOR, font=title_font)
    sw = draw.textlength(subtitle, font=sub_font)
    draw.text(((CANVAS[0] - sw) / 2, FOOTER_SUB_Y), subtitle, fill=SUB_COLOR, font=sub_font)

    spacing = 12
    dot_d = FOOTER_DOT_R * 2
    pill_w = 26
    pill_h = 10
    y = FOOTER_DOTS_Y

    segment_w = [pill_w if i == active_dot else dot_d for i in range(dot_count)]
    total_w = sum(segment_w) + spacing * (dot_count - 1)
    x = (CANVAS[0] - total_w) / 2
    for i in range(dot_count):
        if i == active_dot:
            draw.rounded_rectangle(
                (x, y - pill_h // 2, x + pill_w, y + pill_h // 2),
                radius=pill_h // 2,
                fill=DOT_ACTIVE,
            )
            x += pill_w + spacing
        else:
            draw.ellipse(
                (x, y - FOOTER_DOT_R, x + dot_d, y + FOOTER_DOT_R),
                fill=DOT_IDLE,
            )
            x += dot_d + spacing


def compose_settings_slide(
    screenshot: Path,
    title: str,
    subtitle: str,
    dot: int,
    widget_kind: str,
) -> Image.Image:
    canvas = Image.new("RGB", CANVAS, BG)
    draw_fixed_panel(canvas)

    shot = Image.open(screenshot).convert("RGBA")
    fitted, widget, shot_x, shot_y = layout_content(shot, widget_kind)
    paste_content(canvas, fitted, widget, shot_x, shot_y)

    draw = ImageDraw.Draw(canvas)
    draw_footer(draw, title, subtitle, dot)
    return canvas


def compose_about_slide(
    screenshot: Path,
    title: str,
    subtitle: str,
    dot: int,
    backdrop: Path | None = None,
) -> Image.Image:
    canvas = Image.new("RGB", CANVAS, BG)
    draw_fixed_panel(canvas)

    if backdrop is not None:
        back = Image.open(backdrop).convert("RGBA")
        box = screenshot_box()
        fitted_back = fit_image(back, box)
        paste_centered(canvas, fitted_back, box)

    about = Image.open(screenshot).convert("RGBA")
    fitted, widget, shot_x, shot_y = layout_content(about, "idle")
    paste_content(canvas, fitted, widget, shot_x, shot_y)

    draw = ImageDraw.Draw(canvas)
    draw_footer(draw, title, subtitle, dot)
    return canvas


def _quantize_frame(frame: Image.Image) -> Image.Image:
    """Per-frame palette so recording reds aren't washed out by other slides."""
    method = Image.Quantize.FASTOCTREE if hasattr(Image.Quantize, "FASTOCTREE") else Image.Quantize.MEDIANCUT
    return frame.quantize(colors=256, method=method, dither=Image.Dither.NONE)


def build_animation(
    slides: list[Image.Image],
    out_path: Path,
    *,
    hold_frames: int = 60,
    frame_ms: int = 33,
) -> None:
    """Hard-cut slide animation — no crossfade to avoid blur."""
    frames: list[Image.Image] = []
    for slide in slides:
        for _ in range(hold_frames):
            frames.append(slide.copy())

    fmt = out_path.suffix.lower()
    if fmt == ".gif":
        # Per-frame local palettes — shared palette turns recording red into dull brown
        q_frames = [_quantize_frame(f) for f in frames]
        q_frames[0].save(
            out_path,
            save_all=True,
            append_images=q_frames[1:],
            duration=frame_ms,
            loop=0,
            disposal=2,
            optimize=False,
        )
        return

    save_kwargs: dict = {
        "save_all": True,
        "append_images": frames[1:],
        "duration": frame_ms,
        "loop": 0,
    }
    if fmt == ".webp":
        save_kwargs["lossless"] = True

    frames[0].save(out_path, **save_kwargs)


def main() -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    require_screenshots()
    render_widget_bubbles()

    slides = [
        compose_about_slide(
            SCREENSHOTS["about"],
            "About",
            "Voice typing that works everywhere",
            0,
        ),
        compose_settings_slide(
            SCREENSHOTS["hotkey"],
            "Idle",
            "Ready — press your hotkey to start",
            1,
            "idle",
        ),
        compose_settings_slide(
            SCREENSHOTS["general"],
            "Recording",
            "Capturing audio from your microphone",
            2,
            "recording",
        ),
        compose_settings_slide(
            SCREENSHOTS["server"],
            "Transcribing",
            "Processing speech with Whisper AI",
            3,
            "transcribing",
        ),
        compose_settings_slide(
            SCREENSHOTS["history"],
            "Success",
            "Text pasted into your active window",
            4,
            "success",
        ),
        compose_settings_slide(
            SCREENSHOTS["models"],
            "Models",
            "Download and manage Whisper models",
            5,
            "idle",
        ),
    ]

    names = [
        "showcase-1-about.png",
        "showcase-2-idle.png",
        "showcase-3-recording.png",
        "showcase-4-transcribing.png",
        "showcase-5-success.png",
        "showcase-6-models.png",
    ]
    for slide, name in zip(slides, names):
        out = OUTPUT / name
        slide.save(out, optimize=True)
        print(f"Wrote {out}")

    for demo_name in ("demo.webp", "demo.gif"):
        demo_path = OUTPUT / demo_name
        build_animation(slides, demo_path)
        print(f"Wrote {demo_path}")

    preview = OUTPUT / "demo-preview.gif"
    if preview.exists():
        preview.unlink()
        print(f"Removed obsolete {preview}")

    print(f"Done — {len(slides)} showcase slides + demo animations in {OUTPUT}")


if __name__ == "__main__":
    main()
