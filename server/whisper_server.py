#!/usr/bin/env python3
"""
Whisper API Server - Local transcription server for SpeakType.

Run this once, keep it running, and SpeakType connects to it.
Faster startup since the model stays loaded in memory.

Usage:
    python whisper_server.py                    # Default: base model, auto device
    python whisper_server.py --model small      # Use small model
    python whisper_server.py --port 8080        # Different port
    CUDA_VISIBLE_DEVICES=0 python whisper_server.py  # Specific GPU

Then run SpeakType with:
    python speaktype.py --api http://localhost:8002/transcribe
"""

import argparse
import logging
import os
import tempfile
import time

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("whisper_server")

# Auto-configure CUDA library paths if nvidia packages are installed
def _setup_cuda_paths():
    try:
        import nvidia.cublas.lib
        import nvidia.cudnn.lib
        paths = [nvidia.cublas.lib.__path__[0], nvidia.cudnn.lib.__path__[0]]
        existing = os.environ.get("LD_LIBRARY_PATH", "")
        os.environ["LD_LIBRARY_PATH"] = ":".join(paths + ([existing] if existing else []))
        logger.info("CUDA libraries found: cublas + cudnn")
    except ImportError:
        logger.info("CUDA libraries not found, using CPU")

_setup_cuda_paths()
from typing import Dict, Tuple

from fastapi import FastAPI, File, Form, HTTPException, UploadFile
from faster_whisper import WhisperModel
import uvicorn

# Maximum upload size: 50MB (prevents DoS via large file uploads)
MAX_UPLOAD_BYTES = 50 * 1024 * 1024

# Allowed model names to prevent path traversal via model parameter
ALLOWED_MODELS = {"tiny", "base", "small", "medium", "large-v3"}

# === Configuration ===
DEFAULT_MODEL = os.getenv("WHISPER_MODEL", "base")
DEFAULT_DEVICE = os.getenv("WHISPER_DEVICE", "cuda")  # "cuda", "cpu", or "auto"
DEFAULT_COMPUTE = os.getenv("WHISPER_COMPUTE", "float16")  # "float16", "int8", "auto"

# === Model Cache ===
_models: Dict[Tuple[str, str, str], WhisperModel] = {}


def get_model(name: str, device: str, compute: str) -> WhisperModel:
    """Get or load a Whisper model (cached)."""
    key = (name, device, compute)
    if key not in _models:
        logger.info("Loading model: %s (device=%s, compute=%s)...", name, device, compute)
        t0 = time.time()
        _models[key] = WhisperModel(name, device=device, compute_type=compute)
        logger.info("Model loaded in %.1fs", time.time() - t0)
    return _models[key]


# === FastAPI App ===
app = FastAPI(
    title="Whisper API",
    description="Local Whisper transcription server for SpeakType",
    version="1.0.0"
)


@app.get("/health")
def health():
    """Health check endpoint."""
    logger.debug("Health check")
    return {
        "status": "ok",
        "default_model": DEFAULT_MODEL,
        "device": DEFAULT_DEVICE,
        "compute": DEFAULT_COMPUTE
    }


@app.post("/transcribe")
async def transcribe(
    file: UploadFile = File(...),
    language: str = Form(None),
    model: str = Form(None),
):
    """
    Transcribe an audio file.

    - **file**: Audio file (WAV, MP3, etc.)
    - **language**: Language code (e.g., "en", "es"). Auto-detect if not specified.
    - **model**: Model to use (tiny, base, small, medium, large-v3). Uses default if not specified.
    """
    m = model or DEFAULT_MODEL

    # Validate model name to prevent path traversal
    if m not in ALLOWED_MODELS:
        logger.warning("Rejected invalid model: %s", m)
        raise HTTPException(400, f"Invalid model: {m}. Allowed: {', '.join(sorted(ALLOWED_MODELS))}")

    # Read and validate upload size
    content = await file.read()
    file_size_kb = len(content) / 1024
    if len(content) > MAX_UPLOAD_BYTES:
        logger.warning("Rejected oversized upload: %.1fMB", len(content) / (1024 * 1024))
        raise HTTPException(413, f"File too large. Maximum size: {MAX_UPLOAD_BYTES // (1024*1024)}MB")
    if len(content) == 0:
        logger.warning("Rejected empty file upload")
        raise HTTPException(400, "Empty file uploaded")

    logger.info(
        "Transcribe request: file=%s size=%.1fKB model=%s language=%s",
        file.filename, file_size_kb, m, language or "auto",
    )

    whisper = get_model(m, DEFAULT_DEVICE, DEFAULT_COMPUTE)

    # Save uploaded file to temp
    tmp = tempfile.NamedTemporaryFile(delete=False, suffix=".wav")
    try:
        tmp.write(content)
        tmp.close()

        # Transcribe
        t0 = time.time()
        segments, info = whisper.transcribe(tmp.name, language=language)
        segments_list = [{"start": s.start, "end": s.end, "text": s.text} for s in segments]
        text = "".join(s["text"] for s in segments_list)
        elapsed = time.time() - t0

        audio_duration = segments_list[-1]["end"] if segments_list else 0
        logger.info(
            "Transcription complete: %.1fs audio in %.2fs (%.1fx realtime) lang=%s(%d%%) text=%s",
            audio_duration, elapsed,
            audio_duration / elapsed if elapsed > 0 else 0,
            info.language, int(info.language_probability * 100),
            repr(text[:100]) if text else "(empty)",
        )

        return {
            "text": text,
            "language": info.language,
            "language_probability": info.language_probability,
            "model": m,
            "segments": segments_list
        }
    except Exception as e:
        logger.exception("Transcription failed: %s", e)
        raise HTTPException(500, "Transcription failed")
    finally:
        try:
            os.unlink(tmp.name)
        except OSError:
            pass


def main():
    global DEFAULT_MODEL, DEFAULT_DEVICE, DEFAULT_COMPUTE

    parser = argparse.ArgumentParser(description="Whisper API Server")
    parser.add_argument("--model", "-m", default=DEFAULT_MODEL,
                        help=f"Whisper model (default: {DEFAULT_MODEL})")
    parser.add_argument("--device", "-d", default=DEFAULT_DEVICE,
                        help=f"Device: auto, cuda, cpu (default: {DEFAULT_DEVICE})")
    parser.add_argument("--compute", "-c", default=DEFAULT_COMPUTE,
                        help=f"Compute type: auto, float16, int8 (default: {DEFAULT_COMPUTE})")
    parser.add_argument("--port", "-p", type=int, default=int(os.getenv("WHISPER_PORT", "8002")),
                        help="Port to run on (default: 8002, or WHISPER_PORT env)")
    parser.add_argument("--host", default=os.getenv("WHISPER_HOST", "127.0.0.1"),
                        help="Host to bind to (default: 127.0.0.1, or WHISPER_HOST env)")
    args = parser.parse_args()

    # Update defaults from args
    DEFAULT_MODEL = args.model
    DEFAULT_DEVICE = args.device
    DEFAULT_COMPUTE = args.compute

    # Pre-load model
    logger.info("Whisper API Server starting on http://%s:%d", args.host, args.port)
    logger.info("Config: model=%s device=%s compute=%s", DEFAULT_MODEL, DEFAULT_DEVICE, DEFAULT_COMPUTE)
    get_model(DEFAULT_MODEL, DEFAULT_DEVICE, DEFAULT_COMPUTE)
    logger.info("Server ready — accepting requests")

    uvicorn.run(app, host=args.host, port=args.port, log_level="info")


if __name__ == "__main__":
    main()
