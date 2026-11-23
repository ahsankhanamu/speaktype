use crate::commands::AppState;
use crate::logging::log_message;
use crate::templates;
use serde::{Deserialize, Serialize};
use speaktype_session::assemble::assemble;
use speaktype_session::types::{
    Item, ItemPatch, NewItem, Rect, Session, SessionDetail, SessionSummary, Target,
};
use speaktype_session::{bundle, BundleError, RenderedPrompt, Store, StoreError};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

/// Normalised IPC error (spec §6.1): one `{kind, message}` shape.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct IpcError {
    pub kind: String,
    pub message: String,
}

impl IpcError {
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        IpcError {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

impl From<StoreError> for IpcError {
    fn from(e: StoreError) -> Self {
        IpcError::new("store", e.to_string())
    }
}

impl From<BundleError> for IpcError {
    fn from(e: BundleError) -> Self {
        IpcError::new("bundle", e.to_string())
    }
}

impl From<String> for IpcError {
    fn from(e: String) -> Self {
        IpcError::new("session", e)
    }
}

impl From<&str> for IpcError {
    fn from(e: &str) -> Self {
        IpcError::new("session", e)
    }
}

// ─── Typed events ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    pub session: Option<Session>,
    /// "idle" | "active" | "stopping"
    pub phase: String,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SessionItemAdded {
    pub item: Item,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SessionItemUpdated {
    pub item: Item,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SessionItemsDeleted {
    pub ids: Vec<i64>,
}

/// The session list changed (created, stopped, renamed or deleted) — windows
/// showing the list refetch on it.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SessionsChanged {}

// ─── Commands ────────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Where the user-facing bundles live (spec §6.5).
pub(crate) fn sessions_root() -> PathBuf {
    if cfg!(test) {
        return crate::paths::config_dir().join("Sessions");
    }
    dirs::document_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SpeakType")
        .join("Sessions")
}

/// Raw captures keep living in `<config>/scribe/<id>/` (spec §12).
pub(crate) fn session_capture_dir(session_id: i64) -> PathBuf {
    crate::paths::config_dir()
        .join("scribe")
        .join(session_id.to_string())
}

fn auto_session_name(now_ms: i64) -> String {
    // Degenerate ms (outside chrono's range) can only come from a broken
    // clock; the epoch fallback keeps session start deterministic.
    let dt = chrono::DateTime::from_timestamp_millis(now_ms)
        .map(|dt| dt.with_timezone(&chrono::Local))
        .unwrap_or_default();
    format!("Session {}", dt.format("%b %-e %H:%M"))
}

fn phase_for(active: &Option<Session>) -> &'static str {
    if active.is_some() {
        "active"
    } else {
        "idle"
    }
}

/// Emit the session state event. Best-effort: the DB write that preceded it is
/// the source of truth, and a failed notify must not fail the command.
fn emit_state(app: &AppHandle, store: &Store, muted: &AtomicBool) {
    let session = match store.active_session() {
        Ok(session) => session,
        Err(e) => {
            log_message(&format!("[session] active_session for event failed: {e}"));
            None
        }
    };
    let event = SessionState {
        phase: phase_for(&session).to_string(),
        session,
        muted: muted.load(Ordering::SeqCst),
    };
    if let Err(e) = event.emit(app) {
        log_message(&format!("[session] state event emit failed: {e}"));
    }
}

pub(crate) fn emit_item_added(app: &AppHandle, item: &Item) {
    if let Err(e) = (SessionItemAdded { item: item.clone() }).emit(app) {
        log_message(&format!("[session] item added event emit failed: {e}"));
    }
}

fn emit_item_updated(app: &AppHandle, item: &Item) {
    if let Err(e) = (SessionItemUpdated { item: item.clone() }).emit(app) {
        log_message(&format!("[session] item updated event emit failed: {e}"));
    }
}

fn emit_items_deleted(app: &AppHandle, ids: &[i64]) {
    if let Err(e) = (SessionItemsDeleted { ids: ids.to_vec() }).emit(app) {
        log_message(&format!("[session] items deleted event emit failed: {e}"));
    }
}

fn emit_sessions_changed(app: &AppHandle) {
    if let Err(e) = (SessionsChanged {}).emit(app) {
        log_message(&format!(
            "[session] sessions changed event emit failed: {e}"
        ));
    }
}

/// Start a session unless one is already active: starting is idempotent, so a
/// rapid second press returns the active session and emits nothing.
pub(crate) fn start_session(
    store: &Store,
    name: &str,
    template: &str,
    now_ms: i64,
    on_create: impl FnOnce(&Session),
) -> Result<Session, IpcError> {
    let (session, created) = store.start_session(name, template, now_ms)?;
    if created {
        on_create(&session);
    }
    Ok(session)
}

#[tauri::command]
#[specta::specta]
pub fn session_start(
    template: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, Arc<Store>>,
    muted: State<'_, Arc<AtomicBool>>,
) -> Result<Session, IpcError> {
    let default_template = {
        let settings = state
            .settings
            .lock()
            .map_err(|e| IpcError::new("settings", e.to_string()))?;
        let template = settings.scribe_template.clone();
        if template.trim().is_empty() {
            "auto".to_string()
        } else {
            template
        }
    };
    let now = now_ms();
    start_session(
        &store,
        &auto_session_name(now),
        &template.unwrap_or(default_template),
        now,
        |session| {
            emit_state(&app, &store, &muted);
            emit_sessions_changed(&app);
            let dictation_mic = crate::commands::take_dictation_mic(&app);
            crate::narration::start(&app, session.id, dictation_mic);
        },
    )
}

/// End the active session; `after_commit` runs only after the write has
/// committed, so windows reacting to the events always see the committed row.
pub(crate) fn stop_and_notify(
    store: &Store,
    now_ms: i64,
    after_commit: impl FnOnce(&Session) -> Result<(), IpcError>,
) -> Result<Session, IpcError> {
    let active = store
        .active_session()?
        .ok_or_else(|| IpcError::new("session", "No active session"))?;
    let session = store.end_session(active.id, now_ms)?;
    after_commit(&session)?;
    Ok(session)
}

#[tauri::command]
#[specta::specta]
pub fn session_stop(
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    muted: State<'_, Arc<AtomicBool>>,
) -> Result<Session, IpcError> {
    stop_and_notify(&store, now_ms(), |stopped| {
        emit_state(&app, &store, &muted);
        emit_sessions_changed(&app);
        // Stop is instant: the last phrase is transcribed after, then the
        // bundle is written with everything in it.
        let (app, id) = (app.clone(), stopped.id);
        tauri::async_runtime::spawn(async move {
            crate::narration::finish().await;
            let store = app.state::<Arc<Store>>();
            if let Err(e) = bundle_write(&sessions_root(), &store, id) {
                log_message(&format!("[session] Bundle not written: {}", e.message));
            }
            emit_sessions_changed(&app);
        });
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub fn session_active(store: State<'_, Arc<Store>>) -> Result<Option<Session>, IpcError> {
    Ok(store.active_session()?)
}

#[tauri::command]
#[specta::specta]
pub fn session_set_mute(
    is_muted: bool,
    app: AppHandle,
    store: State<'_, Arc<Store>>,
    muted: State<'_, Arc<AtomicBool>>,
) -> Result<(), IpcError> {
    // Stores state only — narration arrives in Phase 2.
    muted.store(is_muted, Ordering::SeqCst);
    log_message(&format!("[session] muted = {is_muted}"));
    emit_state(&app, &store, &muted);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn session_list(
    query: Option<String>,
    store: State<'_, Arc<Store>>,
) -> Result<Vec<SessionSummary>, IpcError> {
    Ok(store.list_sessions(query.as_deref())?)
}

#[tauri::command]
#[specta::specta]
pub fn session_get(
    session_id: i64,
    store: State<'_, Arc<Store>>,
) -> Result<SessionDetail, IpcError> {
    Ok(store.detail(session_id)?)
}

#[tauri::command]
#[specta::specta]
pub fn session_rename(
    session_id: i64,
    name: String,
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<(), IpcError> {
    store.rename_session(session_id, &name)?;
    emit_sessions_changed(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn session_delete(
    session_id: i64,
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<(), IpcError> {
    store.delete_session(session_id)?;
    emit_sessions_changed(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn item_update(
    item_id: i64,
    patch: ItemPatch,
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<Item, IpcError> {
    let item = store.update_item(item_id, patch)?;
    emit_item_updated(&app, &item);
    Ok(item)
}

#[tauri::command]
#[specta::specta]
pub fn item_delete(
    item_ids: Vec<i64>,
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<(), IpcError> {
    store.delete_items(&item_ids)?;
    emit_items_deleted(&app, &item_ids);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn item_restore(
    item_ids: Vec<i64>,
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<(), IpcError> {
    store.restore_items(&item_ids)?;
    for id in &item_ids {
        match store.item(*id) {
            Ok(item) => emit_item_added(&app, &item),
            Err(e) => log_message(&format!("[session] restore notify failed for {id}: {e}")),
        }
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn item_move(
    item_id: i64,
    ord: f64,
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<Item, IpcError> {
    let item = store.move_item(item_id, ord)?;
    emit_item_updated(&app, &item);
    Ok(item)
}

#[tauri::command]
#[specta::specta]
pub fn item_split(
    item_id: i64,
    char_index: u32,
    app: AppHandle,
    store: State<'_, Arc<Store>>,
) -> Result<(Item, Item), IpcError> {
    let index = usize::try_from(char_index)
        .map_err(|_| IpcError::new("session", "charIndex must not be negative"))?;
    let (first, second) = store.split_item(item_id, index)?;
    emit_item_updated(&app, &first);
    emit_item_added(&app, &second);
    Ok((first, second))
}

/// Resolve the preface: the user's own text wins; otherwise the template's
/// prompt, inferring the template from content when it is "auto".
fn resolve_preface(detail: &SessionDetail) -> String {
    if let Some(preface) = detail.session.preface.as_deref() {
        if !preface.trim().is_empty() {
            return preface.to_string();
        }
    }
    match detail.session.template.as_str() {
        "auto" => {
            let content: Vec<&str> = detail
                .items
                .iter()
                .filter_map(|i| i.text.as_deref())
                .collect();
            templates::template_prompt(templates::infer_template(&content.join("\n"))).to_string()
        }
        template => templates::template_prompt(template).to_string(),
    }
}

/// Render the session's prompt for `target` against its bundle directory.
/// Read-only: the bundle is materialised by `bundle_write` (session_stop and,
/// later, the Phase 3 timeline editor), so byte counts reflect whatever is on
/// disk — unwritten images count zero and are named in `warning`.
pub(crate) fn render_prompt(
    root: &Path,
    store: &Store,
    session_id: i64,
    target: Target,
) -> Result<RenderedPrompt, IpcError> {
    let detail = store.detail(session_id)?;
    let dir = bundle::bundle_dir_readonly(root, &detail.session);
    let preface = resolve_preface(&detail);
    Ok(assemble(&detail, target, &preface, &dir))
}

/// Materialise the session's bundle (spec §6.5) and return the rendered
/// prompt with byte counts measured against the files that now exist.
pub(crate) fn bundle_write(
    root: &Path,
    store: &Store,
    session_id: i64,
) -> Result<RenderedPrompt, IpcError> {
    let detail = store.detail(session_id)?;
    let dir = bundle::bundle_dir(root, &detail.session);
    let preface = resolve_preface(&detail);
    let target = detail.session.target.clone();
    let rendered = assemble(&detail, target.clone(), &preface, &dir);
    bundle::write_bundle(root, &detail, &rendered)?;
    // Re-render so byte counts measure the files that now exist; the text is
    // identical because assembly is deterministic.
    Ok(assemble(&detail, target, &preface, &dir))
}

#[tauri::command]
#[specta::specta]
pub fn prompt_render(
    session_id: i64,
    target: Target,
    store: State<'_, Arc<Store>>,
) -> Result<RenderedPrompt, IpcError> {
    render_prompt(&sessions_root(), &store, session_id, target)
}

#[tauri::command]
#[specta::specta]
pub fn prompt_copy(
    session_id: i64,
    target: Target,
    store: State<'_, Arc<Store>>,
) -> Result<(), IpcError> {
    let rendered = render_prompt(&sessions_root(), &store, session_id, target)?;
    crate::commands::copy_to_clipboard(rendered.text).map_err(IpcError::from)
}

#[tauri::command]
#[specta::specta]
pub fn bundle_reveal(session_id: i64, store: State<'_, Arc<Store>>) -> Result<(), IpcError> {
    let session = store.detail(session_id)?.session;
    let dir = bundle::bundle_dir_readonly(&sessions_root(), &session);
    if !dir.exists() {
        return Err(IpcError::new(
            "bundle",
            "This session's folder is written when the session stops.",
        ));
    }
    crate::commands::reveal_path(dir.to_string_lossy().into_owned()).map_err(IpcError::from)
}

/// Phase-0 stub: the overlay windows arrive with Phase 2 (S1).
#[tauri::command]
#[specta::specta]
pub fn overlay_pen(on: bool) -> Result<(), IpcError> {
    log_message(&format!("[overlay] pen {on} (stub)"));
    Ok(())
}

/// The region a mark is captured from: the ink with generous room around it,
/// so the image shows what the ink points at, kept inside the display.
pub(crate) fn mark_region(ink: &Rect, display: &Rect) -> Rect {
    const MIN_W: f64 = 900.0;
    const MIN_H: f64 = 560.0;
    let width = (ink.width * 1.6).max(MIN_W).min(display.width);
    let height = (ink.height * 1.6).max(MIN_H).min(display.height);
    let cx = ink.x + ink.width / 2.0;
    let cy = ink.y + ink.height / 2.0;
    let x = (cx - width / 2.0).clamp(display.x, display.x + display.width - width);
    let y = (cy - height / 2.0).clamp(display.y, display.y + display.height - height);
    Rect {
        x: x.round(),
        y: y.round(),
        width: width.round(),
        height: height.round(),
    }
}

/// A mark is done (the draw key came up and the ink sat still): capture the
/// area around it, ink included, into the active session.
#[tauri::command]
#[specta::specta]
pub async fn overlay_mark_complete(
    app: AppHandle,
    display_id: u32,
    bounds: Rect,
) -> Result<(), IpcError> {
    let store = app.state::<Arc<Store>>();
    let Some(session) = store.active_session()? else {
        return Ok(());
    };
    crate::narration::touch();
    let save = app
        .state::<AppState>()
        .settings
        .lock()
        .map(|s| s.save_marks)
        .unwrap_or(true);
    if !save {
        return Ok(());
    }
    if !crate::permissions::screen_recording_granted() {
        // The first mark asks once; macOS applies the grant after a relaunch.
        crate::permissions::request_screen_recording();
        log_message("[mark] Screen Recording not granted yet — mark not captured");
        return Err(IpcError::new(
            "screenRecording",
            "Allow Screen Recording so marks can be saved",
        ));
    }
    let overlay = app
        .get_webview_window(&format!("overlay-{display_id}"))
        .ok_or_else(|| IpcError::new("mark", "No overlay for that display"))?;
    let scale = overlay
        .scale_factor()
        .map_err(|e| IpcError::new("mark", e.to_string()))?;
    let origin = overlay
        .outer_position()
        .map_err(|e| IpcError::new("mark", e.to_string()))?
        .to_logical::<f64>(scale);
    let size = overlay
        .outer_size()
        .map_err(|e| IpcError::new("mark", e.to_string()))?
        .to_logical::<f64>(scale);
    let ink = Rect {
        x: origin.x + bounds.x,
        y: origin.y + bounds.y,
        width: bounds.width,
        height: bounds.height,
    };
    let display = Rect {
        x: origin.x,
        y: origin.y,
        width: size.width,
        height: size.height,
    };
    let region = mark_region(&ink, &display);

    let dir = session_capture_dir(session.id);
    let stamp = now_ms();
    let full = dir.join(format!("mark-{stamp}.full.png"));
    let small = dir.join(format!("mark-{stamp}.png"));
    let (full_path, small_path) = (full.clone(), small.clone());
    let spec = format!(
        "{},{},{},{}",
        region.x, region.y, region.width, region.height
    );
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        std::fs::create_dir_all(full_path.parent().unwrap_or(Path::new(".")))
            .map_err(|e| e.to_string())?;
        let status = std::process::Command::new("/usr/sbin/screencapture")
            .args(["-x", "-t", "png", "-R", &spec])
            .arg(&full_path)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() || !full_path.exists() {
            return Err("screencapture did not write the mark".into());
        }
        // What the agent sees: at most 1568 px on the long edge.
        speaktype_capture::downscale_png(&full_path, &small_path, 1568)
            .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| IpcError::new("mark", e.to_string()))?
    .map_err(|e| IpcError::new("mark", e))?;

    let item = NewItem {
        kind: speaktype_session::ItemKind::Mark,
        t_start_ms: stamp,
        t_end_ms: None,
        text: None,
        image_path: Some(small.to_string_lossy().into_owned()),
        source_app: crate::paste::get_frontmost_window().map(|w| w.name),
        payload: serde_json::json!({
            "region": region,
            "ink": ink,
            "display": display_id,
            "original": full.to_string_lossy(),
        }),
    };
    let item = store.add_item(session.id, item)?;
    emit_item_added(&app, &item);
    log_message(&format!(
        "[mark] Captured mark {} into session {}",
        item.id, session.id
    ));
    Ok(())
}

/// Append a dictation transcript to the active session (if any). Sessions are
/// no longer closed by dictation (spec §12).
pub(crate) fn add_dictation_item(
    app: &AppHandle,
    store: &Store,
    session_id: i64,
    text: &str,
    t_start_ms: i64,
    t_end_ms: i64,
) {
    let item = NewItem {
        kind: speaktype_session::ItemKind::Speech,
        t_start_ms,
        t_end_ms: Some(t_end_ms),
        text: Some(text.to_string()),
        image_path: None,
        source_app: paste_source_app(),
        payload: serde_json::json!({}),
    };
    match store.add_item(session_id, item) {
        Ok(item) => emit_item_added(app, &item),
        Err(e) => log_message(&format!("[session] dictation capture failed: {e}")),
    }
}

/// The per-session screenshot cap was hit (spec §12) — surfaced as an error so
/// the Phase 2 HUD can show it.
pub(crate) fn screenshot_limit_error(max: usize) -> IpcError {
    IpcError::new(
        "screenshotLimit",
        format!("This session has {max} screenshots, the most it can hold."),
    )
}

/// The ⌥⌘S screenshot path (spec §12): interactive `screencapture -i`, OCR into
/// the item, written through `Store::add_item` on the active session — a no-op
/// when there isn't one. Enforces the per-session screenshot cap.
pub(crate) async fn capture_screenshot(app: AppHandle) -> Result<(), IpcError> {
    let store = app.state::<Arc<Store>>();
    let Ok(Some(session)) = store.active_session() else {
        log_message("[capture] Screenshot hotkey with no active session — ignored");
        return Ok(());
    };

    let max = app
        .state::<AppState>()
        .settings
        .lock()
        .ok()
        .map(|s| s.scribe_max_screenshots)
        .unwrap_or(50);
    let taken = match store.detail(session.id) {
        Ok(detail) => detail
            .items
            .iter()
            .filter(|i| i.kind == speaktype_session::ItemKind::Screenshot)
            .count(),
        Err(e) => {
            log_message(&format!("[capture] Screenshot cap check failed: {e}"));
            return Ok(());
        }
    };
    if taken >= max {
        return Err(screenshot_limit_error(max));
    }

    let session_id = session.id;
    let capture = tokio::task::spawn_blocking(move || {
        crate::capture::capture_interactive_screenshot(session_id)
    })
    .await;
    let (abs_path, dims) = match capture {
        Ok(Ok(captured)) => captured,
        Ok(Err(e)) => {
            log_message(&format!("[capture] Screenshot capture: {e}"));
            return Ok(());
        }
        Err(e) => {
            log_message(&format!("[capture] Screenshot task failed: {e}"));
            return Ok(());
        }
    };

    let ocr_path = abs_path.clone();
    let ocr_text = match tokio::task::spawn_blocking(move || crate::ocr::ocr_image(&ocr_path)).await
    {
        Ok(Ok(ocr)) => ocr,
        Ok(Err(e)) => {
            log_message(&format!("[capture] Screenshot OCR failed: {e}"));
            None
        }
        Err(e) => {
            log_message(&format!("[capture] Screenshot OCR task failed: {e}"));
            None
        }
    };

    let mut payload = serde_json::json!({ "dimensions": dims });
    if let Some(ocr) = &ocr_text {
        payload["ocr"] = serde_json::json!(ocr);
    }
    log_message(&format!(
        "[capture] Screenshot OCR {}",
        ocr_text
            .as_ref()
            .map(|t| format!("{} chars", t.len()))
            .unwrap_or_else(|| "none".into())
    ));

    let item = NewItem {
        kind: speaktype_session::ItemKind::Screenshot,
        t_start_ms: now_ms(),
        t_end_ms: None,
        text: ocr_text,
        image_path: Some(abs_path),
        source_app: paste_source_app(),
        payload,
    };
    match store.add_item(session.id, item) {
        Ok(item) => emit_item_added(&app, &item),
        Err(e) => log_message(&format!("[capture] Screenshot store failed: {e}")),
    }
    Ok(())
}

pub(crate) fn paste_source_app() -> Option<String> {
    crate::paste::get_frontmost_window().map(|w| w.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_mark_is_captured_with_room_around_it_on_screen() {
        let display = Rect {
            x: 0.0,
            y: 0.0,
            width: 1512.0,
            height: 982.0,
        };
        let ink = Rect {
            x: 700.0,
            y: 400.0,
            width: 60.0,
            height: 30.0,
        };
        let region = mark_region(&ink, &display);
        assert_eq!((region.width, region.height), (900.0, 560.0));
        assert!(region.x <= ink.x && region.x + region.width >= ink.x + ink.width);
    }

    #[test]
    fn a_mark_at_the_corner_stays_inside_the_display() {
        let display = Rect {
            x: 0.0,
            y: 0.0,
            width: 1512.0,
            height: 982.0,
        };
        let ink = Rect {
            x: 1480.0,
            y: 960.0,
            width: 20.0,
            height: 15.0,
        };
        let region = mark_region(&ink, &display);
        assert_eq!(region.x + region.width, 1512.0);
        assert_eq!(region.y + region.height, 982.0);
    }
    use serde_json::json;
    use speaktype_session::ItemKind;

    /// A minimal valid 1x1 transparent PNG.
    const ONE_PX_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x04, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0x78,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    fn tempdir() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "speaktype_sessioncmd_test_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).expect("tempdir");
        dir
    }

    fn temp_store(dir: &Path) -> Store {
        Store::open(&dir.join("scribe.db")).expect("store")
    }

    /// A store with one session and one mark backed by a real PNG.
    fn store_with_mark(dir: &Path) -> (Store, Session) {
        let store = temp_store(dir);
        let session = store
            .create_session("Checkout bug", "auto", 1_700_000_000_000)
            .expect("session");
        let captures = session_capture_dir(session.id);
        std::fs::create_dir_all(&captures).expect("captures dir");
        std::fs::write(captures.join("m1.png"), ONE_PX_PNG).expect("png");
        store
            .add_item(
                session.id,
                NewItem {
                    kind: ItemKind::Mark,
                    t_start_ms: 0,
                    t_end_ms: None,
                    text: None,
                    image_path: Some(captures.join("m1.png").to_string_lossy().into_owned()),
                    source_app: None,
                    payload: json!({}),
                },
            )
            .expect("mark");
        (store, session)
    }

    #[test]
    fn session_start_is_idempotent() {
        let dir = tempdir();
        let store = temp_store(&dir);
        let mut created = 0;
        let first = start_session(&store, "One", "auto", 1_700_000_000_000, |_| created += 1)
            .expect("first start");
        let second = start_session(&store, "Two", "thesis", 1_700_000_060_000, |_| created += 1)
            .expect("second start");

        assert_eq!(
            first.id, second.id,
            "second start returns the active session"
        );
        assert_eq!(created, 1, "no create or emit on the idempotent path");

        let conn = rusqlite::Connection::open(dir.join("scribe.db")).expect("second connection");
        let (rows, open): (i64, i64) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(ended_at IS NULL), 0) FROM scribe_sessions",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("count");
        assert_eq!((rows, open), (1, 1), "exactly one row, still open");
    }

    #[test]
    fn session_stop_emits_state_after_commit() {
        let dir = tempdir();
        let store = temp_store(&dir);
        let session = store
            .create_session("S", "auto", 1_700_000_000_000)
            .expect("session");

        let mut emitted = false;
        let stopped = stop_and_notify(&store, 1_700_000_060_000, |stopped| {
            // When the event fires, the write must already be visible from a
            // second connection.
            let conn =
                rusqlite::Connection::open(dir.join("scribe.db")).expect("second connection");
            let ended: Option<String> = conn
                .query_row(
                    "SELECT ended_at FROM scribe_sessions WHERE id = ?1",
                    [stopped.id],
                    |r| r.get(0),
                )
                .expect("read ended_at");
            assert!(ended.is_some(), "emit ran before the commit was visible");
            assert_eq!(stopped.id, session.id);
            emitted = true;
            Ok(())
        })
        .expect("stop");

        assert!(emitted, "the emitter must run");
        assert_eq!(stopped.ended_at_ms, Some(1_700_000_060_000));
    }

    #[test]
    fn stop_without_active_session_errors() {
        let dir = tempdir();
        let store = temp_store(&dir);
        assert!(stop_and_notify(&store, 0, |_| Ok(())).is_err());
    }

    #[test]
    fn prompt_render_is_read_only() {
        let dir = tempdir();
        let root = dir.join("Sessions");
        let (store, session) = store_with_mark(&dir);

        let rendered =
            render_prompt(&root, &store, session.id, Target::CodingAgent).expect("render");
        let dir_on_disk = bundle::bundle_dir_readonly(&root, &session);
        assert!(
            !dir_on_disk.exists(),
            "render must not create the bundle directory"
        );
        assert_eq!(rendered.image_count, 1);
        assert_eq!(
            rendered.total_image_bytes, 0,
            "unwritten bundle images count zero bytes"
        );
        assert_eq!(rendered.warning.as_deref(), Some("Image 1 is missing."));
        assert!(rendered.text.contains("images/01-mark.png"));
    }

    #[test]
    fn stop_writes_the_bundle_after_commit() {
        let dir = tempdir();
        let root = dir.join("Sessions");
        let (store, _session) = store_with_mark(&dir);

        let stopped = stop_and_notify(&store, 1_700_000_060_000, |stopped| {
            bundle_write(&root, &store, stopped.id).map(|_| ())
        })
        .expect("stop");

        let bundle_dir =
            bundle::bundle_dir(&root, &store.detail(stopped.id).expect("detail").session);
        assert!(
            bundle_dir.join("images/01-mark.png").is_file(),
            "image written"
        );
        let rendered =
            render_prompt(&root, &store, stopped.id, Target::CodingAgent).expect("render");
        assert_eq!(
            std::fs::read_to_string(bundle_dir.join("prompt.md")).expect("prompt.md"),
            rendered.text
        );
        let size = std::fs::metadata(bundle_dir.join("images/01-mark.png"))
            .expect("metadata")
            .len();
        assert_eq!(rendered.warning, None, "written images are not missing");
        assert_eq!(rendered.total_image_bytes, size);
    }

    #[test]
    fn screenshot_limit_error_shape() {
        let e = screenshot_limit_error(50);
        assert_eq!(e.kind, "screenshotLimit");
        assert_eq!(
            e.message,
            "This session has 50 screenshots, the most it can hold."
        );
    }
}
