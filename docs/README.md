# SpeakType documentation

SpeakType is push-to-talk voice typing with local Whisper transcription. Pick the path that fits how you work.

## Choose your path

| I want… | Start here |
|---------|------------|
| Easiest macOS experience | [Install the desktop app (DMG)](install/macos-widget.md) |
| Python CLI on macOS | [macOS CLI setup](install/macos-cli.md) |
| Python CLI on Linux | [Linux setup](install/linux.md) |
| Python CLI on Windows | [Windows setup](install/windows.md) |
| Decide between widget and CLI | [Getting started](getting-started.md) |
| Build the widget from source | [Widget build guide](build-widget-from-source.md) |
| macOS permissions (mic + accessibility) | [Permissions](permissions.md) |
| Privacy and data handling | [Privacy](privacy.md) |
| CLI deep dive (flags, options) | [Client setup](client-setup.md) |
| Security hardening notes | [Security](security.md) |

## Advanced (CLI power users)

Optional setups — not required for normal use:

| I want… | Start here |
|---------|------------|
| Keep a model loaded for faster CLI restarts | [Advanced Python server](advanced-python-server.md) |
| External transcription APIs (Groq, etc.) | [Client setup → Advanced](client-setup.md#advanced-optional) |

## Components

| Component | UI | Inference | Endpoint |
|-----------|-----|-----------|----------|
| Desktop app | Tauri widget | whisper.cpp sidecar | `POST /inference` |
| CLI (default) | Terminal | faster-whisper in-process | — |
| Python server + CLI | Terminal | faster-whisper | `POST /transcribe` |

The desktop app and Python stack are independent — the widget does not use the Python server.
