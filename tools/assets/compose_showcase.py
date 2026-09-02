#!/usr/bin/env python3
"""Compose README showcase slides and demo animation from source assets.

Source files (inputs):
    assets/source/screenshots/screenshot-*.png
    assets/source/widgets/{idle,recording,transcribing,success,error}.png

Outputs (composed):
    assets/showcase-*.png, assets/showcase-flow.png, assets/showcase-flow-linkedin.png, assets/demo.webp, assets/demo.gif

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


def prepare_widget(kind: str, *, scale_mult: float = 1.0) -> Image.Image:
    widget = load_widget(kind)
    w, h = widget.size
    display_scale = WIDGET_DISPLAY_SCALE * scale_mult
    scaled = widget.resize(
        (max(1, int(w * display_scale)), max(1, int(h * display_scale))),
        Image.Resampling.LANCZOS,
    )
    max_w = WIDGET_MAX_WIDTH.get(kind)
    if max_w:
        max_w = int(max_w * scale_mult)
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


FLOW_WIDGET_KINDS = ("idle", "recording", "transcribing", "success")
FLOW_LABELS = ("Idle", "Recording", "Transcribing", "Success")
FLOW_PASTE_LABELS = ("Empty field", "Text pasted")
LINKEDIN_PASTE_LABELS = ("Focused field", "Auto pasted")
FLOW_SAMPLE_TEXT = (
    "Press your hotkey, speak, and press again — your text appears "
    "in whatever app is focused."
)
FLOW_ARROW_COLOR = (100, 100, 100)
FLOW_LABEL_COLOR = (110, 110, 110)
FLOW_DIVIDER_COLOR = (220, 220, 220)
FLOW_COLUMN_GAP = 32
FLOW_INNER_PAD = 24
FLOW_ARROW_HEAD = 9
FLOW_ARROW_STEM = 22
FLOW_ARROW_GAP = 10
FLOW_LABEL_GAP = 6
FLOW_TEXTBOX_BORDER = (209, 213, 219)
FLOW_TEXTBOX_BG = (255, 255, 255)
FLOW_TEXTBOX_TEXT = (31, 41, 55)
FLOW_TEXTBOX_PLACEHOLDER = (156, 163, 175)
FLOW_TEXTBOX_CURSOR = (59, 130, 246)
FLOW_TEXTBOX_CHROME = (243, 244, 246)
FLOW_COL_MODELS = 0.40
FLOW_COL_WIDGETS = 0.26
LINKEDIN_COL_PAD = 4
LINKEDIN_COL_GAP = 24
LINKEDIN_PANEL_MARGIN_X = 24
LINKEDIN_PANEL_INNER_PAD = 12
LINKEDIN_BASE = (960, 628)
LINKEDIN_CANVAS_HEIGHT = 1256
FLOW_HEADLINE_TITLE = "SpeakType"
FLOW_HEADLINE_SUBTITLE = "Local voice typing for any app"
FLOW_COLUMN_TITLES = ("Setup", "Dictate", "Paste")
LINKEDIN_COLUMN_TITLES = ("Setup", "Dictate", "Auto paste")


def _linkedin_scale() -> float:
    return 2.0


def _li(value: float) -> int:
    return int(round(value * _linkedin_scale()))


def draw_down_arrow(
    draw: ImageDraw.ImageDraw,
    x: int,
    y_top: int,
    y_bottom: int,
    *,
    color: tuple[int, int, int] = FLOW_ARROW_COLOR,
    width: int = 2,
    head: int = FLOW_ARROW_HEAD,
) -> None:
    stem_bottom = y_bottom - head
    if stem_bottom > y_top:
        draw.line((x, y_top, x, stem_bottom), fill=color, width=width)
    draw.polygon(
        [(x, y_bottom), (x - head, y_bottom - head), (x + head, y_bottom - head)],
        fill=color,
    )


def draw_right_arrow(
    draw: ImageDraw.ImageDraw,
    x_left: int,
    x_right: int,
    y: int,
    *,
    color: tuple[int, int, int] = FLOW_ARROW_COLOR,
    width: int = 2,
    head: int = FLOW_ARROW_HEAD,
) -> None:
    stem_right = x_right - head
    if stem_right > x_left:
        draw.line((x_left, y, stem_right, y), fill=color, width=width)
    draw.polygon(
        [(x_right, y), (x_right - head, y - head), (x_right - head, y + head)],
        fill=color,
    )


def _scale_widgets_to_fit(
    widgets: list[Image.Image],
    max_w: int,
    max_h: int,
    *,
    arrow_stem: int,
    label_h: int,
    gap: int,
) -> list[Image.Image]:
    arrow_block = arrow_stem + gap * 2
    total_h = sum(w.height for w in widgets) + arrow_block * (len(widgets) - 1) + label_h * len(widgets)
    max_widget_w = max(w.width for w in widgets)
    scale = min(max_w / max_widget_w, max_h / total_h, 1.0)
    if scale >= 0.999:
        return widgets
    return [
        w.resize((max(1, int(w.width * scale)), max(1, int(w.height * scale))), Image.Resampling.LANCZOS)
        for w in widgets
    ]


def _wrap_text(text: str, font: ImageFont.FreeTypeFont | ImageFont.ImageFont, max_width: float) -> list[str]:
    words = text.split()
    lines: list[str] = []
    current: list[str] = []
    probe = ImageDraw.Draw(Image.new("RGB", (1, 1)))
    for word in words:
        trial = " ".join([*current, word])
        if probe.textlength(trial, font=font) <= max_width:
            current.append(word)
        else:
            if current:
                lines.append(" ".join(current))
            current = [word]
    if current:
        lines.append(" ".join(current))
    return lines


def render_textbox_mock(
    size: tuple[int, int],
    *,
    text: str | None = None,
    font_size: int = 15,
) -> Image.Image:
    w, h = size
    panel = Image.new("RGBA", size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(panel)
    radius = max(8, int(12 * w / 220))
    chrome_h = max(20, int(28 * h / 108))

    draw.rounded_rectangle((0, 0, w - 1, h - 1), radius=radius, fill=FLOW_TEXTBOX_BG + (255,), outline=FLOW_TEXTBOX_BORDER, width=max(1, w // 220))
    draw.rectangle((1, chrome_h, w - 2, h - 2), fill=FLOW_TEXTBOX_BG + (255,))
    draw.rounded_rectangle((0, 0, w - 1, chrome_h + 6), radius=radius, fill=FLOW_TEXTBOX_CHROME + (255,))
    dot_r = max(3, int(4 * w / 220))
    for i, color in enumerate(((239, 68, 68), (234, 179, 8), (34, 197, 94))):
        cx = 10 + i * 14 * w // 220
        draw.ellipse((cx, 10 * h // 108, cx + dot_r * 2, 10 * h // 108 + dot_r * 2), fill=color)

    pad_x = max(10, int(14 * w / 220))
    pad_y = chrome_h + max(8, int(12 * h / 108))
    inner_w = w - pad_x * 2
    inner_h = h - pad_y - max(8, int(12 * h / 108))
    body_font = load_font(font_size)
    line_h = int(body_font.size * 1.45)

    if text:
        y = pad_y
        for line in _wrap_text(text, body_font, inner_w):
            draw.text((pad_x, y), line, fill=FLOW_TEXTBOX_TEXT, font=body_font)
            y += line_h
            if y > pad_y + inner_h - line_h:
                break
    else:
        draw.text((pad_x, pad_y), "Type here…", fill=FLOW_TEXTBOX_PLACEHOLDER, font=body_font)
        draw.rectangle((pad_x + 2, pad_y + 2, pad_x + 3, pad_y + line_h - 4), fill=FLOW_TEXTBOX_CURSOR)

    return panel


def _flow_column_boxes(
    inner_x1: int,
    inner_y1: int,
    inner_x2: int,
    inner_y2: int,
    *,
    column_gap: int = FLOW_COLUMN_GAP,
    col_models: float = FLOW_COL_MODELS,
    col_widgets: float = FLOW_COL_WIDGETS,
    col_paste: float | None = None,
    fixed_columns: bool = False,
) -> tuple[tuple[int, int, int, int], tuple[int, int, int, int], tuple[int, int, int, int], list[int]]:
    inner_w = inner_x2 - inner_x1
    gap = column_gap
    usable = inner_w - gap * 2
    w2 = int(usable * col_widgets)
    if fixed_columns and col_paste is not None:
        w1 = int(usable * col_models)
        w3 = int(usable * col_paste)
        drift = usable - w1 - w2 - w3
        w2 += drift
    elif col_paste is not None:
        w3 = int(usable * col_paste)
        w1 = usable - w2 - w3
    else:
        w1 = int(usable * col_models)
        w3 = usable - w1 - w2

    x = inner_x1
    col1 = (x, inner_y1, x + w1, inner_y2)
    x += w1 + gap
    col2 = (x, inner_y1, x + w2, inner_y2)
    x += w2 + gap
    col3 = (x, inner_y1, x + w3, inner_y2)
    dividers = [col1[2] + gap // 2, col2[2] + gap // 2]
    return col1, col2, col3, dividers


def _draw_vertical_stack(
    canvas: Image.Image,
    box: tuple[int, int, int, int],
    items: list[Image.Image],
    labels: list[str],
    *,
    label_font: ImageFont.FreeTypeFont | ImageFont.ImageFont,
    label_h: int,
    arrow_stem: int = FLOW_ARROW_STEM,
    arrow_gap: int = FLOW_ARROW_GAP,
    arrow_head: int = FLOW_ARROW_HEAD,
) -> None:
    x1, y1, x2, y2 = box
    col_w, col_h = x2 - x1, y2 - y1
    arrow_block = arrow_stem + arrow_gap * 2
    total_h = sum(item.height for item in items) + arrow_block * (len(items) - 1) + (label_h + FLOW_LABEL_GAP) * len(items)
    max_item_w = max(item.width for item in items)
    scale = min(col_w / max_item_w, col_h / total_h, 1.0)
    if scale < 0.999:
        items = [
            item.resize((max(1, int(item.width * scale)), max(1, int(item.height * scale))), Image.Resampling.LANCZOS)
            for item in items
        ]
        total_h = sum(item.height for item in items) + arrow_block * (len(items) - 1) + (label_h + FLOW_LABEL_GAP) * len(items)

    y = y1 + (col_h - total_h) // 2
    cx = x1 + col_w // 2
    draw = ImageDraw.Draw(canvas)
    for item, label in zip(items, labels):
        ox = cx - item.width // 2
        if item.mode == "RGBA":
            canvas.paste(item, (ox, y), item)
        else:
            canvas.paste(item, (ox, y))
        y += item.height + FLOW_LABEL_GAP
        lw = draw.textlength(label, font=label_font)
        draw.text((cx - lw / 2, y), label, fill=FLOW_LABEL_COLOR, font=label_font)
        y += label_h
        if label != labels[-1]:
            arrow_top = y + arrow_gap
            arrow_bottom = arrow_top + arrow_stem
            draw_down_arrow(draw, cx, arrow_top, arrow_bottom, head=arrow_head)
            y = arrow_bottom + arrow_gap


def draw_panel_box(canvas: Image.Image, box: tuple[int, int, int, int], radius: int = 24) -> None:
    x1, y1, x2, y2 = box
    panel = rounded_panel((x2 - x1, y2 - y1), radius=radius)
    canvas.paste(panel, (x1, y1), panel)


def draw_flow_headline(
    canvas: Image.Image,
    title: str,
    subtitle: str,
    *,
    canvas_width: int,
    scale: float = 1.0,
) -> None:
    draw = ImageDraw.Draw(canvas)
    if scale > 1.1:
        title_size = _li(38)
        sub_size = _li(19)
        title_y = _li(14)
        sub_y = _li(68)
    else:
        title_size = 46
        sub_size = 23
        title_y = 36
        sub_y = 92
    title_font = load_font(title_size, bold=True)
    sub_font = load_font(sub_size)
    tw = draw.textlength(title, font=title_font)
    draw.text(((canvas_width - tw) / 2, title_y), title, fill=TITLE_COLOR, font=title_font)
    sw = draw.textlength(subtitle, font=sub_font)
    draw.text(((canvas_width - sw) / 2, sub_y), subtitle, fill=SUB_COLOR, font=sub_font)


def draw_column_titles(
    canvas: Image.Image,
    boxes: tuple[tuple[int, int, int, int], tuple[int, int, int, int], tuple[int, int, int, int]],
    titles: tuple[str, str, str],
    *,
    top_y: int,
    font_size: int = 14,
) -> None:
    draw = ImageDraw.Draw(canvas)
    font = load_font(font_size, bold=True)
    for box, title in zip(boxes, titles):
        x1, _, x2, _ = box
        tw = draw.textlength(title, font=font)
        draw.text((x1 + (x2 - x1 - tw) / 2, top_y), title, fill=FLOW_LABEL_COLOR, font=font)


def compose_flow_diagram(
    *,
    canvas_size: tuple[int, int] = CANVAS,
    models_path: Path | None = None,
    headline: tuple[str, str] | None = None,
    column_titles: tuple[str, str, str] | None = None,
    paste_labels: tuple[str, str] | None = None,
) -> Image.Image:
    """Three columns: models setup, widget state flow, empty-to-pasted text field."""
    canvas = Image.new("RGB", canvas_size, BG)
    cw, ch = canvas_size
    margin_x = PANEL_MARGIN_X
    panel_top = 150 if headline else PANEL_TOP
    panel_bottom = ch - (32 if headline else (CANVAS[1] - PANEL_BOTTOM))
    inner_pad = FLOW_INNER_PAD
    arrow_stem = FLOW_ARROW_STEM
    arrow_gap = FLOW_ARROW_GAP
    arrow_head = FLOW_ARROW_HEAD
    column_gap = FLOW_COLUMN_GAP
    panel_box = (margin_x, panel_top, cw - margin_x, panel_bottom)
    draw_panel_box(canvas, panel_box)

    if headline:
        draw_flow_headline(canvas, headline[0], headline[1], canvas_width=cw)

    px1, py1, px2, py2 = panel_box
    inner_x1 = px1 + inner_pad
    inner_y1 = py1 + inner_pad + (18 if column_titles else 0)
    inner_x2 = px2 - inner_pad
    inner_y2 = py2 - inner_pad
    inner_h = inner_y2 - inner_y1

    models_box, widgets_box, paste_box, dividers = _flow_column_boxes(
        inner_x1,
        inner_y1,
        inner_x2,
        inner_y2,
        column_gap=column_gap,
    )

    if column_titles:
        draw_column_titles(canvas, (models_box, widgets_box, paste_box), column_titles, top_y=py1 + 10)

    models = Image.open(models_path or SCREENSHOTS["models"]).convert("RGBA")
    paste_centered(canvas, fit_image(models, models_box), models_box)

    label_font = load_font(16)
    label_h = int(label_font.size * 1.3)
    widgets = [prepare_widget(kind) for kind in FLOW_WIDGET_KINDS]
    wx1, wy1, wx2, wy2 = widgets_box
    widgets = _scale_widgets_to_fit(
        widgets,
        wx2 - wx1,
        wy2 - wy1,
        arrow_stem=arrow_stem,
        label_h=label_h + FLOW_LABEL_GAP,
        gap=arrow_gap,
    )

    arrow_block = arrow_stem + arrow_gap * 2
    stack_h = sum(w.height for w in widgets) + arrow_block * (len(widgets) - 1) + (label_h + FLOW_LABEL_GAP) * len(widgets)
    wy = wy1 + (wy2 - wy1 - stack_h) // 2
    wcx = wx1 + (wx2 - wx1) // 2
    draw = ImageDraw.Draw(canvas)
    for widget, label in zip(widgets, FLOW_LABELS):
        ox = wcx - widget.width // 2
        canvas.paste(widget, (ox, wy), widget)
        wy += widget.height + FLOW_LABEL_GAP
        lw = draw.textlength(label, font=label_font)
        draw.text((wcx - lw / 2, wy), label, fill=FLOW_LABEL_COLOR, font=label_font)
        wy += label_h
        if label != FLOW_LABELS[-1]:
            arrow_top = wy + arrow_gap
            arrow_bottom = arrow_top + arrow_stem
            draw_down_arrow(draw, wcx, arrow_top, arrow_bottom, head=arrow_head)
            wy = arrow_bottom + arrow_gap

    px1_box, py1_box, px2_box, py2_box = paste_box
    paste_w = px2_box - px1_box
    textbox_h = min(150, int(inner_h * 0.34))
    textbox_w = min(paste_w - 8, 240)
    empty_box = render_textbox_mock((textbox_w, textbox_h))
    filled_box = render_textbox_mock((textbox_w, textbox_h), text=FLOW_SAMPLE_TEXT)
    _draw_vertical_stack(
        canvas,
        paste_box,
        [empty_box, filled_box],
        list(paste_labels or FLOW_PASTE_LABELS),
        label_font=label_font,
        label_h=label_h,
        arrow_stem=arrow_stem,
        arrow_gap=arrow_gap,
        arrow_head=arrow_head,
    )

    divider_y1 = inner_y1 + 12
    divider_y2 = inner_y2 - 12
    for divider_x in dividers:
        draw.line((divider_x, divider_y1, divider_x, divider_y2), fill=FLOW_DIVIDER_COLOR, width=1)

    bridge_y = inner_y1 + inner_h // 2
    draw_right_arrow(draw, dividers[0] + 8, dividers[0] + column_gap - 8, bridge_y, head=arrow_head)
    draw_right_arrow(draw, dividers[1] + 8, dividers[1] + column_gap - 8, bridge_y, head=arrow_head)

    return canvas


def compose_flow_linkedin() -> Image.Image:
    """Content-sized LinkedIn layout — column widths fit assets, no wasted fractions."""
    li = _linkedin_scale()
    margin_x = _li(LINKEDIN_PANEL_MARGIN_X)
    panel_top = _li(108)
    panel_bottom_margin = _li(20)
    inner_pad = _li(LINKEDIN_PANEL_INNER_PAD)
    col_gap = _li(LINKEDIN_COL_GAP)
    col_pad = _li(LINKEDIN_COL_PAD)
    canvas_h = LINKEDIN_CANVAS_HEIGHT

    inner_y1 = panel_top + inner_pad + _li(18)
    inner_y2 = canvas_h - panel_bottom_margin - inner_pad
    inner_h = inner_y2 - inner_y1

    models_raw = Image.open(SCREENSHOTS["models"]).convert("RGBA")
    model_scale = min(1.0, (inner_h * 0.96) / models_raw.height)
    models_img = models_raw.resize(
        (max(1, int(models_raw.width * model_scale)), max(1, int(models_raw.height * model_scale))),
        Image.Resampling.LANCZOS,
    )

    label_font = load_font(_li(13))
    label_h = int(label_font.size * 1.3)
    arrow_stem = _li(14)
    arrow_gap = _li(6)
    arrow_head = _li(7)
    widgets = [prepare_widget(kind, scale_mult=li) for kind in FLOW_WIDGET_KINDS]
    widgets = _scale_widgets_to_fit(
        widgets,
        99999,
        inner_h,
        arrow_stem=arrow_stem,
        label_h=label_h + FLOW_LABEL_GAP,
        gap=arrow_gap,
    )
    widgets_w = max(w.width for w in widgets)

    text_font = _li(14)
    textbox_w = min(_li(200), max(_li(160), int(models_img.width * 0.58)))
    textbox_h = _li(96)
    empty_box = render_textbox_mock((textbox_w, textbox_h), font_size=text_font)
    filled_box = render_textbox_mock((textbox_w, textbox_h), text=FLOW_SAMPLE_TEXT, font_size=text_font)
    paste_w = max(empty_box.width, filled_box.width)

    col1_w = models_img.width + col_pad * 2
    col2_w = widgets_w + col_pad * 2
    col3_w = paste_w + col_pad * 2
    content_w = col1_w + col_gap + col2_w + col_gap + col3_w
    panel_w = content_w + inner_pad * 2
    canvas_w = panel_w + margin_x * 2

    canvas = Image.new("RGB", (canvas_w, canvas_h), BG)
    panel_box = (margin_x, panel_top, margin_x + panel_w, canvas_h - panel_bottom_margin)
    draw_panel_box(canvas, panel_box, radius=_li(24))
    draw_flow_headline(canvas, FLOW_HEADLINE_TITLE, FLOW_HEADLINE_SUBTITLE, canvas_width=canvas_w, scale=li)

    px1, py1, px2, py2 = panel_box
    x = px1 + inner_pad
    models_box = (x, inner_y1, x + col1_w, inner_y2)
    x += col1_w + col_gap
    widgets_box = (x, inner_y1, x + col2_w, inner_y2)
    x += col2_w + col_gap
    paste_box = (x, inner_y1, x + col3_w, inner_y2)
    dividers = [models_box[2] + col_gap // 2, widgets_box[2] + col_gap // 2]

    draw_column_titles(
        canvas,
        (models_box, widgets_box, paste_box),
        LINKEDIN_COLUMN_TITLES,
        top_y=py1 + _li(10),
        font_size=_li(14),
    )

    paste_centered(canvas, models_img, models_box)

    wx1, wy1, wx2, wy2 = widgets_box
    arrow_block = arrow_stem + arrow_gap * 2
    stack_h = sum(w.height for w in widgets) + arrow_block * (len(widgets) - 1) + (label_h + FLOW_LABEL_GAP) * len(widgets)
    wy = wy1 + (wy2 - wy1 - stack_h) // 2
    wcx = wx1 + (wx2 - wx1) // 2
    draw = ImageDraw.Draw(canvas)
    for widget, label in zip(widgets, FLOW_LABELS):
        ox = wcx - widget.width // 2
        canvas.paste(widget, (ox, wy), widget)
        wy += widget.height + FLOW_LABEL_GAP
        lw = draw.textlength(label, font=label_font)
        draw.text((wcx - lw / 2, wy), label, fill=FLOW_LABEL_COLOR, font=label_font)
        wy += label_h
        if label != FLOW_LABELS[-1]:
            arrow_top = wy + arrow_gap
            arrow_bottom = arrow_top + arrow_stem
            draw_down_arrow(draw, wcx, arrow_top, arrow_bottom, head=arrow_head, width=max(2, _li(2)))
            wy = arrow_bottom + arrow_gap

    _draw_vertical_stack(
        canvas,
        paste_box,
        [empty_box, filled_box],
        list(LINKEDIN_PASTE_LABELS),
        label_font=label_font,
        label_h=label_h,
        arrow_stem=arrow_stem,
        arrow_gap=arrow_gap,
        arrow_head=arrow_head,
    )

    divider_y1 = inner_y1 + _li(12)
    divider_y2 = inner_y2 - _li(12)
    for divider_x in dividers:
        draw.line((divider_x, divider_y1, divider_x, divider_y2), fill=FLOW_DIVIDER_COLOR, width=max(1, _li(1)))

    bridge_y = inner_y1 + inner_h // 2
    bridge_inset = _li(6)
    draw_right_arrow(draw, dividers[0] + bridge_inset, dividers[0] + col_gap - bridge_inset, bridge_y, head=arrow_head, width=max(2, _li(2)))
    draw_right_arrow(draw, dividers[1] + bridge_inset, dividers[1] + col_gap - bridge_inset, bridge_y, head=arrow_head, width=max(2, _li(2)))

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

    flow_path = OUTPUT / "showcase-flow.png"
    compose_flow_diagram().save(flow_path, optimize=True)
    print(f"Wrote {flow_path}")

    linkedin_flow_path = OUTPUT / "showcase-flow-linkedin.png"
    compose_flow_linkedin().save(linkedin_flow_path, optimize=True)
    print(f"Wrote {linkedin_flow_path}")

    preview = OUTPUT / "demo-preview.gif"
    if preview.exists():
        preview.unlink()
        print(f"Removed obsolete {preview}")

    print(f"Done — {len(slides)} showcase slides + demo animations in {OUTPUT}")


if __name__ == "__main__":
    main()
