use crate::logging::log_message;
use crate::paths;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Session row for the list view (no events).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScribeSession {
    pub id: i64,
    pub name: String,
    pub template: String,
    pub created_at: String,
    pub ended_at: Option<String>,
}

/// A single capture event inside a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScribeEvent {
    pub id: i64,
    pub session_id: i64,
    pub event_type: String,
    pub timestamp: String,
    pub content: Option<String>,
    pub image_path: Option<String>,
    pub source_app: Option<String>,
    pub metadata: Option<String>,
}

/// Shared handle to the scribe SQLite store + active-session pointer.
#[derive(Debug)]
pub struct ScribeStore {
    conn: Mutex<Connection>,
    pub active_session: Mutex<Option<i64>>,
}

fn now_str() -> String {
    chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%S%.3f")
        .to_string()
}

fn session_from_row(row: &Row<'_>) -> Result<ScribeSession, rusqlite::Error> {
    Ok(ScribeSession {
        id: row.get(0)?,
        name: row.get(1)?,
        template: row.get(2)?,
        created_at: row.get(3)?,
        ended_at: row.get(4)?,
    })
}

fn event_from_row(row: &Row<'_>) -> Result<ScribeEvent, rusqlite::Error> {
    Ok(ScribeEvent {
        id: row.get(0)?,
        session_id: row.get(1)?,
        event_type: row.get(2)?,
        timestamp: row.get(3)?,
        content: row.get(4)?,
        image_path: row.get(5)?,
        source_app: row.get(6)?,
        metadata: row.get(7)?,
    })
}

impl ScribeStore {
    /// Test-only constructor around an already-created connection.
    #[cfg(test)]
    pub(crate) fn from_conn(conn: Connection) -> Self {
        ScribeStore {
            conn: Mutex::new(conn),
            active_session: Mutex::new(None),
        }
    }

