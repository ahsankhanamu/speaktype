#!/usr/bin/env bash
# create-dmg.sh — WhatsApp-style compact installer DMG (660×400, 128px icons)
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
BG_DIR="$SCRIPT_DIR/dmg-resources"
BG_PNG="$BG_DIR/background.png"
RW_DMG="${DMG_PATH%.dmg}_rw.dmg"
MOUNT="/Volumes/$VOLUME_NAME"

if [ ! -f "$BG_PNG" ] \
    || [ "$SCRIPT_DIR/compose-dmg-background.py" -nt "$BG_PNG" ] \
    || [ "$SCRIPT_DIR/dmg_layout.py" -nt "$BG_PNG" ]; then
    "$SCRIPT_DIR/render-dmg-background.sh"
fi

cleanup() {
    hdiutil detach "$MOUNT" -force >/dev/null 2>&1 || true
    rm -f "$RW_DMG"
}
trap cleanup EXIT

hdiutil detach "$MOUNT" -force >/dev/null 2>&1 || true
rm -f "$DMG_PATH" "$RW_DMG"

echo "→ Preparing DMG layout..."
hdiutil create -size 320m -volname "$VOLUME_NAME" -fs HFS+ -ov "$RW_DMG" >/dev/null
hdiutil attach "$RW_DMG" -readwrite -noverify -noautoopen -mountpoint "$MOUNT" >/dev/null

cp -R "$APP_BUNDLE" "$MOUNT/SpeakType.app"
ln -sf /Applications "$MOUNT/Applications"
mkdir -p "$MOUNT/.background"
cp "$BG_PNG" "$MOUNT/.background/background.png"
SetFile -a V "$MOUNT/.background" 2>/dev/null || true

read -r APP_X ICON_Y APPS_X WIN_W WIN_H ICON_SIZE <<< "$(python3 -c "import sys; sys.path.insert(0, '$SCRIPT_DIR'); from dmg_layout import APP_X, APPS_X, ICON_SIZE, ICON_Y, WIN_H, WIN_W; print(APP_X, ICON_Y, APPS_X, WIN_W, WIN_H, ICON_SIZE)")"
osascript "$SCRIPT_DIR/dmg-layout.applescript" "$VOLUME_NAME" "$APP_X" "$ICON_Y" "$APPS_X" "$WIN_W" "$WIN_H" "$ICON_SIZE"

sync
hdiutil detach "$MOUNT" >/dev/null
trap - EXIT

hdiutil convert "$RW_DMG" -format UDZO -imagekey zlib-level=9 -o "$DMG_PATH" >/dev/null
rm -f "$RW_DMG"

echo "→ DMG created at $DMG_PATH"
