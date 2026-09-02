# macOS — Python CLI

Use the CLI when you prefer terminal workflows or want faster-whisper in-process without the desktop widget.

## Prerequisites

- Python 3.10+
- Homebrew `portaudio`

```bash
brew install portaudio
```

## Install

```bash
git clone https://github.com/ahsankhanamu/speaktype.git && cd speaktype
make install   # prefers python3.12 / python3.11, requires 3.10+
# or manually:
python3 -m venv .venv && source .venv/bin/activate
pip install -e ".[all]"
```

## Permissions

Grant **Accessibility** to your terminal app (Terminal, iTerm2, etc.):

System Settings → Privacy & Security → Accessibility → enable your terminal.

Microphone access is requested when you first record.

Details: [permissions.md](../permissions.md)

## Run

```bash
source .venv/bin/activate
speaktype
```

Or without the entry point:

```bash
python packages/python/cli/speaktype.py
```

Press **F9**, speak, press **F9** again.

### Common flags

```bash
speaktype --model small
speaktype --hotkey f8
speaktype --language en
speaktype --minimal
```

### External API (optional)

```bash
speaktype --api https://api.groq.com/openai/v1/audio/transcriptions --api-model whisper-large-v3
```

### With local Python server

Keeps the model loaded between recordings (faster restarts):

```bash
# Terminal 1
speaktype-server --model base

# Terminal 2
speaktype --api http://localhost:8002/transcribe
```

More detail: [client-setup.md](../client-setup.md)

## Prefer the desktop app?

Download the DMG: [macos-widget.md](macos-widget.md)
