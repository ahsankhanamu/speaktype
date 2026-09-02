use crate::logging;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, RANGE};
use reqwest::Client;
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter};

const MODEL_REPO_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

static ACTIVE_DOWNLOADS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static PAUSE_REQUESTS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn cleanup_download(model_name: &str) {
    if let Ok(mut downloads) = ACTIVE_DOWNLOADS.lock() {
        downloads.remove(model_name);
    }
}

/// Download using an externally owned cancel flag (for the download queue).
pub async fn download_model_with_cancel(
    app: AppHandle,
    model_name: String,
    cancel_flag: Arc<AtomicBool>,
) -> Result<PathBuf, String> {
    if let Ok(mut downloads) = ACTIVE_DOWNLOADS.lock() {
        downloads.insert(model_name.clone(), cancel_flag);
    }

    let result = download_model_inner(app, model_name.clone()).await;
    cleanup_download(&model_name);
    result
}

async fn download_model_inner(
    app: AppHandle,
    model_name: String,
) -> Result<PathBuf, String> {
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let filename = format!("ggml-{}.bin", model_name);
    let url = format!("{}/{}", MODEL_REPO_BASE, filename);

    let data_dir = crate::paths::config_dir_opt()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

    let models_dir = data_dir.join("models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

    let file_path = models_dir.join(&filename);
    let temp_path = models_dir.join(format!("{}.part", filename));

    // Known size hint (segmented path discovers authoritative size via Range).
    let known = expected_size(&model_name);
    if known > 0 {
        save_expected_size(&model_name, known);
    }

    // Finished .bin already on disk.
    if file_path.exists() {
        let bin_size = std::fs::metadata(&file_path).map_err(|e| e.to_string())?.len();
        let total_hint = expected_size(&model_name);
        if total_hint > 0 && bin_size >= total_hint {
            remove_expected_size(&model_name);
            crate::segmented_download::remove_parts_file(&model_name);
            logging::log_message(&format!("[downloader] Model {} already fully downloaded.", model_name));
            let _ = app.emit("model:progress", serde_json::json!({
                "model": model_name,
                "percent": 100,
                "bytes_downloaded": bin_size,
                "total_bytes": total_hint,
                "speed": 0,
                "eta_secs": 0,
                "phase": "done"
            }));
            return Ok(file_path);
        }
        if bin_size > 0 && !temp_path.exists() {
            std::fs::rename(&file_path, &temp_path).map_err(|e| e.to_string())?;
            logging::log_message(&format!(
                "[downloader] Found incomplete {} ({} bytes) — resuming download",
                model_name, bin_size
            ));
        }
    }

    let cancel_flag = ACTIVE_DOWNLOADS
        .lock()
        .ok()
        .and_then(|d| d.get(&model_name).cloned())
        .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));

    // Prefer sliding-window segmented download; fall back to single-stream.
    match crate::segmented_download::download_segmented(
        app.clone(),
        &client,
        model_name.clone(),
        url.clone(),
        file_path.clone(),
        temp_path.clone(),
        cancel_flag.clone(),
    )
    .await
    {
        Ok(path) => Ok(path),
        Err(crate::segmented_download::SegmentedError::FallBackToSingle(reason)) => {
            logging::log_message(&format!(
                "[downloader] Segmented fallback for {}: {}",
                model_name, reason
            ));
            let mut downloaded = 0u64;
            if temp_path.exists() {
                // Prefer parts.json completed sum when present (sparse file length is not progress).
                downloaded = crate::segmented_download::completed_bytes_from_parts(&model_name)
                    .unwrap_or_else(|| {
                        std::fs::metadata(&temp_path)
                            .map(|m| m.len())
                            .unwrap_or(0)
                    });
                // Single-stream path expects contiguous prefix — if we only have
                // a sparse/partial map, truncate to contiguous prefix from parts.
                if let Some(prefix) = contiguous_prefix_len(&model_name) {
                    downloaded = prefix;
                    // Rewrite as contiguous file for append-based single stream.
                    if prefix == 0 {
                        let _ = std::fs::remove_file(&temp_path);
                    } else {
                        truncate_part_to_prefix(&temp_path, prefix)?;
                    }
                }
            }
            let total_size = expected_size(&model_name);
            if total_size > 0 && downloaded >= total_size {
                std::fs::rename(&temp_path, &file_path).map_err(|e| e.to_string())?;
                remove_expected_size(&model_name);
                crate::segmented_download::remove_parts_file(&model_name);
                return Ok(file_path);
            }
            download_inner_single(
                app,
                model_name,
                filename,
                url,
                file_path,
                temp_path,
                downloaded,
                total_size,
                cancel_flag,
                &client,
            )
            .await
        }
        Err(crate::segmented_download::SegmentedError::Failed(e)) => Err(e),
    }
}

