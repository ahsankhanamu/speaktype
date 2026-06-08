use crate::logging;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, RANGE};
use reqwest::Client;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter};

const MODEL_REPO_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

static ACTIVE_DOWNLOADS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn cleanup_download(model_name: &str) {
    if let Ok(mut downloads) = ACTIVE_DOWNLOADS.lock() {
        downloads.remove(model_name);
    }
}

pub async fn download_model(
    app: AppHandle,
    model_name: String,
) -> Result<PathBuf, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300))
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

    // 1. Ask the server how big the complete file is.
    let head_res = client.head(&url).send().await.map_err(|e| e.to_string())?;
    if !head_res.status().is_success() {
        return Err(format!("Failed to fetch model info (status: {})", head_res.status()));
    }

    let total_size = head_res
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|val| val.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if total_size > 0 {
        save_expected_size(&model_name, total_size);
    }

    // 2. A finished .bin already on disk → nothing to do.
    if file_path.exists() {
        let bin_size = std::fs::metadata(&file_path).map_err(|e| e.to_string())?.len();
        if total_size > 0 && bin_size >= total_size {
            remove_expected_size(&model_name);
            logging::log_message(&format!("[downloader] Model {} already fully downloaded.", model_name));
            let _ = app.emit("model:progress", serde_json::json!({
                "model": model_name,
                "percent": 100,
                "bytes_downloaded": bin_size,
                "total_bytes": total_size,
                "speed": 0,
                "eta_secs": 0,
                "phase": "done"
            }));
            return Ok(file_path);
        }
        // 3. A truncated .bin (interrupted in an older build) and no .part yet:
        //    move it to .part so we can resume instead of restarting from zero.
        if bin_size > 0 && !temp_path.exists() {
            std::fs::rename(&file_path, &temp_path).map_err(|e| e.to_string())?;
            logging::log_message(&format!(
                "[downloader] Found incomplete {} ({} bytes) — resuming download",
                model_name, bin_size
            ));
        }
    }

    // 4. Resume point is whatever is already in .part.
    let mut downloaded = 0u64;
    if temp_path.exists() {
        downloaded = std::fs::metadata(&temp_path).map_err(|e| e.to_string())?.len();
    }

    // .part is already complete → finalize.
    if total_size > 0 && downloaded >= total_size {
        std::fs::rename(&temp_path, &file_path).map_err(|e| e.to_string())?;
        remove_expected_size(&model_name);
        let _ = app.emit("model:progress", serde_json::json!({
            "model": model_name,
            "percent": 100,
            "bytes_downloaded": downloaded,
            "total_bytes": total_size,
            "speed": 0,
            "eta_secs": 0,
            "phase": "done"
        }));
        return Ok(file_path);
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut downloads) = ACTIVE_DOWNLOADS.lock() {
        downloads.insert(model_name.clone(), cancel_flag.clone());
    }

    let model_name_clone = model_name.clone();
    let result = download_inner(
        app, model_name, filename, url, models_dir,
        file_path, temp_path, downloaded, total_size, cancel_flag, &client,
    ).await;

    // Cleanup ACTIVE_DOWNLOADS entry on all exit paths (success, error, cancel)
    cleanup_download(&model_name_clone);

    result
}

async fn download_inner(
    app: AppHandle,
    model_name: String,
    _filename: String,
    url: String,
    _models_dir: PathBuf,
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
        // Not resuming: discard any stale partial so we start clean.
        let _ = std::fs::remove_file(&temp_path);
    }

    // Always download into the .part file; it is only renamed to the final
    // .bin once the full byte count has been written. This guarantees a .bin
    // on disk is always complete.
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

    let mut stream = res.bytes_stream();
    let mut last_progress_emit = std::time::Instant::now();
    let start_time = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        if cancel_flag.load(Ordering::Relaxed) {
            logging::log_message(&format!("[downloader] Download cancelled for {}", model_name));
            let _ = app.emit("model:progress", serde_json::json!({
                "model": model_name,
                "percent": (downloaded as f64 / total_size as f64 * 100.0).round(),
                "bytes_downloaded": downloaded,
                "total_bytes": total_size,
                "speed": 0,
                "eta_secs": 0,
                "phase": "cancelled"
            }));
            return Err("Download cancelled".to_string());
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

    std::fs::rename(&temp_path, &file_path).map_err(|e| e.to_string())?;

    remove_expected_size(&model_name);

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

pub fn get_downloading_models() -> Vec<String> {
    if let Ok(downloads) = ACTIVE_DOWNLOADS.lock() {
        downloads.keys().cloned().collect()
    } else {
        Vec::new()
    }
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

/// Returns the size of a partially downloaded model (.part file), or 0 if none.
pub fn get_partial_download_size(model_name: &str) -> u64 {
    let filename = format!("ggml-{}.bin.part", model_name);
    let path = crate::paths::config_dir_opt().map(|d| d.join("models").join(&filename));
    match path {
        Some(path) if path.exists() => {
            std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
        }
        _ => 0
    }
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
