use crate::capture::png_dimensions;
use crate::logging::log_message;
use crate::scribe::ScribeStore;
use std::path::PathBuf;

use docx_rs::{Docx, Paragraph, Pic, Run, Style, StyleType};

fn category(t: &str) -> &'static str {
    match t {
        "transcript" => "Voice Dictation",
        "screenshot" => "Screenshots",
        "clipboard" => "Copied Text",
        _ => "Notes",
    }
}

fn stamp(ts: &str) -> String {
    chrono::DateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S%.3f")
        .map(|d| d.format("%H:%M:%S").to_string())
        .unwrap_or_else(|_| ts.to_string())
}

/// Register the built-in heading styles docx-rs doesn't include by default.
fn with_styles(doc: Docx) -> Docx {
    doc.add_style(Style::new("Title", StyleType::Paragraph).name("Title"))
        .add_style(Style::new("Heading1", StyleType::Paragraph).name("Heading 1"))
        .add_style(Style::new("Heading2", StyleType::Paragraph).name("Heading 2"))
}

/// Build the session's `.docx` as bytes so callers (and tests) decide where to
/// write them.
pub fn build_session_docx(store: &ScribeStore, session_id: i64) -> Result<Vec<u8>, String> {
    let session = store.get_session(session_id)?.ok_or("Session not found")?;
    let events = store.get_events(session_id)?;

    let mut doc = with_styles(Docx::new());
    doc = doc.add_paragraph(
        Paragraph::new()
            .style("Title")
            .add_run(Run::new().add_text(&session.name)),
    );
    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(format!(
        "Started {} · {} captures",
        session.created_at,
        events.len()
    ))));

    let mut order: Vec<&'static str> = Vec::new();
    for e in &events {
        let cat = category(&e.event_type);
        if !order.contains(&cat) {
            order.push(cat);
        }
    }

    for title in order {
        doc = doc.add_paragraph(
            Paragraph::new()
                .style("Heading1")
                .add_run(Run::new().add_text(title)),
        );
        for e in events.iter().filter(|e| category(&e.event_type) == title) {
            let time = stamp(&e.timestamp);
            match e.event_type.as_str() {
                "screenshot" => {
                    let ocr = e
                        .content
                        .as_deref()
                        .filter(|t| !t.trim().is_empty())
                        .map(|t| t.to_string())
                        .unwrap_or_default();
                    let mut para = Paragraph::new()
                        .add_run(Run::new().bold().add_text(format!("{time}: ")))
                        .add_run(Run::new().add_text("Screenshot"));
                    if let Some(rel) = e.image_path.as_deref() {
                        let abs = ScribeStore::session_dir(session_id)
                            .join(rel.split('/').next_back().unwrap_or(rel));
                        if let Ok(img) = std::fs::read(&abs) {
                            if let Some((w, h)) = png_dimensions(&img) {
                                para = para.add_run(
                                    Run::new().add_image(Pic::new_with_dimensions(img, w, h)),
                                );
                            } else {
                                log_message(&format!(
                                    "[scribe] Skipping non-PNG screenshot {} in docx",
                                    abs.to_string_lossy()
                                ));
                            }
                        } else {
                            log_message(&format!(
                                "[scribe] Screenshot image {} missing for docx export",
                                abs.to_string_lossy()
                            ));
                        }
                    }
                    if !ocr.is_empty() {
                        para = para.add_run(Run::new().add_text(format!("OCR:\n{ocr}")));
                    }
                    doc = doc.add_paragraph(para);
                }
                _ => {
                    doc = doc.add_paragraph(
                        Paragraph::new()
                            .add_run(Run::new().bold().add_text(format!("{time}: ")))
                            .add_run(Run::new().add_text(e.content.as_deref().unwrap_or(""))),
                    );
                }
            }
        }
    }

    let mut out = std::io::Cursor::new(Vec::new());
    doc.build()
        .pack(&mut out)
        .map_err(|e| format!("Failed to write docx: {}", e))?;
    Ok(out.into_inner())
}

/// Write the session's `.docx` to Documents/SpeakType/Scribe and return the path.
pub fn export_session_docx(store: &ScribeStore, session_id: i64) -> Result<PathBuf, String> {
    let session = store.get_session(session_id)?.ok_or("Session not found")?;
    let bytes = build_session_docx(store, session_id)?;

    let docs = dirs::document_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SpeakType")
        .join("Scribe");
    std::fs::create_dir_all(&docs).map_err(|e| e.to_string())?;
    let safe_name: String = session
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let ts = chrono::Local::now().format("%Y-%m-%d").to_string();
    let file = docs.join(format!("{}_{}.docx", safe_name, ts));

    std::fs::write(&file, &bytes).map_err(|e| e.to_string())?;
    log_message(&format!(
        "[scribe] Exported session {} to DOCX {:?}",
        session_id, file
    ));
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scribe::ScribeStore;
    use rusqlite::Connection;

    fn store_with_session() -> (ScribeStore, i64) {
        let dir = std::env::temp_dir().join(format!("scribe_docx_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conn = Connection::open(dir.join("scribe.db")).unwrap();
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
        let id = store.start_session("Docx Test", "general").unwrap();
        store
            .add_event(
                id,
                "transcript",
                Some("hello docx"),
                None,
                Some("Chrome"),
                None,
            )
            .unwrap();
        store
            .add_event(
                id,
                "screenshot",
                Some("OCR text here"),
                Some("x.png"),
                None,
                None,
            )
            .unwrap();
        (store, id)
    }

    #[test]
    fn builds_valid_docx_bytes() {
        let (store, id) = store_with_session();
        let bytes = build_session_docx(&store, id).unwrap();
        assert!(bytes.starts_with(b"PK\x03\x04"), "docx must be a zip");
        // Zip entry names are stored uncompressed — the document part must exist.
        assert!(
            bytes
                .windows(b"word/document.xml".len())
                .any(|w| w == b"word/document.xml"),
            "docx must contain word/document.xml part"
        );
    }

    /// A minimal valid 1x1 transparent PNG (IHDR width/height = 1).
    const ONE_PX_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x04, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0x78,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn png_dimensions_reads_ihdr() {
        assert_eq!(png_dimensions(ONE_PX_PNG), Some((1, 1)));
        assert_eq!(png_dimensions(b"not a png"), None);
    }

    #[test]
    fn embeds_screenshot_image_in_docx() {
        let (store, id) = store_with_session();
        let dir = ScribeStore::session_dir(id);
        // start_session already created the dir; write the image the event
        // points at so the exporter finds it.
        std::fs::write(dir.join("x.png"), ONE_PX_PNG).unwrap();

        let bytes = build_session_docx(&store, id).unwrap();
        assert!(
            bytes.windows(b"media/".len()).any(|w| w == b"media/"),
            "docx must reference the embedded screenshot media part"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
