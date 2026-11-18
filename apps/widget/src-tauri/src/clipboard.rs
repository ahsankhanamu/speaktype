use crate::commands::AppState;
use crate::logging::log_message;
use crate::paste;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Poll the clipboard and capture user copies into the active scribe session.
///
/// Our own dictation paste is excluded via the `is_pasting` flag on AppState,
/// and repeated identical copies are deduplicated by tracking the last text +
/// a maturation timestamp (the paste restore happens ~500ms after our paste).
pub fn spawn_clipboard_monitor(app: AppHandle) {
    std::thread::spawn(move || {
        // Prime the dedup baseline with whatever is already on the clipboard,
        // so a session is only started by a copy the user makes *after* launch
        // — never by the content that was there at startup.
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
            let scribe_on = state
                .settings
                .lock()
                .ok()
                .map(|s| s.scribe_enabled)
                .unwrap_or(false);
            if !scribe_on {
                last_text = None;
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
            // twice months apart *within the same session* — trivial guard.
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

            // Clipboard capture only feeds an *already-live* session. It must
            // never start one: recording begins solely when the user records
            // with the mic, which is what activates the other capture features.
            let session_id = {
                let active = match state.scribe.active_session.lock() {
                    Ok(a) => *a,
                    Err(e) => {
                        log_message(&format!("[scribe] Lock error: {}", e));
                        continue;
                    }
                };
                active
            };
            let Some(session_id) = session_id else {
                continue;
            };

            // If the user copied the exact text they just dictated, treat it as
            // the same content — don't duplicate it as a "copied" event.
            if state
                .scribe
                .clipboard_matches_recent_transcript(session_id, trimmed)
                .unwrap_or(false)
            {
                log_message(&format!(
                    "[scribe] Skipped clipboard copy identical to recent dictation ({} chars)",
                    trimmed.len()
                ));
                last_capture = Some(text);
                continue;
            }

            let source_app = paste::get_frontmost_window().map(|w| w.name);
            match state.scribe.add_event(
                session_id,
                "clipboard",
                Some(trimmed),
                None,
                source_app.as_deref(),
                None,
            ) {
                Ok(ev) => {
                    last_capture = Some(text);
                    log_message(&format!(
                        "[scribe] Captured clipboard ({}) into session {}",
                        ev.id, session_id
                    ));
                    let _ = app.emit("scribe:event_added", serde_json::json!({ "event": ev }));
                }
                Err(e) => log_message(&format!("[scribe] Clipboard capture failed: {}", e)),
            }
        }
    });
}

/// Register the Scribe screenshot hotkey. Returns Err if the modifier is
/// unsupported or the shortcut collides with an existing one. Safe to call
/// repeatedly (a previously registered scribe shortcut is replaced).
pub fn register_scribe_screenshot_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let manager = app.global_shortcut();
    let shortcut = parse_shortcut(hotkey)?;
    let _ = manager.unregister(shortcut);

    let _app_clone = app.clone();
    if let Err(e) = manager.on_shortcut(shortcut, move |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let app_for_cmd = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app_for_cmd.state::<AppState>();
                if let Err(e) =
                    crate::commands::capture_scribe_screenshot(app_for_cmd.clone(), state).await
                {
                    log_message(&format!("[scribe] Screenshot hotkey error: {}", e));
                }
            });
        }
    }) {
        return Err(format!(
            "Failed to register scribe screenshot hotkey: {}",
            e
        ));
    }
    log_message(&format!(
        "[scribe] Screenshot hotkey registered: {}",
        hotkey
    ));
    Ok(())
}

/// Re-register the scribe screenshot hotkey from current settings. Use after
/// the dictation hotkey re-registration wipes all global shortcuts.
pub fn reregister_scribe_screenshot_hotkey(app: &AppHandle) {
    let hotkey = app
        .state::<AppState>()
        .settings
        .lock()
        .ok()
        .map(|s| s.scribe_screenshot_hotkey.clone())
        .unwrap_or_else(|| "Super+Option+S".to_string());
    if let Err(e) = register_scribe_screenshot_hotkey(app, &hotkey) {
        log_message(&format!(
            "[scribe] Screenshot hotkey re-register failed: {}",
            e
        ));
    }
}

fn parse_shortcut(hotkey: &str) -> Result<Shortcut, String> {
    let normalized = crate::settings::Settings::normalize_hotkey(hotkey);
    normalized
        .parse::<Shortcut>()
        .map_err(|e| format!("Invalid screenshot hotkey {:?}: {}", hotkey, e))
}
