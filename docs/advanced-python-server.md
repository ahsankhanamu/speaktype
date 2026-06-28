# Advanced: Python transcription server

Optional setup for **CLI power users** who run `speaktype` frequently and want the model to stay loaded between sessions. This is not part of the default install path.

> **Not for widget users.** The macOS desktop app uses a bundled whisper.cpp sidecar (`POST /inference`) and does **not** use this Python server.

The server (`packages/server/whisper_server.py`, installed as `speaktype-server`) keeps a faster-whisper model loaded for CLI clients. It exposes `POST /transcribe` on port **8002** by default.

## Quick start

```bash
source .venv/bin/activate   # after make install or pip install -e ".[all]"
speaktype-server --model base
```

Verify:

```bash
curl http://localhost:8002/health
```

Run the CLI against it:

```bash
speaktype --api http://localhost:8002/transcribe
```

Or use `make server` from the repo root (expects `.venv`).

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--model` | `base` | `tiny`, `base`, `small`, `medium`, `large-v3` |
| `--device` | `auto` | `cpu`, `cuda`, or `auto` |
| `--compute` | `auto` | `auto`, `float16`, `int8` |
| `--port` | `8002` | Listen port |
| `--host` | `127.0.0.1` | Bind address (use `0.0.0.0` only on trusted networks) |

### GPU (NVIDIA)

```bash
pip install -e ".[gpu]"
speaktype-server --device cuda --compute float16
```

## API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Status and config |
| `/transcribe` | POST | Multipart: `file`, optional `language`, `model` |
| `/docs` | GET | Swagger UI |

## Model sizes

| Model | Download | RAM |
|-------|----------|-----|
| tiny | ~75MB | ~1GB |
| base | ~150MB | ~1GB |
| small | ~500MB | ~2GB |
| medium | ~1.5GB | ~5GB |
| large-v3 | ~3GB | ~10GB |

Models download on first use and are cached locally.

## Security

The server has **no authentication**. Default binding is localhost. Do not expose to the internet without a reverse proxy and auth. See [security.md](security.md).

## Systemd (Linux, optional)

```ini
[Unit]
Description=SpeakType Whisper Server
After=network.target

[Service]
Type=simple
User=YOUR_USER
WorkingDirectory=/path/to/speaktype
ExecStart=/path/to/speaktype/.venv/bin/speaktype-server --model base
Restart=on-failure

[Install]
WantedBy=default.target
```

Enable with `systemctl --user enable --now speaktype-server.service`.

## Docker (contributors)

`Dockerfile`, `Dockerfile.gpu`, and `docker-compose.yml` in `packages/server/` are for contributors who want a containerized server setup. They are not a primary install path — use in-process `speaktype` or the Python server above for daily CLI use.
