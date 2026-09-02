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

macOS stores Accessibility per **code signature**, not per app name. A toggle for `/Applications/SpeakType.app` does not cover a different build (new Developer ID cert, ad-hoc debug binary, DMG copy on another volume).

### `make dev` (debug / Tauri)

`make dev` runs `apps/widget/src-tauri/target/debug/speaktype`, which is a different identity from `/Applications/SpeakType.app`.

On macOS, `make dev` builds the debug binary, wraps it in a real **SpeakType Dev.app**, signs it with your Developer ID as `com.speaktype.widget.dev`, and opens it through LaunchServices.

The bundle is not cosmetic. macOS attributes TCC requests to the *responsible process*: a bare executable started from a shell inherits responsibility from the terminal app, so it can never hold its own Accessibility grant — you would have to grant it to Terminal/iTerm2/Cursor, which would cover everything else those apps launch. An app opened via LaunchServices is responsible for itself.

1. Run `make dev`.
2. System Settings → Privacy & Security → Accessibility → enable **SpeakType Dev**.
3. Quit & Reopen so the running process re-checks.

Because the bundle identifier and signing identity stay fixed, this grant survives rebuilds — grant it once. Microphone is a separate grant for the same reason: `SpeakType Dev` is a distinct app identity from the installed `/Applications/SpeakType.app`, so both permissions are requested once for the dev build.

`make dev-fast` keeps the old `cargo tauri dev` watch mode for quick UI iteration. It runs a bare executable, so **Accessibility will not work** there — use it only for work that does not need the hotkey or pasting.

Granting Accessibility only for the installed app will leave onboarding stuck on `accessibility=false` during `make dev`.

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
