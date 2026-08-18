#!/usr/bin/env bash
# macOS `make dev`: build the debug binary, wrap it in a real .app bundle, and
# launch it through LaunchServices.
#
# Why the bundle: macOS attributes TCC (Accessibility) to the *responsible
# process*. A bare executable started from a shell inherits responsibility from
# the terminal app, so `cargo tauri dev` can never hold its own Accessibility
# grant — you would have to grant it to Terminal/iTerm/Cursor instead. An .app
# opened via LaunchServices is responsible for itself, so the grant lands on
# "SpeakType Dev" and persists across rebuilds (stable Developer ID + bundle id).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=load-secrets.sh
source "$SCRIPT_DIR/load-secrets.sh"
speaktype_load_rust_env
speaktype_prefer_system_xattr

TAURI_DIR="$ROOT/apps/widget-rust/src-tauri"
TARGET_DIR="${CARGO_TARGET_DIR:-$TAURI_DIR/target}/debug"
BUNDLE="$TARGET_DIR/SpeakType Dev.app"
CONTENTS="$BUNDLE/Contents"
ENTITLEMENTS="$TAURI_DIR/Entitlements.plist"
LOG_FILE="$HOME/Library/Application Support/speaktype/speaktype.log"

BUNDLE_ID="com.speaktype.widget.dev"
APP_NAME="SpeakType Dev"

cd "$ROOT"

# Signing vars from make/local.mk when present (Make KEY = value / KEY ?= value).
if [[ -f make/local.mk ]]; then
  while IFS= read -r line || [[ -n "$line" ]]; do
    case "$line" in
      \#*|"") continue ;;
    esac
    if [[ "$line" =~ ^(export[[:space:]]+)?(APPLE_DEVELOPER_ID|APPLE_TEAM_ID)[[:space:]]*(\?\=|\=)[[:space:]]*(.*)$ ]]; then
      key="${BASH_REMATCH[2]}"
      val="${BASH_REMATCH[4]}"
      val="${val%\"}"; val="${val#\"}"
      val="${val%\'}"; val="${val#\'}"
      export "$key=$val"
    fi
  done < make/local.mk
fi

resolve_identity() {
  if [[ -n "${APPLE_DEVELOPER_ID:-}" ]]; then
    printf '%s\n' "$APPLE_DEVELOPER_ID"
    return
  fi
  security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*"\(Developer ID Application: .*\)"/\1/p' \
    | head -1
}

write_info_plist() {
  cat > "$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>English</string>
	<key>CFBundleDisplayName</key>
	<string>$APP_NAME</string>
	<key>CFBundleExecutable</key>
	<string>speaktype</string>
	<key>CFBundleIconFile</key>
	<string>SpeakType.icns</string>
	<key>CFBundleIdentifier</key>
	<string>$BUNDLE_ID</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>$APP_NAME</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>0.1.0</string>
	<key>CFBundleVersion</key>
	<string>0.1.0</string>
	<key>LSMinimumSystemVersion</key>
	<string>10.13</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>NSMicrophoneUsageDescription</key>
	<string>SpeakType needs microphone access for voice-to-text transcription.</string>
</dict>
</plist>
PLIST
}

assemble_bundle() {
  mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"
  write_info_plist

  cp -f "$TARGET_DIR/speaktype" "$CONTENTS/MacOS/speaktype"

  # Sidecar must sit next to the executable for Tauri's sidecar resolution.
  local sidecar="$TARGET_DIR/whisper-server"
  [[ -f "$sidecar" ]] || sidecar="$TAURI_DIR/binaries/whisper-server-aarch64-apple-darwin"
  if [[ -f "$sidecar" ]]; then
    cp -f "$sidecar" "$CONTENTS/MacOS/whisper-server"
  else
    echo "[dev] warning: whisper-server sidecar not found — local transcription will fail"
  fi

  local icns="$TAURI_DIR/icons/icon.icns"
  [[ -f "$icns" ]] && cp -f "$icns" "$CONTENTS/Resources/SpeakType.icns"

  # Stale signature would make codesign refuse to re-sign cleanly.
  rm -rf "$CONTENTS/_CodeSignature"
}

