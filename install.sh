#!/usr/bin/env bash
# SpeakType installer for Linux (CLI only — see docs/install/linux.md)

set -e

echo "Installing SpeakType CLI..."

if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo "This installer is for Linux. See docs/getting-started.md for macOS and Windows."
    exit 1
fi

DISTRO=""
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO=$ID
elif type lsb_release >/dev/null 2>&1; then
    DISTRO=$(lsb_release -si)
elif [ -f /etc/redhat-release ]; then
    DISTRO=$(awk '{print tolower($1)}' /etc/redhat-release)
elif [ -f /etc/arch-release ]; then
    DISTRO="arch"
fi

DISTRO=$(echo "$DISTRO" | tr '[:upper:]' '[:lower:]')
echo "Detected distribution: $DISTRO"

echo "Installing system dependencies..."
case "$DISTRO" in
    ubuntu|debian|linuxmint)
        sudo apt-get update -qq
        sudo apt-get install -y -qq xdotool xclip portaudio19-dev python3-venv
        ;;
    fedora|centos|rhel)
        sudo dnf check-update || sudo yum check-update
        sudo dnf install -y xdotool xclip portaudio-devel python3-venv || \
        sudo yum install -y xdotool xclip portaudio-devel python3-venv
        ;;
    arch|manjaro)
        sudo pacman -Sy --noconfirm
        sudo pacman -S --noconfirm xdotool xclip portaudio python-venv
        ;;
    suse|opensuse|sles)
        sudo zypper refresh
        sudo zypper install -y xdotool xclip portaudio-devel
        ;;
    *)
        echo "Unsupported distribution: $DISTRO"
        echo "Install xdotool, xclip, portaudio dev headers, and python3-venv manually."
        ;;
esac

echo "Setting up Python environment..."
PYTHON=""
for candidate in python3.12 python3.11 python3; do
    if command -v "$candidate" >/dev/null 2>&1 && \
       "$candidate" -c 'import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)' 2>/dev/null; then
        PYTHON=$candidate
        break
    fi
done

if [ -z "$PYTHON" ]; then
    echo "Error: Python 3.10+ is required. Install python3.12, python3.11, or upgrade python3."
    exit 1
fi

echo "Using $($PYTHON --version) ($PYTHON)"
$PYTHON -m venv .venv
source .venv/bin/activate
pip install -q -e ".[all]"

echo ""
echo "Installation complete!"
echo ""
echo "Linux installs the Python CLI only. The desktop widget is macOS-only (download a .dmg or build from source — see docs/install/macos-widget.md)."
echo ""
echo "To run SpeakType:"
echo "  source .venv/bin/activate"
echo "  speaktype"