fn contiguous_prefix_len(model_name: &str) -> Option<u64> {
    let dir = crate::paths::config_dir_opt()?.join("models");
    let path = dir.join(format!("ggml-{}.parts.json", model_name));
    if !path.exists() {
        return None;
    }
    let text = std::fs::read_to_string(&path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let completed = v.get("completed")?.as_array()?;
    let mut ranges: Vec<(u64, u64)> = completed
        .iter()
        .filter_map(|r| {
            let a = r.as_array()?;
            Some((a.first()?.as_u64()?, a.get(1)?.as_u64()?))
        })
        .collect();
    ranges.sort_by_key(|r| r.0);
    let mut prefix = 0u64;
    for (s, e) in ranges {
        if s > prefix {
            break;
        }
        prefix = prefix.max(e);
    }
    Some(prefix)
}

fn truncate_part_to_prefix(temp_path: &PathBuf, prefix: u64) -> Result<(), String> {
    use std::io::{Read, Write};
    let data = {
        let mut f = std::fs::File::open(temp_path).map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; prefix as usize];
        f.read_exact(&mut buf).map_err(|e| e.to_string())?;
        buf
    };
    let mut f = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(temp_path)
        .map_err(|e| e.to_string())?;
    f.write_all(&data).map_err(|e| e.to_string())?;
    f.set_len(prefix).map_err(|e| e.to_string())?;
    Ok(())
}

async fn download_inner_single(
    app: AppHandle,
    model_name: String,
    _filename: String,
    url: String,
    file_path: PathBuf,
    temp_path: PathBuf,
    mut downloaded: u64,
    total_size: u64,
    cancel_flag: Arc<AtomicBool>,
    client: &Client,
) -> Result<PathBuf, String> {
    let mut headers = HeaderMap::new();
    let resume = downloaded > 0 && total_size > 0;
    if resume {
        let range_val = format!("bytes={}-", downloaded);
        headers.insert(RANGE, HeaderValue::from_str(&range_val).map_err(|e| e.to_string())?);
        logging::log_message(&format!("[downloader] Resuming {} from {}/{} bytes ({:.1}%)",
            model_name, downloaded, total_size, (downloaded as f64 / total_size as f64) * 100.0));
    } else {
        downloaded = 0;
        let _ = std::fs::remove_file(&temp_path);
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&temp_path)
        .map_err(|e| e.to_string())?;

    let res = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("Failed to download model (status: {})", res.status()));
    }

    // If we fell back mid-way, refresh total from Content-Length when possible.
    let mut total_size = total_size;
    if total_size == 0 {
        if let Some(cl) = res
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            total_size = downloaded + cl;
            save_expected_size(&model_name, total_size);
        }
    }

    let mut stream = res.bytes_stream();
    let mut last_progress_emit = std::time::Instant::now();
    let start_time = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        if cancel_flag.load(Ordering::Relaxed) {
            let is_pause = take_pause_request(&model_name);
            let phase = if is_pause { "paused" } else { "cancelled" };
            let message = if is_pause {
                "Download paused — tap Resume to continue"
            } else {
                "Download cancelled"
            };
            logging::log_message(&format!(
                "[downloader] Download {} for {}",
                if is_pause { "paused" } else { "cancelled" },
                model_name
            ));
            let _ = app.emit("model:progress", serde_json::json!({
                "model": model_name,
                "percent": if total_size > 0 {
                    (downloaded as f64 / total_size as f64 * 100.0).round()
                } else {
                    0.0
                },
                "bytes_downloaded": downloaded,
                "total_bytes": total_size,
                "speed": 0,
                "eta_secs": 0,
                "phase": phase,
                "message": message,
            }));
            return Err(if is_pause {
                "Download paused".to_string()
            } else {
                "Download cancelled".to_string()
            });
        }

        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let elapsed = start_time.elapsed().as_secs_f64();
        if last_progress_emit.elapsed().as_millis() > 150 && elapsed > 0.0 {
            let percent = if total_size > 0 {
                (downloaded as f64 / total_size as f64 * 100.0).round()
            } else {
                0.0
            };
            let speed = if elapsed > 0.0 {
                (downloaded as f64 / 1024.0 / 1024.0) / elapsed
            } else {
                0.0
            };
            let remaining = total_size.saturating_sub(downloaded);
            let eta_secs = if speed > 0.0 {
                (remaining as f64 / 1024.0 / 1024.0 / speed).round() as u64
            } else {
                0
            };

            let _ = app.emit("model:progress", serde_json::json!({
                "model": model_name,
                "percent": percent,
                "bytes_downloaded": downloaded,
                "total_bytes": total_size,
                "speed_mbps": (speed * 100.0).round() / 100.0,
                "eta_secs": eta_secs,
                "phase": "downloading"
            }));
            last_progress_emit = std::time::Instant::now();
        }
    }

    if total_size > 0 && downloaded < total_size {
        return Err(format!(
            "Incomplete download: {} / {} bytes",
            downloaded, total_size
        ));
    }

    std::fs::rename(&temp_path, &file_path).map_err(|e| e.to_string())?;
    remove_expected_size(&model_name);
    crate::segmented_download::remove_parts_file(&model_name);

    let _ = app.emit("model:progress", serde_json::json!({
        "model": model_name,
        "percent": 100,
        "bytes_downloaded": downloaded,
        "total_bytes": total_size,
        "speed": 0,
        "eta_secs": 0,
        "phase": "done"
    }));

    logging::log_message(&format!("[downloader] Model {} downloaded to {} ({:.1} MB)",
        model_name, file_path.display(), downloaded as f64 / 1024.0 / 1024.0));

    Ok(file_path)
}

