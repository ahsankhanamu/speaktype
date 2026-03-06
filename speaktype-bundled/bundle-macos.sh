#!/usr/bin/env bash
#
# bundle-macos.sh — Prepare embedded resources for the self-contained SpeakType .dmg.
#
# Downloads python-build-standalone, installs server dependencies, downloads
# the base Whisper model, and copies the server script into resources/.
#
# Resilient: uses curl -C for resumable downloads, retries on failure,
# validates downloads before proceeding, and skips already-completed steps.
#
# Usage:
#   ./speaktype-bundled/bundle-macos.sh          # defaults to medium model
#   ./speaktype-bundled/bundle-macos.sh small    # bundle with small model instead
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESOURCES="$SCRIPT_DIR/src-tauri/resources"
CACHE_DIR="$PROJECT_ROOT/.cache"

MODEL_SIZE="${1:-medium}"
PYTHON_VERSION="3.12.8"
PYTHON_BUILD_TAG="20241219"
MAX_RETRIES=3

# Detect architecture
ARCH="$(uname -m)"
if [ "$ARCH" = "arm64" ]; then
    PYTHON_TRIPLE="aarch64-apple-darwin"
elif [ "$ARCH" = "x86_64" ]; then
    PYTHON_TRIPLE="x86_64-apple-darwin"
else
    echo "ERROR: Unsupported architecture: $ARCH" >&2
    exit 1
fi

PYTHON_TARBALL="cpython-${PYTHON_VERSION}+${PYTHON_BUILD_TAG}-${PYTHON_TRIPLE}-install_only_stripped.tar.gz"
PYTHON_URL="https://github.com/indygreg/python-build-standalone/releases/download/${PYTHON_BUILD_TAG}/${PYTHON_TARBALL}"

# ─── Helper: resumable download with retries ──────────────────────────────────
download_with_retry() {
    local url="$1"
    local dest="$2"
    local description="$3"

    # Already downloaded and valid
    if [ -f "$dest" ] && [ ! -f "${dest}.partial" ]; then
        echo "  Using cached: $dest"
        return 0
    fi

    # Move partial file into place for resume
    local partial="${dest}.partial"
    if [ -f "$partial" ]; then
        echo "  Resuming previous partial download..."
        mv "$partial" "$dest"
    fi

    for attempt in $(seq 1 $MAX_RETRIES); do
        echo "  Downloading $description (attempt $attempt/$MAX_RETRIES)..."
        # -C - = resume from where it left off; -L = follow redirects
        if curl -C - -L --progress-bar --fail -o "$dest" "$url"; then
            # Verify the file isn't truncated (at least 1MB for any of our downloads)
            local size
            size=$(stat -f%z "$dest" 2>/dev/null || stat -c%s "$dest" 2>/dev/null || echo "0")
            if [ "$size" -gt 1048576 ]; then
                echo "  Download complete: $(du -h "$dest" | cut -f1)"
                return 0
            else
                echo "  WARNING: Download too small (${size} bytes), retrying..."
                rm -f "$dest"
            fi
        else
            echo "  WARNING: Download failed on attempt $attempt"
            # Keep the partial file for resume on next attempt
            if [ -f "$dest" ]; then
                cp "$dest" "$partial"
            fi
        fi

        if [ "$attempt" -lt "$MAX_RETRIES" ]; then
            local wait=$((attempt * 5))
            echo "  Retrying in ${wait}s..."
            sleep "$wait"
        fi
    done

    echo "ERROR: Failed to download $description after $MAX_RETRIES attempts" >&2
    echo "  URL: $url" >&2
    echo "  You can manually download and place it at: $dest" >&2
    exit 1
}

# ─── Helper: validate tarball by attempting to list contents ───────────────────
validate_tarball() {
    local tarball="$1"
    if ! tar -tzf "$tarball" > /dev/null 2>&1; then
        echo "  WARNING: Tarball is corrupt, removing: $tarball"
        rm -f "$tarball"
        return 1
    fi
    return 0
}

echo "=== SpeakType macOS Bundle Builder ==="
echo "  Model:    $MODEL_SIZE"
echo "  Arch:     $ARCH ($PYTHON_TRIPLE)"
echo "  Output:   $RESOURCES"
echo ""

mkdir -p "$CACHE_DIR" "$RESOURCES"

# ─── 1. Download & extract Python standalone ──────────────────────────────────
echo "=== Step 1/5: Python standalone ==="
PYTHON_BIN="$RESOURCES/python/bin/python3"

if [ -f "$PYTHON_BIN" ]; then
    echo "  Already extracted, skipping. Python: $($PYTHON_BIN --version)"
else
    download_with_retry "$PYTHON_URL" "$CACHE_DIR/$PYTHON_TARBALL" "Python $PYTHON_VERSION"

    # Validate before extracting
    if ! validate_tarball "$CACHE_DIR/$PYTHON_TARBALL"; then
        # Re-download if corrupt
        download_with_retry "$PYTHON_URL" "$CACHE_DIR/$PYTHON_TARBALL" "Python $PYTHON_VERSION (re-download)"
        validate_tarball "$CACHE_DIR/$PYTHON_TARBALL" || { echo "ERROR: Downloaded tarball is corrupt" >&2; exit 1; }
    fi

    echo "  Extracting to $RESOURCES/python..."
    rm -rf "$RESOURCES/python" "$RESOURCES/_tmp"
    mkdir -p "$RESOURCES/_tmp"
    tar -xzf "$CACHE_DIR/$PYTHON_TARBALL" -C "$RESOURCES/_tmp"
    mv "$RESOURCES/_tmp/python" "$RESOURCES/python"
    rm -rf "$RESOURCES/_tmp"

    if [ ! -f "$PYTHON_BIN" ]; then
        echo "ERROR: Python binary not found at $PYTHON_BIN" >&2
        ls -la "$RESOURCES/python/" 2>/dev/null || echo "  (directory does not exist)"
        exit 1
    fi
    echo "  Python: $($PYTHON_BIN --version)"
