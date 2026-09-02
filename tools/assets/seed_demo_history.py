#!/usr/bin/env python3
"""Seed SpeakType demo history for the History-tab screenshot.

Four entries, newest-first (top of History → bottom):
  1 line, 2 lines, 5 lines, 6 lines.

Usage:
    python3 scripts/assets/seed_demo_history.py
    python3 scripts/assets/seed_demo_history.py --append
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timedelta
from pathlib import Path

# Newest at index 0 — matches History UI (top of screenshot first)
DEMO_HISTORY = [
    "Local transcription on your machine — fast, private, no cloud required.",
    (
        "Press your hotkey, speak, and press again — your text appears in the focused "
        "window. Every transcription is saved in History so you can copy or review it later."
    ),
    (
        "It works everywhere you already work. Browsers, IDEs, Slack, email, and plain "
        "text fields all accept paste — so SpeakType fits into your normal workflow. "
        "It picks the right paste shortcut for terminals versus other apps automatically. "
        "You can start dictating in one window and switch focus before you finish. "
        "Your words still land where you intended."
    ),
    (
        "When you are drafting a long message, you do not have to stop and type every "
        "word yourself. Hold your hotkey, explain what you mean in your own voice, and "
        "SpeakType turns it into text you can paste anywhere. You can clarify as you go, "
        "backtrack, or add detail without losing flow. That is the whole idea: speak "
        "naturally, edit later if you need to, and keep moving. Terminals, docs, and "
        "chat all work the same way."
    ),
]


def history_path() -> Path:
    if sys.platform == "darwin":
        base = Path.home() / "Library" / "Application Support" / "speaktype"
    elif sys.platform == "win32":
        base = Path.home() / "AppData" / "Roaming" / "speaktype"
    else:
        base = Path.home() / ".config" / "speaktype"
    return base / "history.json"


def build_entries() -> list[dict[str, str]]:
    now = datetime.now()
    entries = []
    for i, text in enumerate(DEMO_HISTORY):
        ts = now - timedelta(minutes=i * 4)
        entries.append({"text": text, "timestamp": ts.strftime("%Y-%m-%dT%H:%M:%S")})
    return entries


def main() -> int:
    parser = argparse.ArgumentParser(description="Seed SpeakType demo history")
    parser.add_argument("--append", action="store_true", help="Append to existing history")
    args = parser.parse_args()

    path = history_path()
    path.parent.mkdir(parents=True, exist_ok=True)

    new_entries = build_entries()
    if args.append and path.exists():
        data = json.loads(path.read_text())
        data["entries"] = new_entries + data.get("entries", [])
    else:
        data = {"entries": new_entries}

    path.write_text(json.dumps(data, indent=2) + "\n")
    print(f"Wrote {len(new_entries)} demo entries to {path}")
    print("Top of History: 1 line → then 2 → 5 → 6 lines at bottom.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