    pub fn init() -> Result<Arc<Self>, String> {
        let dir = paths::config_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let db_path = dir.join("scribe.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS scribe_sessions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 template TEXT DEFAULT 'general',
                 created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                 ended_at DATETIME
             );
             CREATE TABLE IF NOT EXISTS scribe_events (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 session_id INTEGER NOT NULL REFERENCES scribe_sessions(id) ON DELETE CASCADE,
                 event_type TEXT NOT NULL,
                 timestamp DATETIME NOT NULL,
                 content TEXT,
                 image_path TEXT,
                 source_app TEXT,
                 metadata TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_scribe_events_session
                 ON scribe_events(session_id, timestamp);
             CREATE VIRTUAL TABLE IF NOT EXISTS scribe_events_fts
                 USING fts5(content, source_app, content='scribe_events', content_rowid='id');",
        )
        .map_err(|e| e.to_string())?;
        log_message(&format!("[scribe] Store initialized at {:?}", db_path));
        let store = ScribeStore {
            conn: Mutex::new(conn),
            active_session: Mutex::new(None),
        };
        // Sessions are event-driven and only go "live" on the first capture of
        // this run. An open session left behind by a previous run / crash is
        // stale — close it so the widget never reports RECORDING before any
        // recording happens.
        {
            let conn = store.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE scribe_sessions SET ended_at = ?1 WHERE ended_at IS NULL",
                rusqlite::params![now_str()],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(Arc::new(store))
    }

    pub fn scribe_dir() -> PathBuf {
        crate::paths::config_dir().join("scribe")
    }

    pub fn session_dir(id: i64) -> PathBuf {
        Self::scribe_dir().join(id.to_string())
    }

    pub fn start_session(&self, name: &str, template: &str) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO scribe_sessions (name, template, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, template, now_str()],
        )
        .map_err(|e| e.to_string())?;
        let id = conn.last_insert_rowid();
        let dir = Self::session_dir(id);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        log_message(&format!("[scribe] Session {} started: {:?}", id, name));
        Ok(id)
    }

    pub fn stop_session(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE scribe_sessions SET ended_at = ?2 WHERE id = ?1 AND ended_at IS NULL",
            rusqlite::params![id, now_str()],
        )
        .map_err(|e| e.to_string())?;
        log_message(&format!("[scribe] Session {} stopped", id));
        Ok(())
    }

    /// Return the active session, auto-creating a timestamped one on first
    /// capture. Lets the scribe toggle behave as a pure mode switch: the first
    /// dictated transcript / screenshot / clipboard copy materialises a session.
    pub fn ensure_active_session(&self) -> Result<i64, String> {
        let mut active = self.active_session.lock().map_err(|e| e.to_string())?;
        if let Some(id) = *active {
            return Ok(id);
        }
        // Close any stale open session (e.g. one left over from a crash) so
        // the DB keeps at most one open session at a time.
        {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE scribe_sessions SET ended_at = ?1 WHERE ended_at IS NULL",
                rusqlite::params![now_str()],
            )
            .map_err(|e| e.to_string())?;
        }
        let now = chrono::Local::now();
        let stamp = format!(
            "{} {}",
            now.format("%b %e").to_string().trim(),
            now.format("%H:%M")
        );
        let id = self.start_session(&format!("Scribe session {}", stamp), "auto")?;
        *active = Some(id);
        drop(active);
        log_message(&format!(
            "[scribe] Auto-started session {} from first capture",
            id
        ));
        Ok(id)
    }

    pub fn delete_session(&self, id: i64) -> Result<(), String> {
        {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "DELETE FROM scribe_sessions WHERE id = ?1",
                rusqlite::params![id],
            )
            .map_err(|e| e.to_string())?;
        }
        let dir = Self::session_dir(id);
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
        log_message(&format!("[scribe] Session {} deleted", id));
        Ok(())
    }

    pub fn get_sessions(&self) -> Result<Vec<ScribeSession>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, template, created_at, ended_at
                 FROM scribe_sessions ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], session_from_row)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn get_session(&self, id: i64) -> Result<Option<ScribeSession>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, template, created_at, ended_at
                 FROM scribe_sessions WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([id], session_from_row)
            .map_err(|e| e.to_string())?;
        let mut out = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(out.pop())
    }

    /// Append an event to a session. Returns the full stored event.
    pub fn add_event(
        &self,
        session_id: i64,
        event_type: &str,
        content: Option<&str>,
        image_path: Option<&str>,
        source_app: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<ScribeEvent, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO scribe_events
                 (session_id, event_type, timestamp, content, image_path, source_app, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                session_id,
                event_type,
                now_str(),
                content,
                image_path,
                source_app,
                metadata
            ],
        )
        .map_err(|e| e.to_string())?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO scribe_events_fts(rowid, content, source_app) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, content.unwrap_or(""), source_app.unwrap_or("")],
        )
        .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, event_type, timestamp, content, image_path, source_app, metadata
                 FROM scribe_events WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map([id], event_from_row)
            .map_err(|e| e.to_string())?;
        let event = rows
            .next()
            .transpose()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Event not found after insert".to_string())?;
        Ok(event)
    }

    pub fn get_events(&self, session_id: i64) -> Result<Vec<ScribeEvent>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, event_type, timestamp, content, image_path, source_app, metadata
                 FROM scribe_events WHERE session_id = ?1 ORDER BY timestamp ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([session_id], event_from_row)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn count_events_of_type(&self, session_id: i64, event_type: &str) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT COUNT(*) FROM scribe_events WHERE session_id = ?1 AND event_type = ?2",
            rusqlite::params![session_id, event_type],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
    }

    /// True when `text` matches a recent dictation transcript in the session.
    /// Used by the clipboard monitor so a user copying the text they just
    /// dictated is not captured again as a duplicate "copied" event.
    pub fn clipboard_matches_recent_transcript(
        &self,
        session_id: i64,
        text: &str,
    ) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT content FROM scribe_events
                 WHERE session_id = ?1 AND event_type = 'transcript'
                 ORDER BY id DESC LIMIT 5",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![session_id], |row| {
                row.get::<_, Option<String>>(0)
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            if let Ok(Some(content)) = row {
                if content == text {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn search_sessions(&self, query: &str) -> Result<Vec<ScribeEvent>, String> {
        // Quote each whitespace-separated term so multi-word queries become
        // "term1" AND "term2" instead of a single phrase.
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|t| format!("\"{}\"", t.replace('"', "")))
            .collect();
        let q = terms.join(" AND ");
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.session_id, e.event_type, e.timestamp, e.content,
                        e.image_path, e.source_app, e.metadata
                 FROM scribe_events_fts f
                 JOIN scribe_events e ON e.id = f.rowid
                 WHERE scribe_events_fts MATCH ?1
                 ORDER BY e.timestamp DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([&q], event_from_row)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Rebuild the FTS index (kept in sync by triggers-on-insert being absent;
    /// we rebuild after every insert via add_event for correctness).
    #[allow(dead_code)]
    pub fn rebuild_fts(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("INSERT INTO scribe_events_fts(scribe_events_fts) VALUES('rebuild');")
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (ScribeStore, PathBuf) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "scribe_test_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("scribe.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS scribe_sessions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 template TEXT DEFAULT 'general',
                 created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                 ended_at DATETIME
             );
             CREATE TABLE IF NOT EXISTS scribe_events (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 session_id INTEGER NOT NULL REFERENCES scribe_sessions(id) ON DELETE CASCADE,
                 event_type TEXT NOT NULL,
                 timestamp DATETIME NOT NULL,
                 content TEXT,
                 image_path TEXT,
                 source_app TEXT,
                 metadata TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_scribe_events_session
                 ON scribe_events(session_id, timestamp);
             CREATE VIRTUAL TABLE IF NOT EXISTS scribe_events_fts
                 USING fts5(content, source_app, content='scribe_events', content_rowid='id');",
        )
        .unwrap();
        let store = ScribeStore::from_conn(conn);
        (store, dir)
    }

    #[test]
    fn start_and_stop_session() {
        let (store, dir) = temp_store();
        let id = store.start_session("Research", "general").unwrap();
        assert!(id > 0);
        let s = store.get_session(id).unwrap().unwrap();
        assert_eq!(s.name, "Research");
        assert_eq!(s.template, "general");
        assert!(s.ended_at.is_none());
        store.stop_session(id).unwrap();
        let s = store.get_session(id).unwrap().unwrap();
        assert!(s.ended_at.is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_and_list_events() {
        let (store, dir) = temp_store();
        let id = store.start_session("S", "general").unwrap();
        let ev = store
            .add_event(
                id,
                "transcript",
                Some("hello world"),
                None,
                Some("Chrome"),
                None,
            )
            .unwrap();
        assert!(ev.id > 0);
        assert_eq!(ev.content.as_deref(), Some("hello world"));
        let events = store.get_events(id).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].source_app.as_deref(), Some("Chrome"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_events() {
        let (store, dir) = temp_store();
        let id = store.start_session("S", "general").unwrap();
        store
            .add_event(
                id,
                "transcript",
                Some("quantum entanglement"),
                None,
                None,
                None,
            )
            .unwrap();
        store
            .add_event(
                id,
                "clipboard",
                Some("some notes about physics"),
                None,
                None,
                None,
            )
            .unwrap();
        store.rebuild_fts().unwrap();
        let hits = store.search_sessions("quantum").unwrap();
        assert_eq!(hits.len(), 1);
        let both = store.search_sessions("quantum entanglement").unwrap();
        assert_eq!(both.len(), 1, "multi-term AND query matches the same event");
        let none = store.search_sessions("quantum physics").unwrap();
        assert!(
            none.is_empty(),
            "AND query over disjoint terms matches nothing"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn screenshot_count_and_multi_word_search() {
        let (store, dir) = temp_store();
        let id = store.start_session("S", "general").unwrap();
        for i in 0..3 {
            store
                .add_event(
                    id,
                    "screenshot",
                    Some("img"),
                    Some(&format!("s-{i}.png")),
                    None,
                    None,
                )
                .unwrap();
        }
        assert_eq!(store.count_events_of_type(id, "screenshot").unwrap(), 3);
        assert_eq!(store.count_events_of_type(id, "transcript").unwrap(), 0);
        let _ = fs::remove_dir_all(&dir);
    }
}
