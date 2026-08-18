#!/usr/bin/env bash
#
# setup-dev.sh — One-shot dev environment setup for SpeakType.
#
#   make install
#
# Checks minimum versions, installs only what's missing or too old.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WHISPER_SRC="$PROJECT_ROOT/apps/widget-rust/src-tauri/scripts/build/whisper.cpp"

MIN_PYTHON="3.10.0"
MIN_CMAKE="3.16.0"
MIN_TAURI="2.0.0"

# shellcheck source=whisper-sidecar.sh
source "$SCRIPT_DIR/whisper-sidecar.sh"
# shellcheck source=load-secrets.sh
source "$SCRIPT_DIR/load-secrets.sh"

version_ge() {
    local current="${1#v}"
    local minimum="${2#v}"
    [[ "$(printf '%s\n' "$minimum" "$current" | sort -V | head -1)" == "$minimum" ]]
}

parse_version() {
    grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1
}

need_brew() {
    if command -v brew >/dev/null 2>&1; then
        return 0
    fi
    echo "ERROR: Homebrew required to install $1."
    return 1
}

ensure_python() {
    local py="" ver=""
    for candidate in python3.12 python3.11 python3; do
        if command -v "$candidate" >/dev/null 2>&1 \
            && "$candidate" -c 'import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)' 2>/dev/null; then
            py="$candidate"
            ver="$("$candidate" -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}")')"
            break
        fi
    done
    if [ -z "$py" ]; then
        echo "ERROR: Python ${MIN_PYTHON}+ required (python3.12, python3.11, or python3)."
        exit 1
    fi
    if ! version_ge "$ver" "$MIN_PYTHON"; then
        echo "ERROR: Python $ver found but ${MIN_PYTHON}+ required."
        exit 1
    fi

    if [ -x "$PROJECT_ROOT/.venv/bin/python" ] \
        && "$PROJECT_ROOT/.venv/bin/pip" show speaktype >/dev/null 2>&1; then
        echo "   ✓ Python $ver (.venv ready)"
        return 0
    fi

    echo "→ Python venv + pip packages..."
    make -C "$PROJECT_ROOT" install-python
}

ensure_cmake() {
    local ver=""
    if command -v cmake >/dev/null 2>&1; then
        ver="$(cmake --version 2>/dev/null | parse_version)"
        if [ -n "$ver" ] && version_ge "$ver" "$MIN_CMAKE"; then
            echo "   ✓ cmake $ver"
            return 0
        fi
        echo "→ cmake ${ver:-unknown} below minimum $MIN_CMAKE — upgrading..."
    else
        echo "→ Installing cmake..."
    fi
    need_brew cmake
    brew install cmake
}

ensure_librsvg() {
    if command -v rsvg-convert >/dev/null 2>&1; then
        echo "   ✓ rsvg-convert $(rsvg-convert --version 2>&1 | head -1)"
        return 0
    fi
    echo "→ Installing librsvg..."
    need_brew librsvg
    brew install librsvg
}

ensure_system_xattr() {
    local xattr_path=""
    xattr_path="$(command -v xattr 2>/dev/null || true)"
    if [ ! -x /usr/bin/xattr ]; then
        echo "ERROR: /usr/bin/xattr missing — required for Tauri app bundling."
        exit 1
    fi
    if [ -n "$xattr_path" ] && [ "$xattr_path" != "/usr/bin/xattr" ] \
        && ! "$xattr_path" -h 2>&1 | grep -q ' \[-r\]'; then
        echo "   ⚠ xattr at $xattr_path is not Apple's (breaks Tauri bundling)"
        echo "     Fix: brew uninstall xattr"
        echo "     build.sh prefers /usr/bin/xattr automatically"
    else
        echo "   ✓ xattr ($(command -v xattr))"
    fi
}

ensure_rust() {
    speaktype_load_rust_env
    local toolchain_file="$PROJECT_ROOT/apps/widget-rust/rust-toolchain.toml"
    local channel="1.93.1"
    if [ -f "$toolchain_file" ]; then
        channel="$(sed -n 's/^channel = "\(.*\)"/\1/p' "$toolchain_file" | head -1)"
        channel="${channel:-1.93.1}"
    fi

    if command -v rustc >/dev/null 2>&1; then
        local ver
        ver="$(rustc --version 2>/dev/null | awk '{print $2}')"
        if [ -n "$ver" ] && version_ge "$ver" "$channel"; then
            local active=""
            if command -v rustup >/dev/null 2>&1; then
                active="$(rustup show active-toolchain 2>/dev/null | awk '{print $1}')"
                if [[ "$active" != "${channel}"* ]]; then
                    echo "→ Setting default Rust toolchain to $channel..."
                    rustup default "$channel"
                fi
            fi
            echo "   ✓ Rust $ver (>= $channel)"
            return 0
        fi
        echo "→ Rust ${ver:-unknown} below minimum $channel — updating..."
    fi

    if ! command -v rustup >/dev/null 2>&1; then
        echo "→ Installing Rust (rustup)..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | sh -s -- -y --default-toolchain "$channel"
        speaktype_load_rust_env
    fi

    if ! command -v rustup >/dev/null 2>&1; then
        echo "ERROR: rustup not found after install."
        exit 1
    fi

    echo "→ Installing Rust toolchain $channel..."
    rustup toolchain install "$channel"
    rustup default "$channel"
    echo "   ✓ Rust $(rustc --version | awk '{print $2}')"
}

ensure_tauri_cli() {
    speaktype_load_rust_env
    local ver=""
    if command -v cargo-tauri >/dev/null 2>&1; then
        ver="$(cargo-tauri --version 2>/dev/null | parse_version)"
    elif cargo tauri --version >/dev/null 2>&1; then
        ver="$(cargo tauri --version 2>/dev/null | parse_version)"
    fi

    if [ -n "$ver" ] && version_ge "$ver" "$MIN_TAURI"; then
        echo "   ✓ Tauri CLI $ver"
        return 0
    fi

    if [ -n "$ver" ]; then
        echo "→ Tauri CLI $ver below minimum $MIN_TAURI — upgrading..."
    else
        echo "→ Installing Tauri CLI (this can take a few minutes)..."
    fi
    cargo install tauri-cli --locked
    echo "   ✓ Tauri CLI $(cargo tauri --version 2>/dev/null | parse_version)"
}

echo "═══════════════════════════════════════════════════════════════"
echo "  SpeakType dev setup"
echo "═══════════════════════════════════════════════════════════════"

ensure_python

if [ "$(uname -s)" = "Darwin" ]; then
    echo "→ macOS widget prerequisites..."

    if ! xcode-select -p >/dev/null 2>&1; then
        echo "ERROR: Xcode Command Line Tools missing. Run: xcode-select --install"
        exit 1
    fi
    echo "   ✓ Xcode Command Line Tools"

    ensure_cmake
    ensure_librsvg
    ensure_system_xattr
    ensure_rust
    ensure_tauri_cli

    ensure_whisper_src "$WHISPER_SRC"
    echo "   ✓ whisper.cpp at $WHISPER_SRC"
fi

echo "═══════════════════════════════════════════════════════════════"
echo "  ✓ Dev setup complete"
echo "  Next: make dev (widget)  ·  make build (release DMG)"
echo "═══════════════════════════════════════════════════════════════"
