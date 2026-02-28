# Security Notes

## What's been hardened

### Server (server/whisper_server.py)

- **Non-root container user**: The Docker image runs as `whisper` (UID 1000), not root
- **Upload size limit**: 50MB max per file upload, prevents memory exhaustion DoS
- **Model name validation**: Only whitelisted model names accepted (`tiny`, `base`, `small`, `medium`, `large-v3`), prevents path traversal via model parameter
- **Empty file rejection**: Returns 400 on empty uploads
- **Error message sanitization**: Internal errors don't leak stack traces to clients
- **Localhost binding by default**: Server binds to `127.0.0.1` when run natively (Docker overrides to `0.0.0.0` inside container, exposed only on mapped port)
- **Temp file cleanup**: Uploaded audio files are cleaned up in `finally` blocks

### Container

- **Minimal base image**: `python:3.11-slim` — no unnecessary packages
- **.dockerignore**: Prevents source code, git history, and config from leaking into the image
- **No secrets in image**: API keys, `.env` files excluded from build context

## Known Limitations

- **No authentication**: The transcription API has no auth. If you expose it beyond localhost, add a reverse proxy with auth (nginx + basic auth, or Caddy)
- **No TLS**: The server runs plain HTTP. For remote access, terminate TLS at a reverse proxy
- **No rate limiting**: Consider adding rate limiting if exposing to untrusted networks

## Recommendations

1. **Keep it local**: The default `localhost` binding is the safest. Only expose via Docker port mapping to your host
2. **Don't expose to the internet** without a reverse proxy + auth
3. **Use read-only filesystem** if paranoid: `docker run --read-only --tmpdir /tmp ...`
4. **Network isolation**: In docker-compose, you can add a custom network to isolate the server from other containers
