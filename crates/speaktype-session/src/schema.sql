-- Schema v2. The `scribe_*` table names are kept from v1 (spec §12);
-- databases below user_version 2 are dropped and recreated before this runs.
PRAGMA user_version = 2;

CREATE TABLE IF NOT EXISTS scribe_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    template TEXT NOT NULL DEFAULT 'general',
    created_at TEXT NOT NULL,
    ended_at TEXT,
    target TEXT NOT NULL DEFAULT 'codingAgent',
    preface TEXT,
    bundle_path TEXT
);

CREATE TABLE IF NOT EXISTS scribe_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES scribe_sessions(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    t_start_ms INTEGER NOT NULL,
    t_end_ms INTEGER,
    ord REAL NOT NULL,
    content TEXT,
    caption TEXT,
    image_path TEXT,
    source_app TEXT,
    metadata TEXT,
    deleted INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_scribe_events_session ON scribe_events(session_id, ord);

CREATE VIRTUAL TABLE IF NOT EXISTS scribe_events_fts USING fts5(content, caption, ocr);
