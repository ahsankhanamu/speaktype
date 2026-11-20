use crate::commands::AppState;
use crate::logging::log_message;
use crate::paste;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// Poll the clipboard and capture user copies into the active session.
///
/// Our own dictation paste is excluded via the `is_pasting` flag on AppState,
/// and repeated identical copies are deduplicated by tracking the last text +
/// a maturation timestamp (the paste restore happens ~500ms after our paste).
/// With no active session the monitor is a no-op: captures never start one.
pub fn spawn_clipboard_monitor(app: AppHandle) {
    std::thread::spawn(move || {
        // Prime the dedup baseline with whatever is already on the clipboard,
        // so a capture only happens for a copy the user makes *after* launch
        // — never for the content that was there at startup.
        let mut last_text: Option<String> = arboard::Clipboard::new()
            .ok()
            .and_then(|mut cb| cb.get_text().ok());
        let mut last_capture: Option<String> = None;
        let mut paste_maturation = std::time::Instant::now();
        let mut was_pasting = false;

        loop {
            std::thread::sleep(Duration::from_millis(400));

            let state = app.state::<AppState>();
            let is_pasting = state.is_pasting.load(Ordering::SeqCst);

            let monitor_enabled = state
                .settings
                .lock()
                .ok()
                .map(|s| s.scribe_clipboard_monitor)
                .unwrap_or(true);
            if !monitor_enabled {
                continue;
            }

            if is_pasting {
                if !was_pasting {
                    paste_maturation = std::time::Instant::now();
                    was_pasting = true;
                }
                last_text = None;
                continue;
            }
            was_pasting = false;

            // Right after our own paste (which set+restored the clipboard) the
            // user's original text is back; skip capturing the restore window.
            if paste_maturation.elapsed() < Duration::from_millis(700) {
                continue;
            }

            let current = arboard::Clipboard::new()
                .ok()
                .and_then(|mut cb| cb.get_text().ok());
            let text = match current {
                Some(t) => t,
                None => {
                    last_text = None;
                    continue;
                }
            };

            if last_text.as_deref() == Some(text.as_str()) {
                continue;
            }
            last_text = Some(text.clone());

            // Avoid re-capturing the exact same content if the user copies it
            // twice in a row — trivial guard.
            if last_capture.as_deref() == Some(text.as_str()) {
                continue;
            }

            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.matches('\n').count() > 500 {
                continue;
            }
            if trimmed.len() > 8_000 {
                continue;
            }

            // Clipboard capture only feeds an *already-live* session (spec §12).
            let session_store = app.state::<std::sync::Arc<speaktype_session::Store>>();
            let Ok(Some(session)) = session_store.active_session() else {
                continue;
            };

            // If the user copied the exact text they just dictated, treat it as
            // the same content — don't duplicate it as a "copied" item.
            if clipboard_matches_recent_dictation(&session_store, session.id, trimmed) {
                log_message(&format!(
                    "[clipboard] Skipped copy identical to recent dictation ({} chars)",
                    trimmed.len()
                ));
                last_capture = Some(text);
                continue;
            }

            let item = speaktype_session::NewItem {
                kind: speaktype_session::ItemKind::Clip,
                t_start_ms: chrono::Utc::now().timestamp_millis(),
                t_end_ms: None,
                text: Some(trimmed.to_string()),
                image_path: None,
                source_app: paste::get_frontmost_window().map(|w| w.name),
                payload: serde_json::json!({}),
            };
            match session_store.add_item(session.id, item) {
                Ok(item) => {
                    last_capture = Some(text);
                    log_message(&format!(
                        "[clipboard] Captured ({}) into session {}",
                        item.id, session.id
                    ));
                    crate::session_cmds::emit_item_added(&app, &item);
                    crate::narration::touch();
                }
                Err(e) => log_message(&format!("[clipboard] Capture failed: {}", e)),
            }
        }
    });
}

/// True when `text` matches one of the session's last five dictations, so a
/// copy of freshly dictated text is not captured twice (spec §12).
fn clipboard_matches_recent_dictation(
    store: &speaktype_session::Store,
    session_id: i64,
    text: &str,
) -> bool {
    match store.detail(session_id) {
        Ok(detail) => detail
            .items
            .iter()
            .filter(|i| i.kind == speaktype_session::ItemKind::Speech)
            .rev()
            .take(5)
            .any(|i| i.text.as_deref() == Some(text)),
        Err(_) => false,
    }
}
