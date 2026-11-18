use crate::scribe::ScribeStore;

/// Persona templates for the LLM assembly step (Phase 2). For the MVP the
/// session renders to a plain chronological markdown document; these prompt
/// strings define how the assembled raw captures get restructured.
pub const TEMPLATES: &[(&str, &str)] = &[
    (
        "general",
        "You are a research assistant. The user captured a working session with \
         voice dictation, screenshots, and clipboard copies. Assemble the most \
         useful parts into a well-structured document with clear sections, key \
         findings, and a short summary.",
    ),
    (
        "thesis",
        "You are an academic writing assistant. The user is working on a thesis. \
         Turn their session captures into a structured thesis draft with \
         introduction, methods, findings, and references.",
    ),
    (
        "qa_test_plan",
        "You are a QA documentation assistant. Turn the session captures into a \
         structured test plan or bug report with clear reproduction steps, \
         expected vs actual behavior, and affected components.",
    ),
    (
        "blog",
        "You are a blog-writing assistant. Turn the user's session captures into \
         a well-structured blog post with an introduction, body sections, and a \
         conclusion.",
    ),
    (
        "meeting_notes",
        "You are a meeting-notes assistant. Turn the session captures into \
         structured meeting notes with decisions, action items, and key \
         discussion points.",
    ),
];

pub fn template_prompt(template: &str) -> &'static str {
    TEMPLATES
        .iter()
        .find(|(key, _)| *key == template)
        .map(|(_, prompt)| *prompt)
        .unwrap_or(TEMPLATES[0].1)
}

