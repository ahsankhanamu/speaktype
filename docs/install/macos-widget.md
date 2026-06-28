# macOS — Desktop Widget (Recommended)

The easiest way to use SpeakType on macOS is the signed `.dmg` from [GitHub Releases](https://github.com/ahsankhanamu/speaktype/releases). No Python, Rust, or Node.js required.

> **Version note:** The widget is v0.1.0 (pre-1.0). Python CLI/server packages are v1.0.0.

## Install

1. Download the latest `SpeakType_*_aarch64.dmg` from Releases
2. Open the DMG and drag **SpeakType** to **Applications**
3. Launch SpeakType from Applications (or Spotlight)

On first launch, macOS may warn that the app is from an unidentified developer if you built from source without notarization. Release builds from the maintainer are signed and notarized.

## First-run onboarding

SpeakType walks you through setup:

1. **Microphone** — allow SpeakType to record your voice
2. **Accessibility** — required for the global hotkey and pasting into other apps
3. **Download a model** — pick a size in Settings → Models (`base` is a good default)

See [permissions.md](../permissions.md) for details on each permission.

## Daily use

1. The floating widget appears on screen (or lives in the menu bar)
2. Focus any text field — terminal, browser, IDE, Slack, etc.
3. Press **⌘+Option+L** (default) to start recording
4. Speak naturally
5. Press **⌘+Option+L** again — transcribed text is pasted into the focused window

Change the hotkey in **Settings → Hotkey** (F9 is available as a preset). Adjust model and other options in **Settings**.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| Hotkey does nothing | Re-grant Accessibility in System Settings → Privacy & Security → Accessibility |
| No speech detected | Check mic input in System Settings → Sound → Input |
| Slow transcription | Use a smaller model (`tiny` or `base`) in Settings → Models |
| Want CLI instead | See [macos-cli.md](macos-cli.md) |

## Build from source

Developers and contributors: [build-widget-from-source.md](../build-widget-from-source.md)
