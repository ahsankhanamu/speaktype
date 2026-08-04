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

    const MAX_ATTEMPTS: u32 = 5;
    let mut attempt: u32 = 0;
    let result = loop {
        attempt += 1;
        // Reset cancel flag between attempts unless user already stopped.
        if !cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
            // ok
        }

        let result =
            downloader::download_model_with_cancel(app.clone(), model.clone(), cancel_flag.clone())
                .await;

        match &result {
            Ok(_) => break result,
            Err(e)
                if downloader::classify_download_error(e) == downloader::DownloadErrorKind::UserStop =>
            {
                break result;
            }
            Err(e)
                if downloader::classify_download_error(e)
                    == downloader::DownloadErrorKind::Permanent =>
            {
                break result;
            }
            Err(e) if attempt >= MAX_ATTEMPTS => {
                log_message(&format!(
                    "[queue] Giving up on {} after {} attempts: {}",
                    model, attempt, e
                ));
                break Err(format!(
                    "{} (auto-retry exhausted after {} attempts)",
                    e, MAX_ATTEMPTS
                ));
            }
            Err(e) => {
                let wait_secs = match attempt {
                    1 => 2u64,
                    2 => 4,
                    3 => 8,
                    4 => 16,
                    _ => 30,
                };
                // ±20% jitter
                let jitter = (wait_secs as f64 * 0.2 * (attempt as f64 * 0.37 % 1.0)) as u64;
                let wait = wait_secs.saturating_sub(jitter / 2) + (jitter % (jitter.max(1)));
                let wait = wait.clamp(1, 30);

                log_message(&format!(
                    "[queue] Transient failure for {} (attempt {}/{}): {} — retrying in {}s",
                    model, attempt, MAX_ATTEMPTS, e, wait
                ));

                let have = downloader::get_partial_download_size(&model);
                let total = downloader::expected_size(&model);
                let _ = app.emit(
                    "model:progress",
                    json!({
                        "model": model,
                        "phase": "retrying",
                        "message": format!("Connection lost — retrying in {}s… ({}/{})", wait, attempt, MAX_ATTEMPTS),
                        "attempt": attempt,
                        "max_attempts": MAX_ATTEMPTS,
                        "bytes_downloaded": have,
                        "total_bytes": total,
                        "percent": if total > 0 {
                            (have as f64 / total as f64 * 100.0).round()
                        } else {
                            0.0
                        },
                    }),
                );

                // Stay registered so Pause/Cancel during backoff still works.
                downloader::register_active_download(&model, cancel_flag.clone());
                let mut remaining = wait;
                let mut user_stop: Option<String> = None;
                while remaining > 0 {
                    if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        let is_pause = downloader::take_pause_request(&model);
                        let phase = if is_pause { "paused" } else { "cancelled" };
                        let message = if is_pause {
                            "Download paused — tap Resume to continue"
                        } else {
                            "Download cancelled"
                        };
                        let _ = app.emit(
                            "model:progress",
                            json!({
                                "model": model,
                                "phase": phase,
                                "message": message,
                                "bytes_downloaded": have,
                                "total_bytes": total,
                            }),
                        );
                        user_stop = Some(if is_pause {
                            "Download paused".to_string()
                        } else {
                            "Download cancelled".to_string()
                        });
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    remaining -= 1;
                }
                downloader::unregister_active_download(&model);

                if let Some(stop) = user_stop {
                    break Err(stop);
                }

                cancel_flag.store(false, std::sync::atomic::Ordering::Relaxed);
                continue;
            }
        }
    };

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
            // Re-emit after leaving the active queue so the UI refresh sees Resume,
            // not a stuck "Downloading" state from the race with the first event.
            let have = downloader::get_partial_download_size(&model);
            let total = downloader::expected_size(&model);
            let phase = if e == "Download paused" {
                "paused"
            } else {
                "cancelled"
            };
            let message = if e == "Download paused" {
                "Download paused — tap Resume to continue"
            } else {
                "Download cancelled"
            };
            let _ = app.emit(
                "model:progress",
                json!({
                    "model": model,
                    "phase": phase,
                    "message": message,
                    "bytes_downloaded": have,
                    "total_bytes": total,
                    "percent": if total > 0 {
                        (have as f64 / total as f64 * 100.0).round().min(99.0)
                    } else {
                        0.0
                    },
                }),
            );
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
