# Windows — Python CLI

SpeakType on Windows uses the Python CLI. There is no desktop widget for Windows yet.

## Prerequisites

- Python 3.10+ from [python.org](https://www.python.org/downloads/) (check "Add to PATH")
- A working microphone

No extra system packages are required beyond Python.

## Install

```powershell
git clone https://github.com/ahsankhanamu/speaktype.git
cd speaktype
python -m venv .venv
.\.venv\Scripts\activate
pip install -e ".[all]"
```

## Run

```powershell
.\.venv\Scripts\activate
speaktype
```

Or:

```powershell
python packages\cli\speaktype.py
```

Press **F9**, speak, press **F9** again.

### Common flags

```powershell
speaktype --model small
speaktype --hotkey f8
speaktype --language en
```

### With Python server (optional)

```powershell
# Terminal 1
speaktype-server --model base

# Terminal 2
speaktype --api http://localhost:8002/transcribe
```

## Microphone settings

If SpeakType doesn't hear you:

1. Settings → System → Sound → Input — select the correct mic
2. Speak at normal volume; try `--model tiny` to test quickly

## Troubleshooting

Windows uses **pyautogui** for paste simulation (Ctrl+V). Some elevated or protected apps may block simulated keystrokes.

For API mode and advanced options, see [client-setup.md](../client-setup.md).
