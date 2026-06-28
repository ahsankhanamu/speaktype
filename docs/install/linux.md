# Linux — Python CLI

SpeakType on Linux is **CLI-only**. There is no widget DMG for Linux — use the Python client with local faster-whisper or an external API.

## Quick install

```bash
git clone https://github.com/ahsankhanamu/speaktype.git && cd speaktype
./install.sh
source .venv/bin/activate
speaktype
```

The installer detects your distro, installs system packages (`xdotool`, `xclip`, `portaudio` dev headers), and creates a venv with **Python 3.10+** (prefers `python3.12` or `python3.11` when available).

## Manual install

### Debian / Ubuntu

```bash
sudo apt install xdotool xclip portaudio19-dev python3-venv
git clone https://github.com/ahsankhanamu/speaktype.git && cd speaktype
make install   # prefers python3.12 / python3.11, requires 3.10+
# or manually:
python3 -m venv .venv && source .venv/bin/activate
pip install -e ".[all]"
```

### Fedora / RHEL

```bash
sudo dnf install xdotool xclip portaudio-devel python3-venv
```

### Arch

```bash
sudo pacman -S xdotool xclip portaudio
```

## Run

```bash
source .venv/bin/activate
speaktype
```

Press **F9**, speak, press **F9** again. Text pastes into the focused window.

### With Python server (optional)

```bash
speaktype-server --model base &
speaktype --api http://localhost:8002/transcribe
```

## Wayland vs X11

Global hotkeys use **pynput**, which requires **X11**. On Wayland:

- Switch to an X11 session, or
- Run with `GDK_BACKEND=x11`

Native Wayland support is a welcome contribution — see [CONTRIBUTING.md](../../CONTRIBUTING.md).

## Run as a systemd user service

Create `~/.config/systemd/user/speaktype.service`:

```ini
[Unit]
Description=SpeakType voice typing CLI
After=default.target

[Service]
Type=simple
WorkingDirectory=%h/speaktype
ExecStart=%h/speaktype/.venv/bin/speaktype --minimal
Restart=on-failure

[Install]
WantedBy=default.target
```

Enable with `systemctl --user enable --now speaktype.service`.

Adjust paths to match your clone location.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| No speech detected | Check default mic: `pactl list sources short`; boost gain if needed |
| Hotkey ignored | Confirm X11 session; check pynput permissions |
| Port 8002 in use | `lsof -i :8002` or change server `--port` |

See also [client-setup.md](../client-setup.md) and the root README troubleshooting section.