pub(crate) fn take_pause_request(model_name: &str) -> bool {
    PAUSE_REQUESTS
        .lock()
        .ok()
        .map(|mut pause| pause.remove(model_name))
        .unwrap_or(false)
}

pub fn is_download_active(model_name: &str) -> bool {
    ACTIVE_DOWNLOADS
        .lock()
        .ok()
        .map(|downloads| downloads.contains_key(model_name))
        .unwrap_or(false)
}

pub fn pause_download(model_name: &str) -> bool {
    if let Ok(mut pause) = PAUSE_REQUESTS.lock() {
        pause.insert(model_name.to_string());
    }
    cancel_download(model_name)
}

pub fn cancel_download(model_name: &str) -> bool {
    if let Ok(downloads) = ACTIVE_DOWNLOADS.lock() {
        if let Some(flag) = downloads.get(model_name) {
            flag.store(true, Ordering::Relaxed);
            logging::log_message(&format!("[downloader] Cancel requested for {}", model_name));
            return true;
        }
    }
    false
}

/// Remove all on-disk artifacts for a model so a download can start from scratch.
pub fn clear_model_files(model_name: &str) -> Result<(), String> {
    cancel_download(model_name);
    let Some(dir) = crate::paths::config_dir_opt().map(|d| d.join("models")) else {
        return Err("Could not determine config directory".to_string());
    };
    for suffix in ["bin", "bin.part", "expected", "bin.incomplete", "parts.json"] {
        let path = dir.join(format!("ggml-{}.{}", model_name, suffix));
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            logging::log_message(&format!("[downloader] Removed {}", path.display()));
        }
    }
    Ok(())
}

/// Keep a cancel flag registered while the queue is in a retry backoff sleep.
pub fn register_active_download(model_name: &str, cancel_flag: Arc<AtomicBool>) {
    if let Ok(mut downloads) = ACTIVE_DOWNLOADS.lock() {
        downloads.insert(model_name.to_string(), cancel_flag);
    }
}

