use crate::downloader;
use crate::logging::log_message;
use crate::server;
use crate::settings::Settings;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter};

const MAX_CONCURRENT: usize = 2;

#[derive(Clone)]
struct QueueEntry {
    model: String,
    restart: bool,
    load_after: bool,
}

struct QueueState {
    waiting: VecDeque<QueueEntry>,
    active: HashMap<String, Arc<AtomicBool>>,
}

static QUEUE: LazyLock<Mutex<QueueState>> = LazyLock::new(|| {
    Mutex::new(QueueState {
        waiting: VecDeque::new(),
        active: HashMap::new(),
    })
});

pub fn queue_status() -> (Vec<String>, Vec<String>) {
    let q = QUEUE.lock().ok();
    match q {
        Some(q) => (
            q.active.keys().cloned().collect(),
            q.waiting.iter().map(|e| e.model.clone()).collect(),
        ),
        None => (Vec::new(), Vec::new()),
    }
}

fn emit_queue_update(app: &AppHandle) {
    let (active, waiting) = queue_status();
    let _ = app.emit(
        "model:queue",
        json!({ "active": active, "waiting": waiting }),
    );
}

fn emit_waiting(app: &AppHandle, model: &str, position: usize) {
    let _ = app.emit(
        "model:progress",
        json!({
            "model": model,
            "phase": "waiting",
            "message": format!("Waiting (#{} in queue)", position),
            "queue_position": position,
        }),
    );
}

pub fn enqueue(
    app: AppHandle,
    settings: Arc<Mutex<Settings>>,
    model: String,
    restart: bool,
    load_after: bool,
) -> Result<serde_json::Value, String> {
    if !restart && downloader::is_model_complete(&model) {
        return Ok(json!({ "status": "complete", "model": model }));
    }

    let mut q = QUEUE.lock().map_err(|e| e.to_string())?;

    if q.active.contains_key(&model) {
        return Ok(json!({ "status": "downloading", "model": model }));
    }

    if let Some(pos) = q.waiting.iter().position(|e| e.model == model) {
        return Ok(json!({
            "status": "queued",
            "model": model,
            "position": pos + 1,
        }));
    }

    q.waiting.push_back(QueueEntry {
        model: model.clone(),
        restart,
        load_after,
    });

    let position = q.waiting.len();
    drop(q);

    log_message(&format!(
        "[queue] Enqueued {} (restart={}, load_after={}, position={})",
        model, restart, load_after, position
    ));
    emit_waiting(&app, &model, position);
    emit_queue_update(&app);

    try_start_next(app, settings);
    Ok(json!({ "status": "queued", "model": model, "position": position }))
}

/// Remove a model from the waiting queue (not an in-progress download).
pub fn cancel_waiting_with_app(app: &AppHandle, model: &str) -> bool {
    let removed = if let Ok(mut q) = QUEUE.lock() {
        let before = q.waiting.len();
        q.waiting.retain(|e| e.model != model);
        q.waiting.len() != before
    } else {
        false
    };

    if removed {
        log_message(&format!("[queue] Removed {} from waiting queue", model));
        emit_queue_update(app);
        let _ = app.emit(
            "model:progress",
            json!({
                "model": model,
                "phase": "cancelled",
                "message": "Removed from queue",
            }),
        );
    }
    removed
}

/// Pause an active download. Progress is kept on disk and can be resumed later.
pub fn pause_with_app(app: &AppHandle, model: &str) -> bool {
    let paused = if let Ok(q) = QUEUE.lock() {
        if q.active.contains_key(model) {
            downloader::pause_download(model);
            log_message(&format!("[queue] Paused download: {}", model));
            true
        } else {
            false
        }
    } else {
        false
    };

    if paused {
        emit_queue_update(app);
    }
    paused
}

fn try_start_next(app: AppHandle, settings: Arc<Mutex<Settings>>) {
    loop {
        let slot = {
            let mut q = match QUEUE.lock() {
                Ok(q) => q,
                Err(_) => return,
            };
            if q.active.len() >= MAX_CONCURRENT || q.waiting.is_empty() {
                return;
            }
            let entry = q.waiting.pop_front().unwrap();
            let cancel_flag = Arc::new(AtomicBool::new(false));
            q.active.insert(entry.model.clone(), cancel_flag.clone());
            (entry, cancel_flag)
        };

        let (entry, cancel_flag) = slot;
        let model = entry.model.clone();
        let app_clone = app.clone();
        let settings_clone = settings.clone();

        log_message(&format!("[queue] Starting download slot for {}", model));
        emit_queue_update(&app);

        tauri::async_runtime::spawn(async move {
            run_download(app_clone, settings_clone, entry, cancel_flag).await;
        });
    }
}

async fn run_download(
    app: AppHandle,
    settings: Arc<Mutex<Settings>>,
    entry: QueueEntry,
    cancel_flag: Arc<AtomicBool>,
) {
    let model = entry.model.clone();

    if entry.restart {
        let _ = downloader::clear_model_files(&model);
    }

    let result = downloader::download_model_with_cancel(app.clone(), model.clone(), cancel_flag).await;

    {
        if let Ok(mut q) = QUEUE.lock() {
            q.active.remove(&model);
        }
    }
    emit_queue_update(&app);

    match result {
        Ok(_) => {
            log_message(&format!("[queue] Download complete: {}", model));
            if entry.load_after {
                load_model_after_download(&app, &settings, &model).await;
            } else {
                let _ = app.emit(
                    "model:progress",
                    json!({
                        "model": model,
                        "phase": "done",
                        "message": format!("{} downloaded", model),
                    }),
                );
            }
        }
        Err(e) if e == "Download paused" || e == "Download cancelled" => {
            // Progress event already emitted by the downloader.
        }
        Err(e) => {
            let _ = app.emit(
                "model:progress",
                json!({ "model": model, "phase": "error", "message": e }),
            );
        }
    }

    try_start_next(app, settings);
}

async fn load_model_after_download(
    app: &AppHandle,
    settings: &Arc<Mutex<Settings>>,
    model: &str,
) {
    let _ = app.emit(
        "model:progress",
        json!({
            "model": model,
            "phase": "loading",
            "message": format!("Loading {} into memory...", model),
        }),
    );

    server::stop_server();
    if let Ok(mut s) = settings.lock() {
        s.model = model.to_string();
        let _ = s.save();
    }

    if let Some(info) = server::start_server(app, model) {
        let port = info.port;
        if server::wait_for_server(port).await {
            if let Ok(mut s) = settings.lock() {
                s.api_url = format!("http://127.0.0.1:{}/inference", port);
                let _ = s.save();
            }
            let _ = app.emit(
                "model:progress",
                json!({
                    "model": model,
                    "phase": "done",
                    "message": format!("{} model ready", model),
                }),
            );
            return;
        }
        server::stop_server();
    }

    let _ = app.emit(
        "model:progress",
        json!({
            "model": model,
            "phase": "error",
            "message": "Downloaded but failed to start server",
        }),
    );
}
