# Docker Setup Guide

TalkType has two components:
- **server/whisper_server.py** — The transcription API server (runs in Docker)
- **client/talktype.py** — The desktop client (runs on your host, needs mic/keyboard/display)

The Docker setup containerizes the server, so you get a clean, reproducible transcription backend with one command.

## Quick Start (CPU)

```bash
# Start the whisper server
cd server && docker compose up -d

# Verify it's running
curl http://localhost:8003/health

# Run the client on your host (from project root)
cd .. && python client/talktype.py --api http://localhost:8003/transcribe
```

That's it. Press F9, speak, press F9 — text appears.

## Quick Start (GPU / NVIDIA)

Edit `server/docker-compose.yml` — comment out the `whisper-server` service and uncomment `whisper-server-gpu`, then:

```bash
cd server && docker compose up -d
```

Requires [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html).

Or build directly:

```bash
docker build -f server/Dockerfile.gpu -t talktype-whisper:gpu .
docker run --gpus all -p 8002:8002 talktype-whisper:gpu
```

## Configuration

All settings via environment variables (in `server/docker-compose.yml` or `.env` file):

| Variable | Default | Description |
|----------|---------|-------------|
| `WHISPER_MODEL` | `base` | Model size: `tiny`, `base`, `small`, `medium`, `large-v3` |
| `WHISPER_DEVICE` | `cpu` | `cpu` or `cuda` |
| `WHISPER_COMPUTE` | `auto` | `auto`, `float16`, `int8` |

Example `.env` file:

```env
WHISPER_MODEL=small
WHISPER_DEVICE=cpu
```

## Model Persistence

Downloaded models are stored in a Docker volume (`whisper-models`). They persist across container restarts, so you only download once.

To clear cached models:

```bash
docker volume rm talktype_whisper-models
```

## Model Size Reference

| Model | Download | RAM Usage | Accuracy |
|-------|----------|-----------|----------|
| tiny | ~75MB | ~1GB | Basic |
| base | ~150MB | ~1GB | Good |
| small | ~500MB | ~2GB | Better |
| medium | ~1.5GB | ~5GB | Great |
| large-v3 | ~3GB | ~10GB | Best |

For most use cases, `base` is the sweet spot. Use `small` if you need better accuracy.

## Useful Commands

```bash
# View logs (from server/ directory)
cd server && docker compose logs -f whisper-server

# Restart with a different model
WHISPER_MODEL=small docker compose up -d

# Stop
docker compose down

# Rebuild after code changes
docker compose build && docker compose up -d

# Test transcription with curl
curl -X POST http://localhost:8003/transcribe \
  -F "file=@recording.wav" \
  -F "language=en"
```

## Troubleshooting

**Container won't start / health check failing:**
- First run takes 30-60s to download the model. Check logs: `docker compose logs -f`

**Port 8003 already in use:**
- Another service is on that port. Change the host port in `server/docker-compose.yml`
- Then update your `--api` URL to match

**Out of memory:**
- Use a smaller model: `WHISPER_MODEL=tiny docker compose up -d`