fi

# ─── 2. Install server dependencies ───────────────────────────────────────────
echo ""
echo "=== Step 2/5: Installing server dependencies ==="

# Check if deps are already installed
if "$PYTHON_BIN" -c "import faster_whisper; import fastapi; import uvicorn" 2>/dev/null; then
    echo "  Dependencies already installed, skipping."
else
    for attempt in $(seq 1 $MAX_RETRIES); do
        echo "  Installing packages (attempt $attempt/$MAX_RETRIES)..."
        if "$PYTHON_BIN" -m pip install --quiet --no-warn-script-location \
            numpy faster-whisper fastapi uvicorn python-multipart; then
            break
        fi
        if [ "$attempt" -eq "$MAX_RETRIES" ]; then
            echo "ERROR: Failed to install dependencies after $MAX_RETRIES attempts" >&2
            exit 1
        fi
        echo "  Retrying in $((attempt * 5))s..."
        sleep $((attempt * 5))
    done
fi

echo "  Verifying imports..."
"$PYTHON_BIN" -c "import faster_whisper; import fastapi; import uvicorn; print('  All imports OK')"

# ─── 3. Download Whisper model ─────────────────────────────────────────────────
echo ""
echo "=== Step 3/5: Downloading Whisper model ($MODEL_SIZE) ==="
MODEL_DIR="$RESOURCES/models/whisper-${MODEL_SIZE}"
mkdir -p "$MODEL_DIR"

# Check if model is already downloaded (look for model.bin — the largest file)
if [ -f "$MODEL_DIR/model.bin" ]; then
    echo "  Model already downloaded, skipping."
else
    # huggingface_hub (used by faster-whisper) supports resumable downloads natively.
    # It downloads to a cache dir first, then moves to the output dir, so partial
    # downloads are automatically resumed on retry.
    for attempt in $(seq 1 $MAX_RETRIES); do
        echo "  Downloading model (attempt $attempt/$MAX_RETRIES)..."
        if "$PYTHON_BIN" -c "
from faster_whisper.utils import download_model
path = download_model('$MODEL_SIZE', output_dir='$MODEL_DIR')
print(f'  Model downloaded to: {path}')
"; then
            break
        fi
        if [ "$attempt" -eq "$MAX_RETRIES" ]; then
            echo "ERROR: Failed to download model after $MAX_RETRIES attempts" >&2
            echo "  You can manually download 'Systran/faster-whisper-${MODEL_SIZE}' from Hugging Face" >&2
            echo "  and place the files in: $MODEL_DIR" >&2
            exit 1
        fi
        echo "  Retrying in $((attempt * 5))s..."
        sleep $((attempt * 5))
    done

    # If the model was downloaded into a subdirectory, flatten it
    if [ -d "$MODEL_DIR/model" ]; then
        mv "$MODEL_DIR/model"/* "$MODEL_DIR/"
        rmdir "$MODEL_DIR/model"
    fi
fi

echo "  Model files:"
ls -lh "$MODEL_DIR/"

# ─── 4. Copy server script ────────────────────────────────────────────────────
echo ""
echo "=== Step 4/5: Copying server script ==="
mkdir -p "$RESOURCES/server"
cp "$PROJECT_ROOT/server/whisper_server.py" "$RESOURCES/server/"
echo "  Copied whisper_server.py"

# ─── 5. Strip bloat & codesign ────────────────────────────────────────────────
echo ""
echo "=== Step 5/5: Stripping bloat & codesigning ==="

echo "  Removing __pycache__, .pyc, tests, pip, ensurepip..."
find "$RESOURCES/python" -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
find "$RESOURCES/python" -name "*.pyc" -delete 2>/dev/null || true
find "$RESOURCES/python" -type d -name "test" -exec rm -rf {} + 2>/dev/null || true
find "$RESOURCES/python" -type d -name "tests" -exec rm -rf {} + 2>/dev/null || true
find "$RESOURCES/python" -type d -name "testing" -exec rm -rf {} + 2>/dev/null || true
rm -rf "$RESOURCES/python/lib/python3.12/ensurepip" 2>/dev/null || true
rm -rf "$RESOURCES/python/lib/python3.12/idlelib" 2>/dev/null || true
rm -rf "$RESOURCES/python/lib/python3.12/tkinter" 2>/dev/null || true
rm -rf "$RESOURCES/python/lib/python3.12/turtle*" 2>/dev/null || true
rm -rf "$RESOURCES/python/lib/python3.12/lib2to3" 2>/dev/null || true
rm -rf "$RESOURCES/python/share" 2>/dev/null || true

echo "  Codesigning native libraries..."
SIGN_COUNT=0
while IFS= read -r -d '' lib; do
    codesign --force --sign - --timestamp=none "$lib" 2>/dev/null && ((SIGN_COUNT++)) || true
done < <(find "$RESOURCES/python" \( -name "*.so" -o -name "*.dylib" \) -print0)
echo "  Signed $SIGN_COUNT libraries"

# ─── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo "=== Bundle resources ready ==="
TOTAL_SIZE=$(du -sh "$RESOURCES" | cut -f1)
echo "  Location: $RESOURCES"
echo "  Total size: $TOTAL_SIZE"
echo ""
echo "  python/   $(du -sh "$RESOURCES/python" | cut -f1)"
echo "  server/   $(du -sh "$RESOURCES/server" | cut -f1)"
echo "  models/   $(du -sh "$RESOURCES/models" | cut -f1)"
echo ""
echo "Next: run 'make bundle' to build the .dmg"
