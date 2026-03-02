# Client Setup (speaktype-cli/speaktype.py)

The client runs on your host machine (not in Docker) because it needs access to your microphone, keyboard, and display.

## macOS

```bash
# Install system dependency
brew install portaudio

# Set up Python environment
cd speaktype
python3 -m venv .venv
source .venv/bin/activate
pip install ".[client]"
```

### Permissions

macOS requires accessibility permissions for keyboard monitoring:
1. System Preferences > Security & Privacy > Privacy > Accessibility
2. Add your terminal app (Terminal, iTerm2, etc.)

## Linux

```bash
# System dependencies
sudo apt install xdotool xclip portaudio19-dev

# Python environment
cd speaktype
python3 -m venv .venv
source .venv/bin/activate
pip install ".[client]"
```

## Windows

```powershell
cd speaktype
python -m venv .venv
.\.venv\Scripts\activate
pip install ".[client]"
```

## Running

With the Docker whisper server already running:

```bash
source .venv/bin/activate
python speaktype-cli/speaktype.py --api http://localhost:8003/transcribe
```

### Common Options

```bash
# Change hotkey
python speaktype-cli/speaktype.py --api http://localhost:8003/transcribe --hotkey f8

# Set language (skip auto-detect for faster results)
python speaktype-cli/speaktype.py --api http://localhost:8003/transcribe --language en

# Minimal UI
python speaktype-cli/speaktype.py --api http://localhost:8003/transcribe --minimal
```

## How It Works

1. Press F9 — starts recording from your microphone
2. Speak your text
3. Press F9 again — stops recording, sends audio to the whisper server
4. Transcribed text is pasted into your currently focused window

The client remembers which window was focused when you started recording, so you can switch to SpeakType's terminal to see status, then your text still gets pasted in the right place.