sign_bundle() {
  local identity err
  identity="$(resolve_identity || true)"

  if [[ -z "$identity" ]]; then
    echo "[dev] no Developer ID found — signing ad-hoc (Accessibility must be re-granted after each rebuild)"
    identity="-"
  fi

  # Inner executables first, then the bundle itself.
  if [[ -f "$CONTENTS/MacOS/whisper-server" ]]; then
    codesign --force --sign "$identity" --timestamp=none \
      "$CONTENTS/MacOS/whisper-server" >/dev/null 2>&1 || true
  fi

  if err="$(codesign --force --sign "$identity" \
    --identifier "$BUNDLE_ID" \
    --options runtime \
    --timestamp=none \
    --entitlements "$ENTITLEMENTS" \
    "$BUNDLE" 2>&1)"; then
    if [[ "$identity" == "-" ]]; then
      echo "[dev] signed $APP_NAME (ad-hoc)"
    else
      echo "[dev] signed $APP_NAME ($identity)"
    fi
  else
    echo "[dev] codesign failed: ${err//$'\n'/ }"
    return 1
  fi
}

quit_running_instances() {
  # Kill the app and the sidecar it spawned — otherwise each rebuild orphans a
  # whisper-server (they outlive their parent).
  pkill -f "$CONTENTS/MacOS/speaktype" 2>/dev/null || true
  pkill -f "$CONTENTS/MacOS/whisper-server" 2>/dev/null || true
  pkill -f "$TARGET_DIR/speaktype" 2>/dev/null || true
  sleep 0.5
}

app_is_running() {
  pgrep -f "$CONTENTS/MacOS/speaktype" >/dev/null 2>&1
}

binary_digest() {
  [[ -f "$1" ]] || return 1
  shasum -a 256 "$1" | awk '{print $1}'
}

rebuild_and_launch() {
  local bundled_digest built_digest
  bundled_digest="$(binary_digest "$CONTENTS/MacOS/speaktype" 2>/dev/null || true)"

  echo "[dev] Building…"
  (cd "$TAURI_DIR" && cargo build) || return 1

  if ! built_digest="$(binary_digest "$TARGET_DIR/speaktype")"; then
    echo "[dev] build produced no binary at $TARGET_DIR/speaktype"
    return 1
  fi

  # Re-signing is what jeopardises the Accessibility grant, so skip the whole
  # sign/relaunch cycle when the build reproduced the binary that is already
  # bundled and running — a UI-only edit that compiles to the same bytes has
  # nothing to ship.
  if [[ -n "$bundled_digest" && "$built_digest" == "$bundled_digest" \
        && -d "$CONTENTS/_CodeSignature" ]] && app_is_running; then
    echo "[dev] binary unchanged — kept the running instance (no re-sign, no relaunch)"
    return 0
  fi

  assemble_bundle
  sign_bundle || return 1
  quit_running_instances

  open "$BUNDLE"
  echo "[dev] Launched $APP_NAME"
}

# Sources that should trigger a rebuild. Rust sources are watched because the
# frontend is embedded at compile time (custom-protocol), so UI edits also need
# a rebuild to take effect.
WATCH_PATHS=(
  "$TAURI_DIR/src"
  "$TAURI_DIR/Cargo.toml"
  "$TAURI_DIR/tauri.conf.json"
  "$ROOT/packages/widget-ui"
)

POLL_INTERVAL=1
# Consecutive equal samples required before a change is considered settled.
DEBOUNCE_SAMPLES=3

