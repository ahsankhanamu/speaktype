#!/usr/bin/env bash
# create-dmg.sh — SpeakType compact installer DMG (520×400, 112px icons)
#
# Headless: icon layout is written by dmgbuild, so no Finder automation
# (Apple Events) permission is required.
#
# Usage: create-dmg.sh <app-bundle> <output-dmg> [volume-name]

set -euo pipefail

APP_BUNDLE="${1:?app bundle path required}"
DMG_PATH="${2:?output dmg path required}"
VOLUME_NAME="${3:-SpeakType}"

if [ ! -d "$APP_BUNDLE" ]; then
    echo "ERROR: app bundle not found: $APP_BUNDLE" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BG_DIR="$SCRIPT_DIR/dmg-resources"
BG_PNG="$BG_DIR/background.png"

if [ ! -f "$BG_PNG" ] \
    || [ "$SCRIPT_DIR/compose-dmg-background.py" -nt "$BG_PNG" ] \
    || [ "$SCRIPT_DIR/dmg_layout.py" -nt "$BG_PNG" ]; then
    "$SCRIPT_DIR/render-dmg-background.sh"
fi

# Prefer the project venv so dmgbuild is available; fall back to PATH python3.
PY="${PROJECT_ROOT}/.venv/bin/python3"
if [ ! -x "$PY" ]; then
    PY="$(command -v python3 || true)"
fi
if [ -z "$PY" ] || ! "$PY" -c "import dmgbuild" >/dev/null 2>&1; then
    echo "ERROR: dmgbuild is not installed." >&2
    echo "  Run:  make install-python   (or:  $PY -m pip install dmgbuild)" >&2
    exit 1
fi

mkdir -p "$(dirname "$DMG_PATH")"
rm -f "$DMG_PATH"

"$PY" "$SCRIPT_DIR/build_dmg.py" "$APP_BUNDLE" "$DMG_PATH" "$VOLUME_NAME"
