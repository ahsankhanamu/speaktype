# Privacy

SpeakType is built for local, private voice typing. Here is exactly what happens to your data.

## Summary

**No accounts. No telemetry. No cloud — by default.**

Audio is processed on your machine. Transcription never leaves your device unless you explicitly configure an external API.

## What stays on your device

| Data | Where it goes |
|------|---------------|
| Microphone audio | Processed locally by whisper.cpp (widget) or faster-whisper (CLI/server) |
| Transcription text | Pasted into your focused app; optional local history in the widget |
| Whisper models | Downloaded once to local cache / app storage |

## What SpeakType does *not* collect

- No analytics or usage tracking
- No crash reporting to third parties
- No account system or login
- No automatic cloud upload of audio or transcripts

## Optional API mode (CLI only)

If you pass `--api URL` to the CLI, audio is sent to **your chosen endpoint** (e.g. Groq, a self-hosted server, or OpenAI-compatible API). That is entirely under your control — SpeakType does not provide or operate that service.

Review the privacy policy of any third-party API you use.

## Open source

SpeakType is MIT-licensed. You can inspect every line that touches audio, network, or storage:

- Widget: `apps/widget/`
- CLI: `packages/python/cli/speaktype.py`
- Server: `packages/python/server/whisper_server.py`

## Network use

Normal operation may contact:

- **Hugging Face** — to download Whisper model weights (widget and CLI)
- **Your configured API** — only if you enable `--api` mode

The widget's bundled whisper.cpp sidecar listens on `127.0.0.1` only.

## Questions

Open a GitHub issue if you have privacy concerns or find behavior that contradicts this document.