# One `path|mtime|size` line per watched source file. Directory entries, `total`
# block counts and editor droppings are deliberately absent: they move without a
# source edit, and the previous `ls -lTR` fingerprint hashed all of them.
file_manifest() {
  {
    find "${WATCH_PATHS[@]}" \
      \( -name '.git' -o -name 'target' -o -name 'gen' -o -name 'node_modules' -o -name 'dist' \) -prune -o \
      -type f \
      \( -name '*.rs' -o -name '*.toml' -o -name '*.json' \
         -o -name '*.html' -o -name '*.css' -o -name '*.js' \) \
      ! -name '.DS_Store' ! -name '*.swp' ! -name '*.swo' ! -name '*~' \
      -exec stat -f '%N|%m|%z' {} \; 2>/dev/null || true
  } | LC_ALL=C sort
}

fingerprint() {
  file_manifest | shasum -a 256 | awk '{print $1}'
}

# Paths present in one manifest but not the other, relative to the repo root.
changed_paths() {
  { diff <(printf '%s\n' "$1") <(printf '%s\n' "$2") || true; } \
    | sed -n 's/^[<>] //p' \
    | cut -d'|' -f1 \
    | LC_ALL=C sort -u \
    | sed "s|^$ROOT/||"
}

# Block until the fingerprint stops moving, so a burst of saves collapses into a
# single rebuild instead of one rebuild per save.
settle() {
  local fp="$1" stable=0 next
  while (( stable < DEBOUNCE_SAMPLES )); do
    sleep "$POLL_INTERVAL"
    next="$(fingerprint)"
    if [[ "$next" == "$fp" ]]; then
      stable=$(( stable + 1 ))
    else
      fp="$next"
      stable=0
    fi
  done
  printf '%s\n' "$fp"
}

# An empty manifest would silently pin the fingerprint and the watcher would
# never fire again, so refuse to start rather than pretend to watch.
if [[ -z "$(file_manifest)" ]]; then
  echo "[dev] error: no watched source files matched under:"
  printf '[dev]   %s\n' "${WATCH_PATHS[@]}"
  exit 1
fi

rm -rf "$TARGET_DIR/SpeakTypeDev.app"
rebuild_and_launch || exit 1

echo
echo "[dev] Accessibility: System Settings → Privacy & Security → Accessibility"
echo "[dev]   Enable \"$APP_NAME\" — it is its own app, so the grant sticks across rebuilds."
echo "[dev]   Bundle: $BUNDLE"
echo

# LaunchServices detaches stdout, so stream the app's log file instead.
mkdir -p "$(dirname "$LOG_FILE")"
touch "$LOG_FILE"
tail -n 0 -f "$LOG_FILE" &
TAIL_PID=$!

cleanup() {
  echo
  echo "[dev] Stopping ${APP_NAME}…"
  kill "$TAIL_PID" 2>/dev/null || true
  pkill -f "$CONTENTS/MacOS/speaktype" 2>/dev/null || true
  pkill -f "$CONTENTS/MacOS/whisper-server" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

echo "[dev] Watching for changes (Ctrl+C to quit)"
echo

last_fp="$(fingerprint)"
last_manifest="$(file_manifest)"
while true; do
  sleep "$POLL_INTERVAL"
  fp="$(fingerprint)"
  [[ "$fp" == "$last_fp" ]] && continue

  settle "$fp" >/dev/null
  pre_manifest="$(file_manifest)"

  echo
  echo "[dev] Change detected — rebuilding…"
  changed_paths "$last_manifest" "$pre_manifest" | sed 's/^/[dev]   /'
  rebuild_and_launch || echo "[dev] rebuild failed — fix the error and save again"

  # Recompute after the build in case it touched watched files.
  last_manifest="$(file_manifest)"
  last_fp="$(fingerprint)"

  if [[ "$last_manifest" != "$pre_manifest" ]]; then
    echo "[dev] watched files changed during the build (external edit, or the build self-triggering):"
    changed_paths "$pre_manifest" "$last_manifest" | sed 's/^/[dev]   /'
  fi
done
