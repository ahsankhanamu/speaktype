#!/usr/bin/env python3
"""Headless DMG builder for the SpeakType installer.

Replaces the old osascript/Finder layout so `make build` never needs
Automation (Apple Events) permission to control Finder.

Usage: build_dmg.py <app-bundle> <output-dmg> [volume-name]
"""

import sys
from pathlib import Path

import dmgbuild


def main() -> None:
    if len(sys.argv) < 3:
        print(__doc__ or "usage: build_dmg.py <app-bundle> <output-dmg> [volume-name]", file=sys.stderr)
        sys.exit(2)

    app_bundle = sys.argv[1]
    dmg_path = sys.argv[2]
    volume_name = sys.argv[3] if len(sys.argv) > 3 else "SpeakType"

    script_dir = Path(__file__).resolve().parent
    sys.path.insert(0, str(script_dir))
    # pyrefly: ignore [missing-import]
    from dmg_layout import (
    APP_W,
    APP_X,
    APPS_X,
    ICON_Y,
    WIN_H,
    WIN_H_DISPLAY,
    WIN_W,
    WIN_W_DISPLAY,
)

    background = script_dir / "dmg-resources" / "background.png"
    if not background.exists():
        print(f"ERROR: background image missing: {background}", file=sys.stderr)
        sys.exit(1)

    app_name = Path(app_bundle).name

    settings = {
        "volume_name": volume_name,
        "format": "UDZO",
        "compression_level": 9,
        "files": [str(app_bundle)],
        "symlinks": {"Applications": "/Applications"},
        "background": str(background),
        "icon_size": APP_W,
        "window_rect": ((200, 120), (WIN_W_DISPLAY, WIN_H_DISPLAY)),
        "icon_locations": {
            app_name: (APP_X, ICON_Y),
            "Applications": (APPS_X, ICON_Y),
        },
    }

    print("→ Creating DMG (headless, no Finder automation)...")
    dmgbuild.build_dmg(str(dmg_path), volume_name, settings=settings)
    print(f"→ DMG created at {dmg_path}")


if __name__ == "__main__":
    main()
