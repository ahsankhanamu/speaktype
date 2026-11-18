use crate::types::{
    Item, ItemKind, ItemPatch, NewItem, Session, SessionDetail, SessionSummary, Target,
};
use chrono::{DateTime, Local};
use rusqlite::{params, params_from_iter, Connection, Row};
use serde_json::Value;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("session {0} not found")]
    SessionNotFound(i64),
    #[error("item {0} not found")]
    ItemNotFound(i64),
    #[error("only speech items can be split")]
    NotSpeech,
    #[error("cannot split a {1}-char text at index {0}")]
    BadSplitIndex(usize, usize),
    #[error("corrupt value in store: {0}")]
    Corrupt(String),
}

/// `created_at` / `ended_at` keep the v1 string columns (spec §12: extend,
/// don't rename) but carry the UTC offset so the ms conversion is exact.
const TIME_FMT: &str = "%Y-%m-%dT%H:%M:%S%.3f%:z";

fn ms_to_str(ms: i64) -> Result<String, StoreError> {
    let dt = DateTime::from_timestamp_millis(ms)
        .ok_or_else(|| StoreError::Corrupt(format!("timestamp {ms} out of range")))?;
    Ok(dt.with_timezone(&Local).format(TIME_FMT).to_string())
}

fn str_to_ms(s: &str) -> Result<i64, StoreError> {
    DateTime::parse_from_str(s, TIME_FMT)
        .map_err(|_| StoreError::Corrupt(format!("unparseable timestamp {s:?}")))
        .map(|dt| dt.timestamp_millis())
}

fn kind_to_db(kind: &ItemKind) -> &'static str {
    match kind {
        ItemKind::Speech => "transcript",
        ItemKind::Mark => "mark",
        ItemKind::Screenshot => "screenshot",
        ItemKind::Clip => "clipboard",
        ItemKind::Note => "note",
    }
}

fn kind_from_db(s: &str) -> Result<ItemKind, StoreError> {
    match s {
        "transcript" => Ok(ItemKind::Speech),
        "mark" => Ok(ItemKind::Mark),
        "screenshot" => Ok(ItemKind::Screenshot),
        "clipboard" => Ok(ItemKind::Clip),
        "note" => Ok(ItemKind::Note),
        other => Err(StoreError::Corrupt(format!("unknown event_type {other:?}"))),
    }
}

fn target_to_db(target: &Target) -> &'static str {
    match target {
        Target::CodingAgent => "codingAgent",
        Target::Chat => "chat",
        Target::Markdown => "markdown",
    }
}

fn target_from_db(s: &str) -> Result<Target, StoreError> {
    match s {
        "codingAgent" => Ok(Target::CodingAgent),
        "chat" => Ok(Target::Chat),
        "markdown" => Ok(Target::Markdown),
        other => Err(StoreError::Corrupt(format!("unknown target {other:?}"))),
    }
}

fn session_from_row(row: &Row<'_>) -> Result<Session, StoreError> {
    let created_at: String = row.get("created_at")?;
    let ended_at: Option<String> = row.get("ended_at")?;
    Ok(Session {
        id: row.get("id")?,
        name: row.get("name")?,
        started_at_ms: str_to_ms(&created_at)?,
        ended_at_ms: ended_at.as_deref().map(str_to_ms).transpose()?,
        template: row.get("template")?,
        target: target_from_db(&row.get::<_, String>("target")?)?,
        preface: row.get("preface")?,
        bundle_path: row.get("bundle_path")?,
    })
}

fn item_from_row(row: &Row<'_>) -> Result<Item, StoreError> {
    let metadata: Option<String> = row.get("metadata")?;
    Ok(Item {
        id: row.get("id")?,
        session_id: row.get("session_id")?,
        kind: kind_from_db(&row.get::<_, String>("event_type")?)?,
        ord: row.get("ord")?,
        t_start_ms: row.get("t_start_ms")?,
        t_end_ms: row.get("t_end_ms")?,
        text: row.get("content")?,
        caption: row.get("caption")?,
        image_path: row.get("image_path")?,
        source_app: row.get("source_app")?,
        payload: metadata
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?
            .unwrap_or(Value::Null),
    })
}

