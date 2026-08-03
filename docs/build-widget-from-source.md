# Build the Widget from Source

Build the SpeakType desktop widget on macOS. End users should **download the `.dmg` from Releases** instead — no build required.

> **Node.js is not required.** The UI is static HTML in `packages/widget-ui/`.

## Prerequisites

| Tool | Version / notes |
|------|-----------------|
| macOS | 12+ (Apple Silicon tested) |
| Xcode Command Line Tools | `xcode-select --install` |
| Rust | 1.93+ (see `apps/widget-rust/rust-toolchain.toml`) |
| cmake | `brew install cmake` |
| librsvg | `brew install librsvg` — provides `rsvg-convert` for DMG background |
| Tauri CLI 2 | `cargo install tauri-cli` |
| Python (optional) | 3.10+ for `make install` — prefers `python3.12` or `python3.11` if installed |

**Release signing only:** Apple Developer ID + Team ID via env or gitignored `make/local.mk` — see `make/config.example`.

## Why the sidecar step matters

The widget bundles a **whisper.cpp** binary (`whisper-server`) as an external sidecar. That binary is **gitignored** at:

```
apps/widget-rust/src-tauri/binaries/whisper-server-aarch64-apple-darwin
```

`cargo tauri dev` and `cargo tauri build` **fail without it**. You must build the sidecar first.

## Build the sidecar (one-time per clone, rebuild after whisper.cpp updates)

From the repo root:

```bash
WHISPER_SRC="apps/widget-rust/src-tauri/scripts/build/whisper.cpp"
SIDECAR_BUILD="$WHISPER_SRC/build"

# Clone whisper.cpp if missing (directory is gitignored)
if [ ! -d "$WHISPER_SRC" ]; then
  mkdir -p "$(dirname "$WHISPER_SRC")"
  git clone https://github.com/ggerganov/whisper.cpp.git "$WHISPER_SRC"
fi

mkdir -p "$SIDECAR_BUILD"
cd "$SIDECAR_BUILD"
cmake -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DGGML_METAL=ON ..
cmake --build . --target whisper-server -j"$(sysctl -n hw.ncpu)"

mkdir -p ../../binaries
cp bin/whisper-server ../../binaries/whisper-server-aarch64-apple-darwin
chmod +x ../../binaries/whisper-server-aarch64-apple-darwin
```

This matches step 2 of `scripts/build.sh`.

## Development

```bash
make install   # optional: Python venv for CLI/server work
make dev       # cargo tauri dev
```

Grant Microphone + Accessibility when the app launches.

## Release build (signed DMG)

Full pipeline: clean → sidecar → Tauri → codesign → DMG → notarize.

```bash
export APPLE_DEVELOPER_ID="Developer ID Application: Your Name (TEAMID)"
export APPLE_TEAM_ID="TEAMID"
# or: cp make/config.example make/local.mk  # gitignored

make build     # runs scripts/build.sh → dist/SpeakType_0.1.0_aarch64.dmg
```

One-time notary credentials: `make notary-setup`

## Project layout

| Path | Purpose |
|------|---------|
| `apps/widget-rust/` | Tauri Rust shell |
| `packages/widget-ui/` | HTML/CSS/JS UI (no npm build step) |
| `apps/widget-rust/src-tauri/binaries/` | Sidecar binaries (gitignored) |
| `scripts/build.sh` | Full release pipeline |

## CI note

GitHub Actions release workflow clones whisper.cpp and builds the sidecar before `cargo tauri build`. Local `make dev` does not — you build the sidecar manually as above.

## Regenerate marketing assets

```bash
make assets
# or: python3 scripts/assets/compose_showcase.py
```

## Version

Widget version is **0.1.0** (pre-1.0) in `apps/widget-rust/src-tauri/tauri.conf.json`. Python packages are **1.0.0**.
