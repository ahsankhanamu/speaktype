#!/usr/bin/env bash
# Render background.png for the installer DMG (520×380 content @2x).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VENV_PYTHON="$SCRIPT_DIR/../.venv/bin/python3"
if [ -x "$VENV_PYTHON" ]; then
  PYTHON="$VENV_PYTHON"
else
  PYTHON=python3
fi

if ! command -v rsvg-convert >/dev/null 2>&1; then
  echo "ERROR: rsvg-convert required (brew install librsvg)" >&2
  exit 1
fi

if ! "$PYTHON" -c "from PIL import Image" >/dev/null 2>&1; then
  echo "ERROR: Pillow required for the active Python ($("$PYTHON" --version 2>&1))" >&2
  echo "  $PYTHON -m pip install pillow" >&2
  exit 1
fi

"$PYTHON" "$SCRIPT_DIR/compose-dmg-background.py"

# Normalize DPI so Finder displays WIN_W×WIN_H correctly on Retina (@2x → 144dpi).
DPI="$("$PYTHON" -c "import sys; sys.path.insert(0, '$SCRIPT_DIR'); from dmg_layout import BG_DPI; print(BG_DPI)")"
sips -s dpiWidth "$DPI" -s dpiHeight "$DPI" "$SCRIPT_DIR/dmg-resources/background.png" >/dev/null

echo "→ Rendered $SCRIPT_DIR/dmg-resources/background.png"
