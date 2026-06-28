# Contributing to SpeakType

Thanks for your interest in contributing. SpeakType is a monorepo with a macOS desktop widget (Tauri), an optional Python CLI, and an optional Python transcription server.

## Development setup

```bash
git clone https://github.com/ahsankhanamu/speaktype.git && cd speaktype
make install          # Python venv + CLI/server deps (optional)
make dev              # Run Tauri widget in dev mode (macOS)
```

See [docs/build-widget-from-source.md](docs/build-widget-from-source.md) for widget build prerequisites (Rust, cmake, sidecar). See [docs/getting-started.md](docs/getting-started.md) to choose which component to work on.

## What to test

| Change area | How to verify |
|-------------|---------------|
| Widget UI / Rust | `make dev` — hotkey, recording, paste, settings |
| CLI | `speaktype` after `make install` — F9 flow in terminal + browser |
| Python server | `make server` + `speaktype --api http://localhost:8002/transcribe` |

## Pull requests

1. Fork and create a branch from `main`
2. Keep changes focused — one feature or fix per PR
3. Update docs if you change install paths, flags, or permissions
4. Describe how you tested (platform, widget vs CLI)

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). Be respectful and constructive.

## Security

Report vulnerabilities privately — see [SECURITY.md](SECURITY.md).
