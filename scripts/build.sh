#!/usr/bin/env bash
#
# build.sh - Build, sign, and package SpeakType into a real, shippable .dmg
#
# One command, start to finish:
#   clean → whisper.cpp sidecar → Tauri app → Developer ID sign → DMG → notarize
#
# Output: dist/SpeakType_<version>_aarch64.dmg  (Developer ID signed)
#
# Notarization runs automatically if the keychain profile exists
# (set it up once with `make notary-setup`); otherwise it is skipped with a warning.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_DIR="$PROJECT_ROOT/apps/widget-rust"

# shellcheck source=load-secrets.sh
source "$SCRIPT_DIR/load-secrets.sh"
speaktype_load_local_config "$PROJECT_ROOT/make/local.mk"

: "${APPLE_DEVELOPER_ID:?Set APPLE_DEVELOPER_ID (export it or add to make/local.mk — see make/config.example)}"
: "${APPLE_TEAM_ID:?Set APPLE_TEAM_ID (export it or add to make/local.mk — see make/config.example)}"
NOTARY_PROFILE="${NOTARY_PROFILE:-SpeakType}"

APP_BUNDLE="$APP_DIR/src-tauri/target/release/bundle/macos/SpeakType.app"
ENTITLEMENTS="$APP_DIR/src-tauri/Entitlements.plist"
MACOS_DIR="$APP_BUNDLE/Contents/MacOS"
MAIN_BINARY="speaktype"

echo "═══════════════════════════════════════════════════════════════"
echo "  Building SpeakType"
echo "═══════════════════════════════════════════════════════════════"

# ── 1. Clean ──────────────────────────────────────────────────────────────────
echo "→ Cleaning dist/ and previous bundle..."
rm -rf "$PROJECT_ROOT/dist"
mkdir -p "$PROJECT_ROOT/dist"
rm -rf "$APP_DIR/src-tauri/target/release/bundle"

# ── 2. whisper.cpp sidecar ──────────────────────────────────────────────────────
echo "→ Building whisper.cpp sidecar..."
SIDECAR_DIR="$APP_DIR/src-tauri/scripts/build/whisper.cpp/build"
mkdir -p "$SIDECAR_DIR"
cd "$SIDECAR_DIR"
cmake -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DGGML_METAL=ON ..
cmake --build . --target whisper-server -j"$(sysctl -n hw.ncpu)"
mkdir -p "$APP_DIR/src-tauri/binaries"
cp bin/whisper-server "$APP_DIR/src-tauri/binaries/whisper-server-aarch64-apple-darwin"

# ── 3. Tauri app ────────────────────────────────────────────────────────────────
echo "→ Building Tauri app..."
cd "$APP_DIR"
cargo tauri build

if [ ! -d "$APP_BUNDLE" ]; then
    echo "ERROR: .app bundle not found at $APP_BUNDLE — Tauri build failed."
    exit 1
fi

# ── 4. Developer ID signing ─────────────────────────────────────────────────────
# Apple's timestamp server (timestamp.apple.com) is occasionally slow/flaky and
# makes codesign fail with "A timestamp was expected but was not found." Retry so
# a single transient TSA hiccup doesn't waste an entire build.
codesign_retry() {
    local target="$1"; shift
    for attempt in 1 2 3 4 5; do
        if codesign --force --timestamp "$@" -s "$APPLE_DEVELOPER_ID" "$target"; then
            return 0
        fi
        echo "     timestamp attempt $attempt failed, retrying in 3s..."
        sleep 3
    done
    echo "ERROR: codesign failed for $target after 5 attempts (timestamp server unreachable?)."
    return 1
}

echo "→ Signing with: $APPLE_DEVELOPER_ID"

if [ -f "$MACOS_DIR/whisper-server" ]; then
    echo "   • sidecar (whisper-server)"
    codesign_retry "$MACOS_DIR/whisper-server"
fi

echo "   • main binary ($MAIN_BINARY)"
codesign_retry "$MACOS_DIR/$MAIN_BINARY" --options=runtime --entitlements "$ENTITLEMENTS"

echo "   • .app bundle"
codesign_retry "$APP_BUNDLE" --options=runtime --entitlements "$ENTITLEMENTS"

# ── 5. DMG → dist/ ───────────────────────────────────────────────────────────────
VERSION="$(grep -m1 '"version"' "$APP_DIR/src-tauri/tauri.conf.json" | sed -E 's/.*"version" *: *"([^"]+)".*/\1/')"
DMG_PATH="$PROJECT_ROOT/dist/SpeakType_${VERSION}_aarch64.dmg"

echo "→ Creating DMG at $DMG_PATH..."
hdiutil detach "/Volumes/SpeakType" 2>/dev/null || true
rm -f "$DMG_PATH"

TMPDIR="$(mktemp -d)"
cp -R "$APP_BUNDLE" "$TMPDIR/SpeakType.app"
ln -s /Applications "$TMPDIR/Applications"
hdiutil create -volname "SpeakType" -srcfolder "$TMPDIR" -ov -format UDZO -imagekey zlib-level=9 "$DMG_PATH"
rm -rf "$TMPDIR"

echo "→ Signing DMG..."
codesign_retry "$DMG_PATH"

# ── 6. Notarize (if credentials are set up) ─────────────────────────────────────
if xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1; then
    echo "→ Notarizing (this can take a few minutes)..."
    xcrun notarytool submit "$DMG_PATH" --keychain-profile "$NOTARY_PROFILE" --team-id "$APPLE_TEAM_ID" --wait
    echo "→ Stapling ticket..."
    xcrun stapler staple "$DMG_PATH"
    echo "   ✓ Notarized & stapled"
else
    echo "   ⚠ Skipping notarization — no '$NOTARY_PROFILE' keychain profile."
    echo "     Run 'make notary-setup' once to enable it."
fi

echo "═══════════════════════════════════════════════════════════════"
echo "  ✓ Done:  $DMG_PATH"
echo "═══════════════════════════════════════════════════════════════"
exit 0