fn fetch_one<T>(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
    map: impl Fn(&Row<'_>) -> Result<T, StoreError>,
) -> Result<Option<T>, StoreError> {
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query(params)?;
    match rows.next()? {
        Some(row) => Ok(Some(map(row)?)),
        None => Ok(None),
    }
}

fn fetch_all<T>(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
    map: impl Fn(&Row<'_>) -> Result<T, StoreError>,
) -> Result<Vec<T>, StoreError> {
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query(params)?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(map(row)?);
    }
    Ok(out)
}

/// SQLite-backed session store. Every method holds the connection lock across
/// the whole read-modify-write, so multi-step operations (like `add_item`'s
/// ord assignment) are atomic across threads.
pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// Open (creating if needed) a v2 store at `path`. A database below
    /// user_version 2 — an unreleased dev Scribe database — is dropped and
    /// recreated, never migrated (spec §10).
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        Store::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        Store::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, StoreError> {
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 2 {
            conn.execute_batch(
                "DROP TABLE IF EXISTS scribe_events_fts;
                 DROP TABLE IF EXISTS scribe_events;
                 DROP TABLE IF EXISTS scribe_sessions;",
            )?;
        }
        conn.execute_batch(include_str!("schema.sql"))?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    pub fn create_session(
        &self,
        name: &str,
        template: &str,
        now_ms: i64,
    ) -> Result<Session, StoreError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO scribe_sessions (name, template, created_at) VALUES (?1, ?2, ?3)",
            params![name, template, ms_to_str(now_ms)?],
        )?;
        self.session_row(&conn, conn.last_insert_rowid())
    }

    /// Returns the active session, or creates one; `true` when created. Check and
    /// insert happen under one lock so concurrent starts never make two sessions.
    pub fn start_session(
        &self,
        name: &str,
        template: &str,
        now_ms: i64,
    ) -> Result<(Session, bool), StoreError> {
        let conn = self.lock()?;
        let active = fetch_one(
            &conn,
            "SELECT * FROM scribe_sessions WHERE ended_at IS NULL ORDER BY id DESC LIMIT 1",
            [],
            session_from_row,
        )?;
        if let Some(session) = active {
            return Ok((session, false));
        }
        conn.execute(
            "INSERT INTO scribe_sessions (name, template, created_at) VALUES (?1, ?2, ?3)",
            params![name, template, ms_to_str(now_ms)?],
        )?;
        Ok((self.session_row(&conn, conn.last_insert_rowid())?, true))
    }

    pub fn end_session(&self, id: i64, now_ms: i64) -> Result<Session, StoreError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE scribe_sessions SET ended_at = ?2 WHERE id = ?1 AND ended_at IS NULL",
            params![id, ms_to_str(now_ms)?],
        )?;
        self.session_row(&conn, id)
    }

    /// The active session: `ended_at IS NULL`, newest.
    pub fn active_session(&self) -> Result<Option<Session>, StoreError> {
        let conn = self.lock()?;
        fetch_one(
            &conn,
            "SELECT * FROM scribe_sessions WHERE ended_at IS NULL
             ORDER BY id DESC LIMIT 1",
            [],
            session_from_row,
        )
    }

    pub fn add_item(&self, session_id: i64, item: NewItem) -> Result<Item, StoreError> {
        let conn = self.lock()?;
        let ord: f64 = conn.query_row(
            "SELECT COALESCE(MAX(ord), 0) + 1 FROM scribe_events WHERE session_id = ?1",
            params![session_id],
            |r| r.get(0),
        )?;
        let payload_json = serde_json::to_string(&item.payload)?;
        conn.execute(
            "INSERT INTO scribe_events
                 (session_id, event_type, t_start_ms, t_end_ms, ord,
                  content, image_path, source_app, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session_id,
                kind_to_db(&item.kind),
                item.t_start_ms,
                item.t_end_ms,
                ord,
                item.text,
                item.image_path,
                item.source_app,
                payload_json,
            ],
        )?;
        let stored = self.item_row(&conn, conn.last_insert_rowid())?;
        fts_put(&conn, &stored)?;
        Ok(stored)
    }

    pub fn detail(&self, session_id: i64) -> Result<SessionDetail, StoreError> {
        let conn = self.lock()?;
        let session = self.session_row(&conn, session_id)?;
        let items = fetch_all(
            &conn,
            "SELECT * FROM scribe_events
             WHERE session_id = ?1 AND deleted = 0
             ORDER BY ord, id",
            params![session_id],
            item_from_row,
        )?;
        Ok(SessionDetail { session, items })
    }

    pub fn update_item(&self, id: i64, patch: ItemPatch) -> Result<Item, StoreError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE scribe_events
             SET content = COALESCE(?2, content), caption = COALESCE(?3, caption)
             WHERE id = ?1",
            params![id, patch.text, patch.caption],
        )?;
        let item = self.item_row(&conn, id)?;
        fts_put(&conn, &item)?;
        Ok(item)
    }

    /// Soft delete, so Timeline deletes are undoable.
    pub fn delete_items(&self, ids: &[i64]) -> Result<(), StoreError> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.lock()?;
        let placeholders = vec!["?"; ids.len()].join(",");
        conn.execute(
            &format!("UPDATE scribe_events SET deleted = 1 WHERE id IN ({placeholders})"),
            params_from_iter(ids.iter()),
        )?;
        Ok(())
    }

    pub fn restore_items(&self, ids: &[i64]) -> Result<(), StoreError> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.lock()?;
        let placeholders = vec!["?"; ids.len()].join(",");
        conn.execute(
            &format!("UPDATE scribe_events SET deleted = 0 WHERE id IN ({placeholders})"),
            params_from_iter(ids.iter()),
        )?;
        Ok(())
    }

    pub fn move_item(&self, id: i64, ord: f64) -> Result<Item, StoreError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE scribe_events SET ord = ?2 WHERE id = ?1",
            params![id, ord],
        )?;
        self.item_row(&conn, id)
    }

    /// One item by id, including soft-deleted ones (event payloads for Undo).
    pub fn item(&self, id: i64) -> Result<Item, StoreError> {
        let conn = self.lock()?;
        self.item_row(&conn, id)
    }

    /// Split a speech item at a character boundary. The second half is
    /// inserted directly after the first, midway to the next item.
    pub fn split_item(&self, id: i64, char_index: usize) -> Result<(Item, Item), StoreError> {
        let conn = self.lock()?;
        let first = self.item_row(&conn, id)?;
        if first.kind != ItemKind::Speech {
            return Err(StoreError::NotSpeech);
        }
        let text = first.text.clone().ok_or(StoreError::NotSpeech)?;
        let chars: Vec<char> = text.chars().collect();
        if char_index == 0 || char_index >= chars.len() {
            return Err(StoreError::BadSplitIndex(char_index, chars.len()));
        }
        let head: String = chars[..char_index].iter().collect();
        let tail: String = chars[char_index..].iter().collect();

        conn.execute(
            "UPDATE scribe_events SET content = ?2 WHERE id = ?1",
            params![id, head],
        )?;
        let next_ord: Option<f64> = conn.query_row(
            "SELECT MIN(ord) FROM scribe_events
             WHERE session_id = ?1 AND ord > ?2 AND deleted = 0",
            params![first.session_id, first.ord],
            |r| r.get(0),
        )?;
        let second_ord = (first.ord + next_ord.unwrap_or(first.ord + 1.0)) / 2.0;
        conn.execute(
            "INSERT INTO scribe_events
                 (session_id, event_type, t_start_ms, t_end_ms, ord,
                  content, image_path, source_app, metadata)
             VALUES (?1, 'transcript', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                first.session_id,
                first.t_start_ms,
                first.t_end_ms,
                second_ord,
                tail,
                first.image_path,
                first.source_app,
                serde_json::to_string(&Value::Null)?,
            ],
        )?;
        let updated_first = self.item_row(&conn, id)?;
        let second = self.item_row(&conn, conn.last_insert_rowid())?;
        fts_put(&conn, &updated_first)?;
        fts_put(&conn, &second)?;
        Ok((updated_first, second))
    }

    /// Sessions, newest first. `query` filters to sessions with a live item
    /// whose text, caption or OCR matches (FTS5, terms ANDed).
    pub fn list_sessions(&self, query: Option<&str>) -> Result<Vec<SessionSummary>, StoreError> {
        let conn = self.lock()?;
        let ids: Vec<i64> = match query {
            Some(q) => {
                let match_expr = q
                    .split_whitespace()
                    .map(|t| format!("\"{}\"", t.replace('"', "")))
                    .collect::<Vec<_>>()
                    .join(" AND ");
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT s.id FROM scribe_sessions s
                     JOIN scribe_events e ON e.session_id = s.id
                     JOIN scribe_events_fts f ON f.rowid = e.id
                     WHERE scribe_events_fts MATCH ?1 AND e.deleted = 0
                     ORDER BY s.id DESC",
                )?;
                let rows = stmt.query_map(params![match_expr], |r| r.get(0))?;
                rows.collect::<Result<_, _>>()?
            }
            None => {
                let mut stmt = conn.prepare("SELECT id FROM scribe_sessions ORDER BY id DESC")?;
                let rows = stmt.query_map([], |r| r.get(0))?;
                rows.collect::<Result<_, _>>()?
            }
        };

        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let session = self.session_row(&conn, id)?;
            let (mark_count, clip_count): (i64, i64) = conn.query_row(
                "SELECT COALESCE(SUM(event_type = 'mark'), 0),
                        COALESCE(SUM(event_type = 'clipboard'), 0)
                 FROM scribe_events WHERE session_id = ?1 AND deleted = 0",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let snippet: Option<String> = fetch_one(
                &conn,
                "SELECT content FROM scribe_events
                 WHERE session_id = ?1 AND deleted = 0
                   AND content IS NOT NULL AND content != ''
                 ORDER BY ord, id LIMIT 1",
                params![id],
                |r| Ok(r.get::<_, String>(0)?),
            )?;
            out.push(SessionSummary {
                id: session.id,
                name: session.name,
                started_at_ms: session.started_at_ms,
                ended_at_ms: session.ended_at_ms,
                mark_count: mark_count as u32,
                clip_count: clip_count as u32,
                snippet: snippet.map(|s| s.chars().take(120).collect::<String>()),
            });
        }
        Ok(out)
    }

    pub fn rename_session(&self, id: i64, name: &str) -> Result<(), StoreError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE scribe_sessions SET name = ?2 WHERE id = ?1",
            params![id, name],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, id: i64) -> Result<(), StoreError> {
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM scribe_events_fts WHERE rowid IN
                 (SELECT id FROM scribe_events WHERE session_id = ?1)",
            params![id],
        )?;
        conn.execute(
            "DELETE FROM scribe_events WHERE session_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM scribe_sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_output(
        &self,
        id: i64,
        target: Target,
        template: &str,
        preface: Option<&str>,
    ) -> Result<Session, StoreError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE scribe_sessions SET target = ?2, template = ?3, preface = ?4 WHERE id = ?1",
            params![id, target_to_db(&target), template, preface],
        )?;
        self.session_row(&conn, id)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
        self.conn
            .lock()
            .map_err(|_| StoreError::Corrupt("store mutex poisoned".into()))
    }

    fn session_row(&self, conn: &Connection, id: i64) -> Result<Session, StoreError> {
        fetch_one(
            conn,
            "SELECT * FROM scribe_sessions WHERE id = ?1",
            params![id],
            session_from_row,
        )?
        .ok_or(StoreError::SessionNotFound(id))
    }

    fn item_row(&self, conn: &Connection, id: i64) -> Result<Item, StoreError> {
        fetch_one(
            conn,
            "SELECT * FROM scribe_events WHERE id = ?1",
            params![id],
            item_from_row,
        )?
        .ok_or(StoreError::ItemNotFound(id))
    }
}

