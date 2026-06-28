# Security notes

## Python server (`packages/server/whisper_server.py`)

- **Upload size limit:** 50MB max per file — reduces memory exhaustion risk
- **Model name validation:** Only whitelisted names (`tiny`, `base`, `small`, `medium`, `large-v3`)
- **Empty file rejection:** Returns 400 on empty uploads
- **Error sanitization:** Internal errors do not leak stack traces to clients
- **Localhost binding by default:** Binds to `127.0.0.1` when run natively
- **Temp file cleanup:** Uploaded audio cleaned up in `finally` blocks

## Widget sidecar

- Listens on **127.0.0.1** only — not reachable from other machines by default
- Bundled in the macOS app; no separate network exposure

## Known limitations

- **No authentication:** The transcription API has no auth. If you bind beyond localhost, add a reverse proxy with auth (nginx, Caddy, etc.)
- **No TLS:** Plain HTTP — terminate TLS at a reverse proxy for remote access
- **No rate limiting:** Add rate limiting if exposed to untrusted networks

## Recommendations

1. **Keep it local** — default `127.0.0.1` is the safest configuration
2. **Do not expose to the internet** without reverse proxy + authentication

## Reporting issues

See [SECURITY.md](../SECURITY.md) for vulnerability reporting.
