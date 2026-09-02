# Client Setup (Python CLI)

The CLI runs on your host machine because it needs access to your microphone, keyboard, and display.

Install paths: [macOS CLI](install/macos-cli.md) · [Linux](install/linux.md) · [Windows](install/windows.md)

## Install

```bash
git clone https://github.com/ahsankhanamu/speaktype.git && cd speaktype
make install
source .venv/bin/activate
```

This installs the `speaktype` console entry point via `pip install -e ".[all]"`.

## macOS permissions

Grant **Accessibility** to your terminal app:

System Settings → Privacy & Security → Accessibility → enable Terminal or iTerm2.

See [permissions.md](permissions.md).

## Linux system deps

```bash
sudo apt install xdotool xclip portaudio19-dev   # Debian/Ubuntu
```

Or run `./install.sh`.

## Running

### Local faster-whisper (default)

Loads the model in-process — the recommended path for most CLI users:

```bash
speaktype
# equivalent: python packages/python/cli/speaktype.py
```

## Common options

```bash
speaktype --model small
speaktype --hotkey f8
speaktype --language en
speaktype --minimal
```

## How it works

1. Press F9 — starts recording from your microphone
2. Speak your text
3. Press F9 again — stops recording, transcribes, pastes into the focused window

The client remembers which window was focused when recording started, so you can switch to the terminal for status and text still pastes in the right place.

Smart paste: **Ctrl+Shift+V** in terminals, **Ctrl+V** elsewhere (Linux uses xdotool/xclip).

## CLI flags reference

| Flag | Description |
|------|-------------|
| `--api URL` | External transcription API |
| `--api-model NAME` | Model name for OpenAI-compatible APIs |
| `--model MODEL` | Local model: tiny, base, small, medium, large-v3 |
| `--hotkey KEY` | Hotkey (default: f9) |
| `--language CODE` | Language code (default: auto) |
| `--minimal` | Minimal terminal UI |

Run `speaktype --help` for the full list.

## Desktop widget

macOS users who want the floating UI should use the [DMG install](install/macos-widget.md) instead — it uses whisper.cpp, not this CLI stack.

## Advanced (optional)

### Python server

If you run the CLI frequently, an optional local server keeps the model loaded between sessions:

```bash
# Terminal 1
speaktype-server --model base

# Terminal 2
speaktype --api http://localhost:8002/transcribe
```

Full details: [advanced Python server](advanced-python-server.md).

### External APIs

SpeakType auto-detects OpenAI-compatible endpoints:

```bash
speaktype --api https://api.groq.com/openai/v1/audio/transcriptions --api-model whisper-large-v3
```

Using an external API sends audio off your machine — see [privacy.md](privacy.md).
