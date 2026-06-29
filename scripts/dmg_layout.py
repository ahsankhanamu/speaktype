"""Shared DMG installer layout (660×400pt window, 128px icons)."""

WIN_W = 600
WIN_H = 460  # Finder window bounds (includes title bar)
TITLE_BAR_H = 20  # background fills the content area below this
CONTENT_H = WIN_H - TITLE_BAR_H
ICON_SIZE = 128
SCALE = 2  # background PNG is @2x logical points

# Applications alias: blue folder graphic vs Finder 128px slot.
APPS_VISUAL_WIDTH = 117
APPS_VISUAL_TRAILING = 11  # graphic right edge sits this far right of slot origin

APPS_ALIAS_LEADING = APPS_VISUAL_WIDTH - APPS_VISUAL_TRAILING

# Equal visual margins: alias trailing offset pulls the folder left of the slot edge.
ICON_MARGIN = ICON_SIZE - APPS_VISUAL_TRAILING
ICON_LABEL_H = 16  # Finder label height below 128px icons
ICON_Y = round(CONTENT_H * 170 / 400)  # icon coords are relative to content area
ICON_CENTER_Y = ICON_Y + ICON_SIZE // 2
ARROW_Y_OFFSET = 52  # pt upward from icon center; increase if arrow still looks low

APP_X = ICON_MARGIN
APPS_X = WIN_W - ICON_SIZE

# Visual bounds for arrow placement (1x points).
APP_SLOT_RIGHT = APP_X + ICON_SIZE
APPS_VISUAL_LEFT = APPS_X - APPS_ALIAS_LEADING
ICON_GAP = APPS_VISUAL_LEFT - APP_SLOT_RIGHT

# Arrow render width: proportional to the visual gap (calibrated at 260pt gap → 320px).
_REF_GAP = 260
_REF_ARROW_W = 320
ARROW_FILL = _REF_ARROW_W / (_REF_GAP * SCALE)
ARROW_W = round(ICON_GAP * SCALE * ARROW_FILL)

BG_W = WIN_W * SCALE
BG_H = CONTENT_H * SCALE
BG_DPI = 72 * SCALE
