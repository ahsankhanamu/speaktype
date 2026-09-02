# SpeakType

**Push-to-talk voice typing that works everywhere.**

Press a hotkey, speak, press again — your words appear wherever you're typing. Local Whisper on your machine. No cloud required.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![macOS](https://img.shields.io/badge/macOS-widget%20%2B%20CLI-lightgrey)
![Linux](https://img.shields.io/badge/Linux-CLI-lightgrey)
![Windows](https://img.shields.io/badge/Windows-CLI-lightgrey)

<p align="center">
  <img src="assets/demo.gif" alt="SpeakType demo — press hotkey, speak, text appears" width="720">
</p>

<p align="center"><sub>More screenshots in <code>assets/</code> and <a href="https://github.com/ahsankhanamu/speaktype/releases">Releases</a>.</sub></p>

## Quick start

### macOS (recommended) — download the app

1. Download the latest **`SpeakType_*_aarch64.dmg`** from [**GitHub Releases**](https://github.com/ahsankhanamu/speaktype/releases)
2. Drag **SpeakType** to **Applications** and launch it
3. Grant **Microphone** and **Accessibility** when prompted
4. Download a model in **Settings → Models** (`base` is a good default)
5. Press **⌘+Option+L** (default), speak, press again — text pastes into the focused window

→ Full walkthrough: [docs/install/macos-widget.md](docs/install/macos-widget.md)

### Linux / Windows — Python CLI

```bash
git clone https://github.com/ahsankhanamu/speaktype.git && cd speaktype
make install          # or ./install.sh on Linux
source .venv/bin/activate
speaktype
```

→ [Linux](docs/install/linux.md) · [Windows](docs/install/windows.md) · [macOS CLI](docs/install/macos-cli.md)

## Why SpeakType?

- **Speak naturally** — explain fully instead of typing truncated thoughts
- **Works everywhere** — terminals, IDEs, browsers, Slack, email
- **Private by default** — audio and transcription stay on your machine
- **Open source** — MIT licensed; inspect, fork, contribute

## Privacy

**No accounts, no telemetry, no cloud** — audio stays on your machine unless you explicitly configure an external API in the CLI.

→ [docs/privacy.md](docs/privacy.md)

## Features

- Floating always-on-top widget with recording animation (macOS)
- Configurable global hotkey — ⌘+Option+L on the macOS app, F9 on the CLI
- Smart paste — Ctrl+Shift+V in terminals, Ctrl+V elsewhere
- Model picker — tiny through large-v3, downloaded once for offline use
- History and settings — hotkey, models, server options

## Choose your path

| Path | Best for | Get started |
|------|----------|-------------|
| **Desktop widget** | Easiest macOS experience | [Download DMG](https://github.com/ahsankhanamu/speaktype/releases) → [install guide](docs/install/macos-widget.md) |
| **Python CLI** | Linux, Windows, macOS terminal | [Getting started](docs/getting-started.md) → [client setup](docs/client-setup.md) |
| **Build from source** | Contributors, custom builds | [Build guide](docs/build-widget-from-source.md) |

Advanced: optional Python server and external APIs for CLI power users — see [docs](docs/README.md#advanced-cli-power-users).

> **Versions:** Desktop widget **v0.1.0** (pre-1.0). Python CLI **v1.0.0**.

<details>
<summary>Advanced (optional Python server)</summary>

For CLI users who restart often: run `speaktype-server` locally and point the client with `--api`. Not used by the macOS widget. See [advanced Python server](docs/advanced-python-server.md).

</details>

## Permissions (macOS)

SpeakType needs **Microphone** and **Accessibility** only — no screen recording, no full disk access.

→ [docs/permissions.md](docs/permissions.md)

## For developers

```bash
git clone https://github.com/ahsankhanamu/speaktype.git && cd speaktype
make install          # Python venv + pip install -e ".[all]"
# Build whisper.cpp sidecar first — see build doc
make dev              # Tauri widget (macOS)
make build            # Signed DMG via tools/build/build.sh
```

Node.js is **not** required for the widget. See [docs/build-widget-from-source.md](docs/build-widget-from-source.md).

Regenerate README media: `make assets`

## Contributing

Contributions welcome — Wayland support, streaming transcription, and docs improvements are great starting points.

→ [CONTRIBUTING.md](CONTRIBUTING.md) · [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

[Tauri](https://tauri.app/) · [whisper.cpp](https://github.com/ggerganov/whisper.cpp) · [OpenAI Whisper](https://github.com/openai/whisper) · [faster-whisper](https://github.com/SYSTRAN/faster-whisper)
