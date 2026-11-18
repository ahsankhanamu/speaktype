# Scribe — session-based multimodal capture

**Experimental feature (macOS widget, branch `feat/scribe`).** Scribe turns a working session — your voice dictation, screenshots, and copied text — into a single document you can export or have an LLM restructure.

No separate install: Scribe lives in **Settings → Scribe** as a seventh tab.

## How it works

1. **Start a session** — give it a name and choose a persona template (General, Thesis, QA Test Plan, Blog Post, Meeting Notes).
2. **Work normally** — SpeakType keeps capturing:
   - **Dictation** — every transcription is appended to the active session automatically (your paste is filtered out).
   - **Copied text** — the clipboard monitor records text you copy while a session is live (toggleable, on by default).
   - **Screenshots** — press the screenshot hotkey (default `Super+Option+S`) to open the native region/window picker; Apple Vision OCR extracts text and stores it with the image.
3. **Stop the session** when you're done.
4. **Export** the session as a **`.md`** or **`.docx`** document to `~/Documents/SpeakType/Scribe/{name}_{date}.md|.docx`, or
5. **Generate a document** — set an OpenAI-compatible endpoint in Scribe settings and the LLM restructures your captures using the session's template.

## Recording with the screen

Screenshots need **Screen Recording** permission (Settings → Privacy & Security), granted once per install. Without it, dictation and clipboard capture still work — screenshots are simply unavailable. The `scribe_permissions` status and a warning banner in the Scribe tab tell you when it's missing.

## Privacy

Everything captures **locally** (SQLite at `~/Library/Application Support/.../scribe.db`, screenshots beside it, FTS search index). No LLM request is sent unless you configure a base URL + API key in Scribe settings or enable the local llama.cpp server; leaving everything empty falls back to the plain local markdown export.

## Settings

| Setting | Default | Meaning |
|---|---|---|
| Document template | `general` | Persona used for new sessions and LLM restructure |
| Screenshot hotkey | `Super+Option+S` | Global shortcut to snapshot + OCR into the live session |
| Screenshot cap | `50` | Max screenshots per session before captures are refused |
| Capture copied text | on | Clipboard monitoring into the active session |
| LLM base URL / key / model | empty | OpenAI-compatible endpoint for document generation |
| Use local llama.cpp | off | Prefer `llama-server` on PATH over the API endpoint above |
| GGUF model path | empty | Model served by the local `llama-server` |
| Context size | `4096` | `--ctx-size` for the local server |

## Implementation notes

- Sessions store events timestamp-ordered in SQLite (FTS5 search across transcripts, OCR, and copies).
- The clipboard monitor deduplicates repeated copies and skips SpeakType's own paste via an in-memory `is_pasting` flag.
- Screenshots use ScreenCaptureKit (`screencapturekit` crate); OCR uses Apple Vision (`apple-vision` crate).
- The active session survives app restarts — the most recent un-ended session resumes automatically.
- Local generation spawns `llama-server` on a free port (8123–8140), polls `/v1/models` until ready, and emits `scribe:llm_server:crash` if it dies; the server is stopped on app exit. `brew install llama.cpp` provides the binary.

## Limits

- macOS only for screenshots (`/usr/sbin/screencapture -i` + Vision).
- Screenshots use macOS's native region/window picker (the system crosshair UI), so users grab exactly what they want.
- DOCX export embeds the screenshot images and their OCR text; markdown export copies PNGs into a sibling `_images` folder so the `.md` stays portable.
- LLM assembly uses a non-streaming request (streaming SSE planned).
- Local LLM requires the user to install `llama.cpp` (`llama-server`), and model loading time can exceed the 120 s startup timeout for large GGUFs.