/// Replace the FTS row for `item`: its text, its caption and the OCR text
/// carried in `payload.ocr`.
fn fts_put(conn: &Connection, item: &Item) -> Result<(), StoreError> {
    conn.execute(
        "DELETE FROM scribe_events_fts WHERE rowid = ?1",
        params![item.id],
    )?;
    conn.execute(
        "INSERT INTO scribe_events_fts(rowid, content, caption, ocr) VALUES (?1, ?2, ?3, ?4)",
        params![
            item.id,
            item.text.as_deref().unwrap_or(""),
            item.caption.as_deref().unwrap_or(""),
            item.payload
                .get("ocr")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    fn mem() -> Store {
        Store::open_in_memory().expect("open_in_memory")
    }

    fn speech(text: &str) -> NewItem {
        NewItem {
            kind: ItemKind::Speech,
            t_start_ms: 0,
            t_end_ms: None,
            text: Some(text.into()),
            image_path: None,
            source_app: None,
            payload: json!({}),
        }
    }

    fn clip(text: &str) -> NewItem {
        NewItem {
            kind: ItemKind::Clip,
            t_start_ms: 0,
            t_end_ms: None,
            text: Some(text.into()),
            image_path: None,
            source_app: None,
            payload: json!({}),
        }
    }

    fn mark(name: &str) -> NewItem {
        NewItem {
            kind: ItemKind::Mark,
            t_start_ms: 0,
            t_end_ms: None,
            text: None,
            image_path: Some(format!("images/{name}.png")),
            source_app: None,
            payload: json!({}),
        }
    }

    fn temp_db() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "speaktype_session_test_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).expect("tempdir");
        dir.join("scribe.db")
    }

    #[test]
    fn opens_v1_db_and_resets_to_v2() {
        let db = temp_db();
        {
            let conn = Connection::open(&db).expect("open v1");
            conn.execute_batch(
                "CREATE TABLE scribe_sessions (
                     id INTEGER PRIMARY KEY AUTOINCREMENT,
                     name TEXT NOT NULL,
                     template TEXT DEFAULT 'general',
                     created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                     ended_at DATETIME
                 );
                 CREATE TABLE scribe_events (
                     id INTEGER PRIMARY KEY AUTOINCREMENT,
                     session_id INTEGER NOT NULL REFERENCES scribe_sessions(id) ON DELETE CASCADE,
                     event_type TEXT NOT NULL,
                     timestamp DATETIME NOT NULL,
                     content TEXT,
                     image_path TEXT,
                     source_app TEXT,
                     metadata TEXT
                 );
                 CREATE INDEX idx_scribe_events_session ON scribe_events(session_id, timestamp);
                 CREATE VIRTUAL TABLE scribe_events_fts USING fts5(
                     content, source_app, content='scribe_events', content_rowid='id'
                 );
                 INSERT INTO scribe_sessions (name, template, created_at)
                     VALUES ('stale dev session', 'general', '2026-01-01T10:00:00.000');
                 PRAGMA user_version = 1;",
            )
            .expect("seed v1");
        }

        let store = Store::open(&db).expect("open upgrades v1");

        let conn = Connection::open(&db).expect("reopen");
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .expect("user_version");
        assert_eq!(version, 2, "user_version must be 2 after open");
        for (table, col) in [
            ("scribe_events", "t_start_ms"),
            ("scribe_events", "t_end_ms"),
            ("scribe_events", "ord"),
            ("scribe_events", "caption"),
            ("scribe_events", "deleted"),
            ("scribe_sessions", "target"),
            ("scribe_sessions", "preface"),
            ("scribe_sessions", "bundle_path"),
        ] {
            let has: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = ?1"),
                    [col],
                    |r| r.get(0),
                )
                .expect("table_info");
            assert_eq!(has, 1, "column {table}.{col} missing after migration");
        }
        let sessions: i64 = conn
            .query_row("SELECT COUNT(*) FROM scribe_sessions", [], |r| r.get(0))
            .expect("count");
        assert_eq!(sessions, 0, "v1 rows are dropped, not migrated");

        let session = store
            .create_session("New", "auto", 1_700_000_000_000)
            .expect("create on upgraded db");
        let item = store
            .add_item(session.id, speech("hello"))
            .expect("add on upgraded db");
        assert_eq!(item.ord, 1.0);
    }

    #[test]
    fn items_return_in_ord_order_without_deleted() {
        let store = mem();
        let session = store.create_session("S", "auto", 0).expect("session");
        let a = store.add_item(session.id, speech("alpha")).expect("a");
        let b = store.add_item(session.id, speech("beta")).expect("b");
        let c = store.add_item(session.id, speech("gamma")).expect("c");
        store.move_item(c.id, a.ord - 0.5).expect("move c first");
        store.delete_items(&[b.id]).expect("delete b");

        let detail = store.detail(session.id).expect("detail");
        let texts: Vec<&str> = detail
            .items
            .iter()
            .map(|i| i.text.as_deref().expect("speech text"))
            .collect();
        assert_eq!(texts, ["gamma", "alpha"]);
    }

    #[test]
    fn move_item_between_neighbours_keeps_order() {
        let store = mem();
        let session = store.create_session("S", "auto", 0).expect("session");
        let i1 = store.add_item(session.id, speech("one")).expect("1");
        let i2 = store.add_item(session.id, speech("two")).expect("2");
        let i3 = store.add_item(session.id, speech("three")).expect("3");

        let midway = (i1.ord + i2.ord) / 2.0;
        store.move_item(i3.id, midway).expect("move");

        let detail = store.detail(session.id).expect("detail");
        let texts: Vec<&str> = detail
            .items
            .iter()
            .map(|i| i.text.as_deref().expect("speech text"))
            .collect();
        assert_eq!(texts, ["one", "three", "two"]);
    }

    #[test]
    fn split_item_splits_on_char_boundary_not_byte() {
        let store = mem();
        let session = store.create_session("S", "auto", 0).expect("session");
        let item = store
            .add_item(session.id, speech("héllo wörld"))
            .expect("add");

        let (first, second) = store.split_item(item.id, 6).expect("split");
        assert_eq!(first.text.as_deref(), Some("héllo "));
        assert_eq!(second.text.as_deref(), Some("wörld"));

        let detail = store.detail(session.id).expect("detail");
        let texts: Vec<&str> = detail
            .items
            .iter()
            .map(|i| i.text.as_deref().expect("speech text"))
            .collect();
        assert_eq!(texts, ["héllo ", "wörld"]);
    }

    #[test]
    fn split_non_speech_errors() {
        let store = mem();
        let session = store.create_session("S", "auto", 0).expect("session");
        let item = store
            .add_item(session.id, clip("cargo build"))
            .expect("clip");
        assert!(store.split_item(item.id, 3).is_err());

        let mark_item = store.add_item(session.id, mark("m1")).expect("mark");
        assert!(store.split_item(mark_item.id, 1).is_err());
    }

    #[test]
    fn delete_then_restore_round_trips() {
        let store = mem();
        let session = store.create_session("S", "auto", 0).expect("session");
        store.add_item(session.id, speech("keep")).expect("a");
        let b = store.add_item(session.id, speech("gone")).expect("b");

        store.delete_items(&[b.id]).expect("delete");
        assert_eq!(store.detail(session.id).expect("detail").items.len(), 1);

        store.restore_items(&[b.id]).expect("restore");
        let detail = store.detail(session.id).expect("detail");
        assert_eq!(detail.items.len(), 2);
        assert_eq!(
            detail
                .items
                .iter()
                .find(|i| i.id == b.id)
                .expect("restored")
                .text
                .as_deref(),
            Some("gone")
        );
    }

    #[test]
    fn list_sessions_search_matches_clip_and_caption_text() {
        let store = mem();
        let a = store.create_session("Cassette", "auto", 100).expect("a");
        store
            .add_item(a.id, clip("quantum flux capacitor"))
            .expect("clip");
        let m = store.add_item(a.id, mark("m1")).expect("mark");
        store
            .update_item(
                m.id,
                ItemPatch {
                    text: None,
                    caption: Some("flux capacitor diagram".into()),
                },
            )
            .expect("caption");
        let b = store.create_session("Other", "auto", 200).expect("b");
        store
            .add_item(b.id, speech("unrelated words"))
            .expect("speech");

        let all = store.list_sessions(None).expect("list");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, b.id, "newest first");
        let summary_a = all.iter().find(|s| s.id == a.id).expect("summary a");
        assert_eq!(summary_a.mark_count, 1);
        assert_eq!(summary_a.clip_count, 1);
        assert_eq!(summary_a.snippet.as_deref(), Some("quantum flux capacitor"));

        let by_clip_text = store.list_sessions(Some("capacitor")).expect("search");
        assert_eq!(by_clip_text.len(), 1);
        assert_eq!(by_clip_text[0].id, a.id);

        let by_caption = store.list_sessions(Some("diagram")).expect("search");
        assert_eq!(by_caption.len(), 1);
        assert_eq!(by_caption[0].id, a.id);

        let none = store.list_sessions(Some("zebra")).expect("search");
        assert!(none.is_empty());
    }

    #[test]
    fn concurrent_add_item_from_two_threads_keeps_unique_ord() {
        let store = mem();
        let session = store.create_session("S", "auto", 0).expect("session");

        std::thread::scope(|scope| {
            for _ in 0..2 {
                scope.spawn(|| {
                    for i in 0..100 {
                        store
                            .add_item(session.id, speech(&format!("t{i}")))
                            .expect("concurrent add");
                    }
                });
            }
        });

        let detail = store.detail(session.id).expect("detail");
        assert_eq!(detail.items.len(), 200);
        let unique: HashSet<u64> = detail.items.iter().map(|i| i.ord.to_bits()).collect();
        assert_eq!(unique.len(), 200, "every ord must be distinct");
    }

    #[test]
    fn session_lifecycle_and_set_output() {
        let store = mem();
        let session = store
            .create_session("Session", "auto", 1_700_000_000_000)
            .expect("create");
        assert_eq!(session.name, "Session");
        assert_eq!(session.template, "auto");
        assert_eq!(session.started_at_ms, 1_700_000_000_000);
        assert_eq!(session.ended_at_ms, None);
        assert_eq!(session.target, Target::CodingAgent);

        let active = store.active_session().expect("active");
        assert_eq!(active.expect("active session").id, session.id);

        let ended = store
            .end_session(session.id, 1_700_000_060_000)
            .expect("end");
        assert_eq!(ended.ended_at_ms, Some(1_700_000_060_000));
        assert!(store.active_session().expect("active").is_none());

        let updated = store
            .set_output(
                session.id,
                Target::Chat,
                "qa_test_plan",
                Some("Frame the plan"),
            )
            .expect("set_output");
        assert_eq!(updated.target, Target::Chat);
        assert_eq!(updated.template, "qa_test_plan");
        assert_eq!(updated.preface.as_deref(), Some("Frame the plan"));

        store.rename_session(session.id, "Renamed").expect("rename");
        assert_eq!(store.list_sessions(None).expect("list")[0].name, "Renamed");

        store.delete_session(session.id).expect("delete");
        assert!(store.list_sessions(None).expect("list").is_empty());
        assert!(store.detail(session.id).is_err());
    }

    #[test]
    fn item_by_id_returns_soft_deleted_items_too() {
        let store = mem();
        let session = store.create_session("S", "auto", 0).expect("session");
        let item = store.add_item(session.id, speech("temp")).expect("add");
        store.delete_items(&[item.id]).expect("delete");
        // detail() hides it, but item() still resolves it (Undo events need it).
        assert!(store.detail(session.id).expect("detail").items.is_empty());
        assert_eq!(
            store.item(item.id).expect("item").text.as_deref(),
            Some("temp")
        );
        assert!(store.item(item.id + 999).is_err());
    }

    #[test]
    fn item_patch_updates_text_and_caption() {
        let store = mem();
        let session = store.create_session("S", "auto", 0).expect("session");
        let mark_item = store.add_item(session.id, mark("m1")).expect("mark");

        let patched = store
            .update_item(
                mark_item.id,
                ItemPatch {
                    text: None,
                    caption: Some("framed the bug".into()),
                },
            )
            .expect("patch");
        assert_eq!(patched.caption.as_deref(), Some("framed the bug"));
        assert_eq!(patched.text, None, "None in the patch leaves text alone");
    }
    #[test]
    fn concurrent_starts_create_one_session() {
        let store = std::sync::Arc::new(Store::open_in_memory().unwrap());
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let store = store.clone();
                std::thread::spawn(move || store.start_session("s", "auto", 1).unwrap().0.id)
            })
            .collect();
        let ids: std::collections::HashSet<i64> =
            handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(ids.len(), 1);
    }
}
