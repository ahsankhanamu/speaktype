# macOS Permissions

SpeakType needs exactly **two** permissions on macOS. Nothing else — no screen recording, no full disk access, no contacts.

| Permission | Why SpeakType needs it |
|------------|------------------------|
| **Microphone** | Record your voice when you press the hotkey |
| **Accessibility** | Listen for the global hotkey and paste text into other apps |

## Desktop widget

On first launch, SpeakType's onboarding guides you through each step:

1. **Microphone** — click Allow in the system dialog, or open System Settings → Privacy & Security → Microphone and enable SpeakType
2. **Accessibility** — open System Settings → Privacy & Security → Accessibility and enable SpeakType

If you skip a step, the onboarding screen lets you retry.

### Re-granting permissions

If the hotkey stops working after an macOS update:

1. System Settings → Privacy & Security → Accessibility
2. Toggle SpeakType off and on, or remove and re-add it
3. Quit and relaunch SpeakType

## Python CLI

The CLI runs inside your terminal. Grant **Accessibility** to **Terminal**, **iTerm2**, or whichever app you use — not a separate "SpeakType" entry.

Microphone access is requested the first time you record.

```bash
# After granting permissions, test:
speaktype --minimal
```

## Privacy

SpeakType does not upload audio anywhere by default. See [privacy.md](privacy.md).

## What SpeakType does *not* need

- Screen Recording
- Full Disk Access
- Input Monitoring (separate from Accessibility on some macOS versions — Accessibility covers hotkey + paste)
- Network (except when you download models or use optional API mode)