pub fn unregister_active_download(model_name: &str) {
    cleanup_download(model_name);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadErrorKind {
    UserStop,
    Permanent,
    Transient,
}

/// Classify download failures for queue-level auto-retry.
pub fn classify_download_error(err: &str) -> DownloadErrorKind {
    let lower = err.to_lowercase();
    if err == "Download paused" || err == "Download cancelled" {
        return DownloadErrorKind::UserStop;
    }
    if lower.contains("not enough disk space")
        || lower.contains("no space")
        || lower.contains("status: 404")
        || lower.contains("(status: 401)")
        || lower.contains("(status: 403)")
        || lower.contains("etag mismatch")
    {
        return DownloadErrorKind::Permanent;
    }
    DownloadErrorKind::Transient
}

pub fn get_downloading_models() -> Vec<String> {
    if let Ok(downloads) = ACTIVE_DOWNLOADS.lock() {
        downloads.keys().cloned().collect()
    } else {
        Vec::new()
    }
}

/// Live byte progress for in-flight downloads (for UI hydration on tab open).
pub fn get_live_download_progress() -> HashMap<String, (u64, u64)> {
    // Progress is event-driven; this reads current .part size as best-effort snapshot.
    let mut out = HashMap::new();
    if let Ok(downloads) = ACTIVE_DOWNLOADS.lock() {
        for model in downloads.keys() {
            let have = get_partial_download_size(model);
            let total = expected_size(model);
            if have > 0 || total > 0 {
                out.insert(model.clone(), (have, total));
            }
        }
    }
    out
}

pub fn get_model_size(model_name: &str) -> u64 {
    let filename = format!("ggml-{}.bin", model_name);
    let data_dir = dirs::config_dir()
        .map(|d| d.join("speaktype").join("models").join(&filename));
    match data_dir {
        Some(path) if path.exists() => {
            std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
        }
        _ => 0
    }
}

/// Known full sizes (bytes) of the ggml whisper models on Hugging Face.
/// Used to detect truncated/incomplete `.bin` files when no `.expected`
/// sidecar is present (e.g. a download that was interrupted in an older build).
fn known_model_size(model_name: &str) -> u64 {
    match model_name {
        "tiny" => 77_691_713,
        "tiny.en" => 77_704_715,
        "base" => 147_951_465,
        "base.en" => 147_964_211,
        "small" => 487_601_967,
        "small.en" => 487_614_201,
        "medium" => 1_533_763_059,
        "medium.en" => 1_533_774_781,
        "large-v1" | "large-v2" => 3_094_623_691,
        "large-v3" => 3_095_033_483,
        "large-v3-turbo" => 1_624_555_275,
        _ => 0,
    }
}

/// Best-known expected total size for a model: the `.expected` sidecar (the real
/// Content-Length recorded during download) if present, else the known table.
pub fn expected_size(model_name: &str) -> u64 {
    let from_file = get_expected_model_size(model_name);
    if from_file > 0 {
        from_file
    } else {
        known_model_size(model_name)
    }
}

const SERVER_MODEL_FALLBACKS: &[&str] = &[
    "medium",
    "small",
    "base",
    "tiny",
    "medium.en",
    "small.en",
    "base.en",
    "tiny.en",
    "large-v3-turbo",
    "large-v3",
];

/// Pick a model that is safe to load: the preferred one if complete, otherwise
/// the best locally available complete model.
pub fn resolve_model_for_server(preferred: &str) -> Option<String> {
    if is_model_complete(preferred) {
        return Some(preferred.to_string());
    }

    let actual = get_model_size(preferred);
    if actual > 0 {
        logging::log_message(&format!(
            "[downloader] Model '{}' is incomplete ({} bytes, expected ~{} bytes)",
            preferred,
            actual,
            expected_size(preferred)
        ));
    }

    for model in SERVER_MODEL_FALLBACKS {
        if *model != preferred && is_model_complete(model) {
            logging::log_message(&format!(
                "[downloader] Falling back from '{}' to complete model '{}'",
                preferred, model
            ));
            return Some((*model).to_string());
        }
    }

    None
}

/// Whether a model's `.bin` exists AND is fully downloaded (size matches expected
/// within a small tolerance). A truncated file is NOT considered complete.
pub fn is_model_complete(model_name: &str) -> bool {
    let actual = get_model_size(model_name);
    if actual == 0 {
        return false;
    }
    let expected = expected_size(model_name);
    if expected == 0 {
        // Unknown expected size — best effort: assume a present file is usable.
        return true;
    }
    // Allow a 2MB slack in case HF re-packs a model with a tiny size delta.
    actual + 2 * 1024 * 1024 >= expected
}

/// Returns bytes already on disk for an incomplete model (.part, truncated .bin,
/// or a quarantined .bin.incomplete file).
pub fn get_partial_download_size(model_name: &str) -> u64 {
    // Segmented downloads: only trust completed ranges (sparse .part length is total size).
    if let Some(from_parts) = crate::segmented_download::completed_bytes_from_parts(model_name) {
        return from_parts;
    }

    let Some(dir) = crate::paths::config_dir_opt().map(|d| d.join("models")) else {
        return 0;
    };

    let expected = expected_size(model_name);
    let mut best = 0u64;
    for suffix in ["bin.part", "bin.incomplete", "bin"] {
        let path = dir.join(format!("ggml-{}.{}", model_name, suffix));
        if path.exists() {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            if suffix == "bin" && is_model_complete(model_name) {
                continue;
            }
            // Preallocated sparse .part reports full logical size with no parts.json yet.
            if suffix == "bin.part" && expected > 0 && size >= expected {
                continue;
            }
            best = best.max(size);
        }
    }
    best
}

/// True when on-disk artifacts mean a download can be resumed (even if byte
/// progress is still 0, e.g. sparse preallocate before the first chunk landed).
pub fn has_partial_download(model_name: &str) -> bool {
    if is_model_complete(model_name) {
        return false;
    }
    let Some(dir) = crate::paths::config_dir_opt().map(|d| d.join("models")) else {
        return false;
    };
    let parts = dir.join(format!("ggml-{}.parts.json", model_name));
    let part = dir.join(format!("ggml-{}.bin.part", model_name));
    let incomplete = dir.join(format!("ggml-{}.bin.incomplete", model_name));
    let bin = dir.join(format!("ggml-{}.bin", model_name));
    parts.exists()
        || part.exists()
        || incomplete.exists()
        || (bin.exists() && get_model_size(model_name) > 0)
}

/// Returns the expected total size for a model from the .expected file, or 0.
pub fn get_expected_model_size(model_name: &str) -> u64 {
    let filename = format!("ggml-{}.expected", model_name);
    let path = crate::paths::config_dir_opt().map(|d| d.join("models").join(&filename));
    match path {
        Some(path) if path.exists() => {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0)
        }
        _ => 0
    }
}

