# =============================================================================
# SpeakType Whisper Server - Docker Image
# =============================================================================
# Runs the whisper_server.py FastAPI transcription service.
# The speaktype.py client runs on your host (needs audio/keyboard/display).
#
# Build:  docker build -t speaktype-whisper .
# Run:    docker run -p 8003:8002 speaktype-whisper
# =============================================================================

FROM python:3.13-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

# Install only the system deps needed for audio processing
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libsndfile1 \
        ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd --gid 1000 whisper && \
    useradd --uid 1000 --gid whisper --create-home whisper

WORKDIR /app

# Copy project files needed for install
COPY pyproject.toml ./
COPY server/whisper_server.py ./

# Install base dependencies (server) from pyproject.toml
RUN pip install --no-cache-dir .

# Model cache directory - mount a volume here to persist models across restarts
RUN mkdir -p /home/whisper/.cache && chown -R whisper:whisper /home/whisper/.cache
VOLUME ["/home/whisper/.cache"]

# Switch to non-root user
USER whisper

# Default configuration via environment variables
ENV WHISPER_MODEL=base \
    WHISPER_DEVICE=cpu \
    WHISPER_COMPUTE=auto \
    WHISPER_HOST=0.0.0.0 \
    WHISPER_PORT=8002

EXPOSE 8002

HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8002/health')" || exit 1

ENTRYPOINT ["python", "whisper_server.py"]
CMD ["--host", "0.0.0.0", "--port", "8002", "--device", "cpu"]