/// Lightweight, keyword-based template detection so the app can pick how to
/// assemble a document without asking the user. Used when the session is created
/// with "auto" and again at generation time on the real captured content.
pub fn infer_template(text: &str) -> &'static str {
    let t = text.to_lowercase();
    if contains_any(
        &t,
        &[
            "thesis",
            "dissertation",
            "methodology",
            "literature review",
            "abstract",
            "research question",
            "defense",
            "supervisor",
            "citation",
        ],
    ) {
        "thesis"
    } else if contains_any(
        &t,
        &[
            "meeting",
            "standup",
            "agenda",
            "action item",
            "action items",
            "decision",
            "attendee",
            "follow-up",
            "kickoff",
            "retro",
        ],
    ) {
        "meeting_notes"
    } else if contains_any(
        &t,
        &[
            "bug",
            "reproduc",
            "test plan",
            "test case",
            "regression",
            "qa",
            "staging",
            "error log",
            "crash report",
            "investigate",
            "expected vs actual",
        ],
    ) {
        "qa_test_plan"
    } else if contains_any(
        &t,
        &[
            "blog",
            "post",
            "article",
            "audience",
            "publish",
            "newsletter",
        ],
    ) {
        "blog"
    } else {
        "general"
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| text.contains(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_session_type_from_name() {
        assert_eq!(
            infer_template("Thesis research — methodology review"),
            "thesis"
        );
        assert_eq!(infer_template("Standup and action items"), "meeting_notes");
        assert_eq!(infer_template("Regression bug on checkout"), "qa_test_plan");
        assert_eq!(infer_template("Blog post draft"), "blog");
        assert_eq!(infer_template("Catch-up session"), "general");
    }

    #[test]
    fn infers_from_captured_content_over_name() {
        let content = "discussed the kickoff agenda for the new sprint and assigned action items";
        assert_eq!(infer_template(content), "meeting_notes");
        let content = "reproduced the crash on staging, writing a test case";
        assert_eq!(infer_template(content), "qa_test_plan");
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::scribe::ScribeStore;
    use rusqlite::Connection;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SEQ: AtomicUsize = AtomicUsize::new(0);

    fn store_with_shot() -> (ScribeStore, i64) {
        let dir = std::env::temp_dir().join(format!(
            "scribe_tpl_test_{}_{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conn = Connection::open(dir.join("scribe.db")).unwrap();
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
             CREATE INDEX idx_scribe_events_session
                 ON scribe_events(session_id, timestamp);
             CREATE VIRTUAL TABLE IF NOT EXISTS scribe_events_fts
                 USING fts5(content, source_app, content='scribe_events', content_rowid='id');",
        )
        .unwrap();
        let store = ScribeStore::from_conn(conn);
        let id = store.start_session("Render Test", "general").unwrap();
        store
            .add_event(
                id,
                "screenshot",
                Some("OCR line"),
                Some("scribe/9/shot.png"),
                None,
                None,
            )
            .unwrap();
        (store, id)
    }

    #[test]
    fn default_root_uses_session_dir() {
        let (store, id) = store_with_session_owned();
        let md = render_session_markdown(&store, id).unwrap();
        let abs = ScribeStore::session_dir(id);
        assert!(
            md.contains(&format!(
                "![screenshot]({}/shot.png)",
                abs.to_string_lossy()
            )),
            "default render should reference the session's absolute path:\n{}",
            md
        );
    }

    #[test]
    fn custom_root_writes_relative_paths() {
        let (store, id) = store_with_session_owned();
        let md =
            render_session_markdown_with(&store, id, Some(std::path::Path::new("export/images")))
                .unwrap();
        assert!(
            md.contains("![screenshot](export/images/shot.png)"),
            "custom root should produce a relative, portable reference:\n{}",
            md
        );
        assert!(
            !md.contains("Application Support"),
            "must not leak absolute paths"
        );
    }

    fn store_with_session_owned() -> (ScribeStore, i64) {
        store_with_shot()
    }
}

/// Render a session into a plain chronological markdown document. This is the
/// MVP export (no LLM): raw transcript, screenshots with OCR text, and
/// clipboard copies in timestamp order.
///
/// Screenshot images are referenced by their absolute session path so they
/// resolve in the in-app preview and read sensibly as LLM input.
pub fn render_session_markdown(store: &ScribeStore, session_id: i64) -> Result<String, String> {
    render_session_markdown_with(store, session_id, None)
}

/// Standalone markdown image references (absolute paths) for every screenshot
/// in a session. Used by the LLM assembly path so the generated document always
/// includes the captured screenshots even when the model drops them from prose.
pub fn session_screenshot_images(store: &ScribeStore, session_id: i64) -> Result<String, String> {
    let events = store.get_events(session_id)?;
    let mut out = String::new();
    for event in &events {
        if event.event_type != "screenshot" {
            continue;
        }
        if let Some(path) = &event.image_path {
            let file = ScribeStore::session_dir(session_id)
                .join(path.split('/').next_back().unwrap_or(path));
            out.push_str(&format!("![screenshot]({})\n", file.to_string_lossy()));
        }
    }
    Ok(out)
}

/// Like [`render_session_markdown`], but writes screenshot references rooted at
/// `image_root` instead of the session's absolute path. Used by export so the
/// `.md` stays portable next to the copied screenshot files.
pub fn render_session_markdown_with(
    store: &ScribeStore,
    session_id: i64,
    image_root: Option<&std::path::Path>,
) -> Result<String, String> {
    let session = store.get_session(session_id)?.ok_or("Session not found")?;
    let events = store.get_events(session_id)?;
    let default_root = ScribeStore::session_dir(session_id);
    let root = image_root.unwrap_or(&default_root);

    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", session.name));
    out.push_str(&format!(
        "_Started {} · {} captures_\n\n",
        session.created_at,
        events.len()
    ));

    let mut categories = std::collections::BTreeMap::<&str, String>::new();
    for event in &events {
        categories
            .entry(match event.event_type.as_str() {
                "transcript" => "Voice Dictation",
                "screenshot" => "Screenshots",
                "clipboard" => "Copied Text",
                other => other,
            })
            .or_default();
    }

    for title in categories.keys() {
        out.push_str(&format!("\n## {}\n\n", title));
        for event in events.iter().filter(|e| {
            let key = match e.event_type.as_str() {
                "transcript" => "Voice Dictation",
                "screenshot" => "Screenshots",
                "clipboard" => "Copied Text",
                other => other,
            };
            key == *title
        }) {
            let stamp = &event.timestamp[11..19];
            match event.event_type.as_str() {
                "transcript" => out.push_str(&format!(
                    "- **{stamp}** {}\n",
                    event.content.as_deref().unwrap_or("")
                )),
                "clipboard" => out.push_str(&format!(
                    "- **{stamp}** {}\n",
                    event.content.as_deref().unwrap_or("")
                )),
                "screenshot" => {
                    out.push_str(&format!("- **{stamp}** screenshot\n"));
                    if let Some(path) = &event.image_path {
                        let file = root.join(path.split('/').next_back().unwrap_or(path));
                        out.push_str(&format!("  ![screenshot]({})\n", file.to_string_lossy()));
                    }
                    // OCR text lives in metadata.ocr for screenshots.
                    if let Some(meta) = &event.metadata {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(meta) {
                            if let Some(ocr) = v.get("ocr").and_then(|o| o.as_str()) {
                                out.push_str(&format!("  > {}\n\n", ocr));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(out)
}