/// Save expected model size to a .expected sidecar file.
pub fn save_expected_size(model_name: &str, total_size: u64) {
    let filename = format!("ggml-{}.expected", model_name);
    if let Some(dir) = crate::paths::config_dir_opt() {
        let path = dir.join("models").join(&filename);
        let _ = std::fs::write(&path, total_size.to_string());
    }
}

/// Remove the .expected sidecar file (called when download completes).
pub fn remove_expected_size(model_name: &str) {
    let filename = format!("ggml-{}.expected", model_name);
    if let Some(dir) = crate::paths::config_dir_opt() {
        let path = dir.join("models").join(&filename);
        let _ = std::fs::remove_file(&path);
    }
}

pub const VAD_MODEL_FILE: &str = "ggml-silero-v6.2.0.bin";
const VAD_MODEL_URL: &str =
    "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v6.2.0.bin";

pub fn vad_model_path() -> Option<PathBuf> {
    crate::paths::config_dir_opt().map(|d| d.join("models").join(VAD_MODEL_FILE))
}

pub fn is_vad_model_ready() -> bool {
    vad_model_path()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len() > 1024)
        .unwrap_or(false)
}

/// Download the Silero VAD model used by whisper.cpp (blocking, ~1MB).
pub fn ensure_vad_model_blocking() -> Option<PathBuf> {
    if is_vad_model_ready() {
        return vad_model_path();
    }

    let path = vad_model_path()?;
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return None;
        }
    }

    let temp_path = path.with_extension("bin.part");
    logging::log_message("[vad] Downloading Silero VAD model...");

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            logging::log_message(&format!("[vad] Failed to create HTTP client: {}", e));
            return None;
        }
    };

    let response = match client.get(VAD_MODEL_URL).send() {
        Ok(r) => r,
        Err(e) => {
            logging::log_message(&format!("[vad] Download failed: {}", e));
            return None;
        }
    };

    if !response.status().is_success() {
        logging::log_message(&format!(
            "[vad] Download failed: HTTP {}",
            response.status()
        ));
        return None;
    }

    let bytes = match response.bytes() {
        Ok(b) => b,
        Err(e) => {
            logging::log_message(&format!("[vad] Failed to read VAD model body: {}", e));
            return None;
        }
    };

    if bytes.len() < 1024 {
        logging::log_message("[vad] Downloaded VAD model looks too small — ignoring");
        return None;
    }

    if std::fs::write(&temp_path, &bytes).is_err() {
        return None;
    }

    if std::fs::rename(&temp_path, &path).is_err() {
        let _ = std::fs::remove_file(&temp_path);
        return None;
    }

    logging::log_message(&format!(
        "[vad] Silero VAD model ready ({}KB)",
        bytes.len() / 1024
    ));
    Some(path)
}
