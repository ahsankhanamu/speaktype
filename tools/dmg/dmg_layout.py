"""Shared SpeakType DMG installer layout.

Design (v3, reimagined):
    A plain shade of gray as the window background, with a single slate arrow
    SVG baked in as the guide, pointing from the app over to the Applications
    folder.  Three items share the row — app icon, arrow, folder icon — and
    every gap is one equal space unit:

        GAP = (WIN_W - APP_W - ARROW_W - FOLDER_W) / 4
        APP_X   = GAP
        ARROW_X = GAP + APP_W + GAP
        APPS_X  = GAP + APP_W + GAP + ARROW_W + GAP

    i.e.  gap · app · gap · arrow · gap · folder · gap  spans width WIN_W.
    The arrow thus lands exactly on the window midline; nothing reads larger
    than the icons around it.  Everything else is reproducible by hand:
    set the view to icon mode, pick a shade of gray as the background color,
    and drag the two icons onto the canvas.

    All coordinates are logical points.  SCALE = 1 keeps the bitmap @1x so
    Finder renders it 1 px = 1 pt regardless of display DPI.
"""

# --- window & background canvas (single source: WIN_W drives both) ---
WIN_W = 660  # shared width (Tauri/community standard DMG window)
WIN_W_DISPLAY = WIN_W
WIN_H = 420
WIN_H_DISPLAY = WIN_H + 16  # 436 — measured Finder window frame (win 660x436)
CHROME_TOP = 32  # measured via AX: content (AXScrollArea) starts 32pt below the top
CONTENT_H = WIN_H_DISPLAY - CHROME_TOP  # 404 — the Finder content/view area the bg fills
SCALE = 1  # 1x background: 1 px = 1 pt, immune to Finder DPI handling
BG_W = WIN_W * SCALE
BG_H = CONTENT_H * SCALE  # bg must exactly fit the viewport: no scroll, no gap
BG_DPI = 72 * SCALE

# --- item sizes (logical points) ---
APP_W = 112  # app icon slot
ARROW_W = 56  # arrow icon width
FOLDER_W = 112  # Applications folder slot
ARROW_H = round(ARROW_W * (41 / 78))  # arrow.svg viewBox aspect (height/width)

# --- horizontal: one equal space unit on every side of every item ---
GAP = (WIN_W - APP_W - ARROW_W - FOLDER_W) / 4
APP_X = round(GAP)
ARROW_X = round(2 * GAP + APP_W)
APPS_X = round(3 * GAP + APP_W + ARROW_W) + 16  # folder sits a little right of its gap slot

# --- vertical centering ---
# Finder draws its label inside the bottom of the icon slot, so the visible
# icon art sits ~12pt above the slot center.  Offsetting the slot center by
# +12pt puts the ART on the true content-area midline.
ICON_CENTER_Y = CONTENT_H // 2 + 12
ICON_Y = ICON_CENTER_Y - APP_W // 2
ARROW_Y = ICON_CENTER_Y - ARROW_H // 2 - 3  # ink sits ~3pt low in its box; nudge up