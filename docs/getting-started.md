# Getting started

SpeakType runs locally on your machine. The macOS desktop app is the simplest path; Linux and Windows use the Python CLI.

## Decision tree

**On macOS and want the simplest setup?**  
→ Download the [desktop app (DMG)](install/macos-widget.md). No Python or Rust required.

**Prefer the terminal, or on Linux/Windows?**  
→ Install the [Python CLI](install/linux.md) (see platform guides below).

**Contributing to the widget?**  
→ [Build from source](build-widget-from-source.md) (macOS, Rust, cmake, sidecar build).

## Prerequisites by path

| Path | Required | Optional |
|------|----------|----------|
| **Widget (DMG user)** | macOS 12+, nothing else | — |
| **Widget (build from source)** | macOS, Xcode CLI, Rust, cmake, Tauri CLI | Apple Developer ID (release signing only) |
| **Python CLI** | Python 3.10+, platform audio deps | NVIDIA + `pip install -e ".[gpu]"` |
| **Node.js** | Not required | — |

## Platform install guides

- [macOS widget (DMG)](install/macos-widget.md) — primary path for macOS users
- [macOS CLI](install/macos-cli.md)
- [Linux](install/linux.md)
- [Windows](install/windows.md)

## After install

1. Grant **Microphone** and **Accessibility** on macOS — [permissions guide](permissions.md)
2. Press your hotkey — **⌘+Option+L** (default) on the macOS app, **F9** (default) on the CLI — speak, press again
3. Text pastes into the focused window (terminals use Ctrl+Shift+V automatically)

## Models

Download once, use offline. Default is **`base`** — good balance of speed and accuracy.

| Model | Download | RAM |
|-------|----------|-----|
| tiny | ~75MB | ~1GB |
| base | ~150MB | ~1GB |
| small | ~500MB | ~2GB |
| medium | ~1.5GB | ~5GB |
| large-v3 | ~3GB | ~10GB |

In the widget: **Settings → Models**. In the CLI: `--model small`.

## Troubleshooting

- **No speech detected** — check mic input, speak at normal volume, try a smaller model
- **Linux hotkey not working** — pynput needs X11; on Wayland use an X11 session or `GDK_BACKEND=x11`
- **macOS permissions** — grant Accessibility to SpeakType or your terminal — [permissions](permissions.md)
- **Slow CLI inference** — use a smaller model (`--model tiny` or `--model base`)

More detail: [Client setup](client-setup.md)

## Advanced (optional)

For CLI power users who restart the client often:

- **Python server** — keeps the model loaded for faster restarts; point the CLI with `--api`. See [advanced Python server](advanced-python-server.md).
- **External APIs** — OpenAI-compatible endpoints (e.g. Groq). See [Client setup → Advanced](client-setup.md#advanced-optional).
