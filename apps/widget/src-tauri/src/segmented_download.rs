//! Sliding-window HTTP Range downloader with adaptive in-flight concurrency.
//!
//! Never pre-splits the whole file into connections. Only `N` Range GETs are
//! in flight at a time; `N` starts at 1 and ramps with hysteresis. A global
//! budget caps total connections across concurrent model downloads.

use crate::downloader::{self, take_pause_request};
use crate::logging;
use futures_util::stream::{FuturesUnordered, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, IF_RANGE, RANGE};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex as AsyncMutex;

const CHUNK_SIZE: u64 = 8 * 1024 * 1024;
const SMALL_FILE_THRESHOLD: u64 = 16 * 1024 * 1024;
const PROBE_BYTES: u64 = 2 * 1024 * 1024;
const PER_MODEL_MAX: usize = 8;
const GLOBAL_MAX_IN_FLIGHT: usize = 8;
const WINDOW_SECS: f64 = 4.0;
const IMPROVE_THRESHOLD: f64 = 0.15;
const MAX_CHUNK_RETRIES: u32 = 3;

static GLOBAL_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub enum SegmentedError {
    /// Server does not support Range the way we need — caller should use single-stream.
    FallBackToSingle(String),
    Failed(String),
}

impl From<SegmentedError> for String {
    fn from(e: SegmentedError) -> Self {
        match e {
            SegmentedError::FallBackToSingle(s) | SegmentedError::Failed(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PartsState {
    #[serde(default)]
    etag: Option<String>,
    total: u64,
    /// Half-open completed ranges `[start, end)`.
    completed: Vec<(u64, u64)>,
}

fn parts_path(models_dir: &Path, model_name: &str) -> PathBuf {
    models_dir.join(format!("ggml-{}.parts.json", model_name))
}

pub fn remove_parts_file(model_name: &str) {
    if let Some(dir) = crate::paths::config_dir_opt().map(|d| d.join("models")) {
        let path = parts_path(&dir, model_name);
        let _ = std::fs::remove_file(&path);
    }
}

pub fn completed_bytes_from_parts(model_name: &str) -> Option<u64> {
    let dir = crate::paths::config_dir_opt()?.join("models");
    let path = parts_path(&dir, model_name);
    if !path.exists() {
        return None;
    }
    let state: PartsState = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
    Some(sum_completed(&state.completed))
}

fn load_parts(path: &Path) -> PartsState {
    if path.exists() {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(state) = serde_json::from_str::<PartsState>(&text) {
                return state;
            }
        }
    }
    PartsState::default()
}

fn save_parts_sync(path: &Path, state: &PartsState) -> Result<(), String> {
    let tmp = path.with_extension("parts.json.tmp");
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

fn merge_ranges(ranges: &mut Vec<(u64, u64)>) {
    if ranges.is_empty() {
        return;
    }
    ranges.sort_by_key(|r| r.0);
    let mut merged = Vec::new();
    let mut cur = ranges[0];
    for &(s, e) in ranges.iter().skip(1) {
        if s <= cur.1 {
            cur.1 = cur.1.max(e);
        } else {
            merged.push(cur);
            cur = (s, e);
        }
    }
    merged.push(cur);
    *ranges = merged;
}

fn sum_completed(ranges: &[(u64, u64)]) -> u64 {
    ranges.iter().map(|(s, e)| e.saturating_sub(*s)).sum()
}

fn next_missing_offset(ranges: &[(u64, u64)], total: u64) -> u64 {
    let mut cursor = 0u64;
    for &(s, e) in ranges {
        if s > cursor {
            return cursor;
        }
        cursor = cursor.max(e);
    }
    cursor.min(total)
}

fn probe_tier_ceiling(speed_mib_s: f64) -> usize {
    if speed_mib_s < 2.0 {
        1
    } else if speed_mib_s < 8.0 {
        2
    } else if speed_mib_s < 20.0 {
        4
    } else {
        8
    }
}

fn try_acquire_global_slot() -> bool {
    loop {
        let cur = GLOBAL_IN_FLIGHT.load(Ordering::SeqCst);
        if cur >= GLOBAL_MAX_IN_FLIGHT {
            return false;
        }
        if GLOBAL_IN_FLIGHT
            .compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return true;
        }
    }
}

fn release_global_slot() {
    GLOBAL_IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
}

fn parse_content_range_total(header: &str) -> Option<u64> {
    // bytes start-end/total
    let total = header.split('/').nth(1)?.trim();
    if total == "*" {
        return None;
    }
    total.parse().ok()
}

fn check_user_stop(model_name: &str, cancel_flag: &AtomicBool) -> Result<(), SegmentedError> {
    if !cancel_flag.load(Ordering::Relaxed) {
        return Ok(());
    }
    let is_pause = take_pause_request(model_name);
    Err(SegmentedError::Failed(if is_pause {
        "Download paused".to_string()
    } else {
        "Download cancelled".to_string()
    }))
}

struct ChunkResult {
    start: u64,
    end: u64,
    data: Vec<u8>,
    etag: Option<String>,
}

struct ChunkFail {
    start: u64,
    end: u64,
    error: String,
    retries_left: u32,
    rate_limited: bool,
    fall_back: bool,
    etag_mismatch: bool,
}

async fn fetch_chunk(
    client: Client,
    url: String,
    start: u64,
    end: u64,
    etag: Option<String>,
    cancel_flag: Arc<AtomicBool>,
    retries_left: u32,
) -> Result<ChunkResult, ChunkFail> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err(ChunkFail {
            start,
            end,
            error: "cancelled".into(),
            retries_left: 0,
            rate_limited: false,
            fall_back: false,
            etag_mismatch: false,
        });
    }

    let mut headers = HeaderMap::new();
    let range_val = format!("bytes={}-{}", start, end - 1);
    if let Ok(v) = HeaderValue::from_str(&range_val) {
        headers.insert(RANGE, v);
    }
    if let Some(ref tag) = etag {
        if let Ok(v) = HeaderValue::from_str(tag) {
            headers.insert(IF_RANGE, v);
        }
    }

    let res = match client.get(&url).headers(headers).send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(ChunkFail {
                start,
                end,
                error: e.to_string(),
                retries_left,
                rate_limited: false,
                fall_back: false,
                etag_mismatch: false,
            });
        }
    };

    let status = res.status();
    if status.as_u16() == 429 || status.as_u16() == 503 {
        return Err(ChunkFail {
            start,
            end,
            error: format!("HTTP {}", status),
            retries_left,
            rate_limited: true,
            fall_back: false,
            etag_mismatch: false,
        });
    }

    if status.as_u16() == 404 {
        return Err(ChunkFail {
            start,
            end,
            error: format!("Failed to download model (status: {})", status),
            retries_left: 0,
            rate_limited: false,
            fall_back: false,
            etag_mismatch: false,
        });
    }

    // If-Range failed → server sent full 200
    if status.as_u16() == 200 && etag.is_some() {
        return Err(ChunkFail {
            start,
            end,
            error: "Remote file changed (ETag mismatch)".into(),
            retries_left: 0,
            rate_limited: false,
            fall_back: false,
            etag_mismatch: true,
        });
    }

    if status.as_u16() == 200 {
        return Err(ChunkFail {
            start,
            end,
            error: "Server ignored Range request".into(),
            retries_left: 0,
            rate_limited: false,
            fall_back: true,
            etag_mismatch: false,
        });
    }

    if status.as_u16() != 206 {
        return Err(ChunkFail {
            start,
            end,
            error: format!("Failed to download model (status: {})", status),
            retries_left,
            rate_limited: status.is_server_error(),
            fall_back: false,
            etag_mismatch: false,
        });
    }

    let resp_etag = res
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let expected_len = end - start;
    let mut collected = Vec::with_capacity(expected_len as usize);
    let mut stream = res.bytes_stream();
    while let Some(item) = stream.next().await {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(ChunkFail {
                start,
                end,
                error: "cancelled".into(),
                retries_left: 0,
                rate_limited: false,
                fall_back: false,
                etag_mismatch: false,
            });
        }
        match item {
            Ok(chunk) => collected.extend_from_slice(&chunk),
            Err(e) => {
                return Err(ChunkFail {
                    start,
                    end,
                    error: e.to_string(),
                    retries_left,
                    rate_limited: false,
                    fall_back: false,
                    etag_mismatch: false,
                });
            }
        }
    }

    if collected.len() as u64 != expected_len {
        return Err(ChunkFail {
            start,
            end,
            error: format!(
                "Short chunk: got {} expected {}",
                collected.len(),
                expected_len
            ),
            retries_left,
            rate_limited: false,
            fall_back: false,
            etag_mismatch: false,
        });
    }

    Ok(ChunkResult {
        start,
        end,
        data: collected,
        etag: resp_etag,
    })
}

fn write_chunk_sync(path: &Path, start: u64, data: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;
    file.write_all(data).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(())
}

fn emit_progress(
    app: &AppHandle,
    model_name: &str,
    downloaded: u64,
    total: u64,
    speed_mib_s: f64,
    connections: usize,
    phase: &str,
    message: Option<&str>,
) {
    let percent = if total > 0 {
        (downloaded as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };
    let remaining = total.saturating_sub(downloaded);
    let eta_secs = if speed_mib_s > 0.0 {
        (remaining as f64 / 1024.0 / 1024.0 / speed_mib_s).round() as u64
    } else {
        0
    };
    let mut payload = serde_json::json!({
        "model": model_name,
        "percent": percent,
        "bytes_downloaded": downloaded,
        "total_bytes": total,
        "speed_mbps": (speed_mib_s * 100.0).round() / 100.0,
        "eta_secs": eta_secs,
        "phase": phase,
        "connections": connections,
    });
    if let Some(msg) = message {
        payload["message"] = serde_json::json!(msg);
    }
    let _ = app.emit("model:progress", payload);
}

fn emit_stop(app: &AppHandle, model_name: &str, downloaded: u64, total: u64, paused: bool) {
    let phase = if paused { "paused" } else { "cancelled" };
    let message = if paused {
        "Download paused — tap Resume to continue"
    } else {
        "Download cancelled"
    };
    emit_progress(
        app,
        model_name,
        downloaded,
        total,
        0.0,
        0,
        phase,
        Some(message),
    );
}

/// Attempt a sliding-window segmented download. Returns FallBackToSingle when
/// the server does not support Range well enough.
pub async fn download_segmented(
    app: AppHandle,
    client: &Client,
    model_name: String,
    url: String,
    file_path: PathBuf,
    temp_path: PathBuf,
    cancel_flag: Arc<AtomicBool>,
) -> Result<PathBuf, SegmentedError> {
    let models_dir = temp_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| SegmentedError::Failed("Invalid temp path".into()))?;
    let parts_file = parts_path(&models_dir, &model_name);

    let mut state = load_parts(&parts_file);

    // Legacy single-stream .part → contiguous prefix.
    if temp_path.exists() && state.completed.is_empty() {
        let len = std::fs::metadata(&temp_path)
            .map(|m| m.len())
            .unwrap_or(0);
        if len > 0 {
            state.completed.push((0, len));
            merge_ranges(&mut state.completed);
            logging::log_message(&format!(
                "[downloader] Migrating legacy .part for {} ({} bytes prefix)",
                model_name, len
            ));
        }
    }

    // Discover size + Range support with a probe GET.
    let first_missing = if state.total > 0 {
        next_missing_offset(&state.completed, state.total)
    } else {
        next_missing_offset(&state.completed, u64::MAX)
    };

    check_user_stop(&model_name, &cancel_flag).map_err(|e| {
        if let SegmentedError::Failed(ref msg) = e {
            if msg == "Download paused" || msg == "Download cancelled" {
                let dl = sum_completed(&state.completed);
                emit_stop(&app, &model_name, dl, state.total, msg == "Download paused");
            }
        }
        e
    })?;

    let probe_end = first_missing + PROBE_BYTES.min(CHUNK_SIZE);
    let mut headers = HeaderMap::new();
    let range_val = format!("bytes={}-{}", first_missing, probe_end.saturating_sub(1));
    headers.insert(
        RANGE,
        HeaderValue::from_str(&range_val).map_err(|e| SegmentedError::Failed(e.to_string()))?,
    );
    if let Some(ref tag) = state.etag {
        if let Ok(v) = HeaderValue::from_str(tag) {
            headers.insert(IF_RANGE, v);
        }
    }

    logging::log_message(&format!(
        "[downloader] Range probe {} {}",
        model_name, range_val
    ));

    let probe_res = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| SegmentedError::Failed(e.to_string()))?;

    let status = probe_res.status();
    if status.as_u16() == 404 {
        return Err(SegmentedError::Failed(format!(
            "Failed to download model (status: {})",
            status
        )));
    }
    if status.as_u16() == 200 {
        return Err(SegmentedError::FallBackToSingle(
            "Server does not support Range requests".into(),
        ));
    }
    if status.as_u16() != 206 {
        return Err(SegmentedError::Failed(format!(
            "Failed to fetch model info (status: {})",
            status
        )));
    }

    let total_size = probe_res
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_range_total)
        .or_else(|| {
            probe_res
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(|len| first_missing + len)
        })
        .filter(|t| *t > 0)
        .ok_or_else(|| {
            SegmentedError::FallBackToSingle("Could not determine total size from Range".into())
        })?;

    if let Some(tag) = probe_res
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
    {
        state.etag = Some(tag.to_string());
    }
    state.total = total_size;
    downloader::save_expected_size(&model_name, total_size);

    let remaining = total_size.saturating_sub(sum_completed(&state.completed));
    if remaining == 0 {
        return finalize(&app, &model_name, &temp_path, &file_path, &parts_file, &state).await;
    }

    if remaining < SMALL_FILE_THRESHOLD && state.completed.is_empty() {
        return Err(SegmentedError::FallBackToSingle(
            "Small remaining download — single connection".into(),
        ));
    }

    // Preallocate sparse file (fails clearly on low disk).
    {
        let path = temp_path.clone();
        let total = total_size;
        tokio::task::spawn_blocking(move || {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&path)
                .map_err(|e| e.to_string())?;
            file.set_len(total).map_err(|e| {
                if e.raw_os_error() == Some(28) || e.to_string().to_lowercase().contains("space") {
                    format!("Not enough disk space to download model ({} bytes): {}", total, e)
                } else {
                    e.to_string()
                }
            })?;
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| SegmentedError::Failed(e.to_string()))?
        .map_err(|e| {
            if e.to_lowercase().contains("disk space") || e.to_lowercase().contains("not enough") {
                SegmentedError::Failed(e)
            } else {
                SegmentedError::FallBackToSingle(format!("Preallocate failed: {}", e))
            }
        })?;
    }

    // Consume probe body as first chunk (may be shorter than PROBE_BYTES near EOF).
    let actual_probe_end = probe_end.min(total_size);
    let expected_probe_len = actual_probe_end.saturating_sub(first_missing);
    let probe_start_time = Instant::now();
    let mut probe_buf = Vec::with_capacity(expected_probe_len as usize);
    let mut stream = probe_res.bytes_stream();
    while let Some(item) = stream.next().await {
        check_user_stop(&model_name, &cancel_flag).map_err(|e| {
            if let SegmentedError::Failed(ref msg) = e {
                if msg == "Download paused" || msg == "Download cancelled" {
                    emit_stop(
                        &app,
                        &model_name,
                        sum_completed(&state.completed),
                        total_size,
                        msg == "Download paused",
                    );
                }
            }
            e
        })?;
        let chunk = item.map_err(|e| SegmentedError::Failed(e.to_string()))?;
        probe_buf.extend_from_slice(&chunk);
        if probe_buf.len() as u64 >= expected_probe_len {
            break;
        }
    }
    if (probe_buf.len() as u64) < expected_probe_len {
        return Err(SegmentedError::Failed(format!(
            "Short probe response: {} < {}",
            probe_buf.len(),
            expected_probe_len
        )));
    }
    // Trim if server sent more
    probe_buf.truncate(expected_probe_len as usize);

    let probe_elapsed = probe_start_time.elapsed().as_secs_f64().max(0.001);
    // Ignore first ~500ms warm-up for tier estimate when possible
    let probe_speed = (probe_buf.len() as f64 / 1024.0 / 1024.0) / probe_elapsed;
    let mut probe_ceiling = probe_tier_ceiling(probe_speed).min(PER_MODEL_MAX);

    {
        let path = temp_path.clone();
        let start = first_missing;
        let data = probe_buf.clone();
        tokio::task::spawn_blocking(move || write_chunk_sync(&path, start, &data))
            .await
            .map_err(|e| SegmentedError::Failed(e.to_string()))?
            .map_err(SegmentedError::Failed)?;
    }
    state.completed.push((first_missing, actual_probe_end));
    merge_ranges(&mut state.completed);
    save_parts_sync(&parts_file, &state).map_err(SegmentedError::Failed)?;

    logging::log_message(&format!(
        "[downloader] Probe done for {} ({:.2} MiB/s) → ceiling {}",
        model_name, probe_speed, probe_ceiling
    ));

    let mut per_model_n: usize = 1;
    let mut consecutive_improves: u32 = 0;
    let mut cooldown = false;
    let mut additive_mode = false;
    let mut last_window_speed = probe_speed;
    let mut window_bytes: u64 = probe_buf.len() as u64;
    let mut window_start = Instant::now();
    let mut last_progress_emit = Instant::now();
    let mut rolling_bytes: u64 = 0;
    let mut rolling_start = Instant::now();
    let file_lock = Arc::new(AsyncMutex::new(()));

    // If already complete after probe
    if sum_completed(&state.completed) >= total_size {
        return finalize(&app, &model_name, &temp_path, &file_path, &parts_file, &state).await;
    }

    let mut in_flight: FuturesUnordered<
        tokio::task::JoinHandle<Result<ChunkResult, ChunkFail>>,
    > = FuturesUnordered::new();
    let mut issued: Vec<(u64, u64)> = Vec::new();
    // Skip ranges already covered / currently issued when advancing
    let mut issue_cursor = next_missing_offset(&state.completed, total_size);

    // Helper to find next unissued missing range start
    let advance_cursor = |completed: &[(u64, u64)],
                          issued: &[(u64, u64)],
                          mut cursor: u64,
                          total: u64|
     -> u64 {
        while cursor < total {
            let mut covered = false;
            for &(s, e) in completed.iter().chain(issued.iter()) {
                if cursor >= s && cursor < e {
                    cursor = e;
                    covered = true;
                    break;
                }
            }
            if !covered {
                break;
            }
        }
        cursor
    };

    issue_cursor = advance_cursor(&state.completed, &issued, issue_cursor, total_size);

    emit_progress(
        &app,
        &model_name,
        sum_completed(&state.completed),
        total_size,
        probe_speed,
        1,
        "downloading",
        None,
    );

    loop {
        if let Err(e) = check_user_stop(&model_name, &cancel_flag) {
            while let Some(joined) = in_flight.next().await {
                release_global_slot();
                let _ = joined;
            }
            issued.clear();
            if let SegmentedError::Failed(ref msg) = e {
                if msg == "Download paused" || msg == "Download cancelled" {
                    emit_stop(
                        &app,
                        &model_name,
                        sum_completed(&state.completed),
                        total_size,
                        msg == "Download paused",
                    );
                }
            }
            return Err(e);
        }

        if sum_completed(&state.completed) >= total_size {
            break;
        }

        // Ramp window
        if window_start.elapsed().as_secs_f64() >= WINDOW_SECS {
            let elapsed = window_start.elapsed().as_secs_f64().max(0.001);
            let speed = (window_bytes as f64 / 1024.0 / 1024.0) / elapsed;
            if !cooldown {
                if speed >= last_window_speed * (1.0 + IMPROVE_THRESHOLD) {
                    consecutive_improves += 1;
                    if consecutive_improves >= 2 {
                        let old = per_model_n;
                        if additive_mode {
                            per_model_n = (per_model_n + 1).min(probe_ceiling).min(PER_MODEL_MAX);
                        } else {
                            per_model_n = (per_model_n * 2).min(probe_ceiling).min(PER_MODEL_MAX);
                        }
                        if per_model_n != old {
                            logging::log_message(&format!(
                                "[downloader] N {}→{} (global {}/{}, speed {:.1}→{:.1} MB/s)",
                                old,
                                per_model_n,
                                GLOBAL_IN_FLIGHT.load(Ordering::SeqCst),
                                GLOBAL_MAX_IN_FLIGHT,
                                last_window_speed,
                                speed
                            ));
                            cooldown = true;
                        }
                        consecutive_improves = 0;
                    }
                } else {
                    consecutive_improves = 0;
                }
            } else {
                cooldown = false;
            }
            last_window_speed = speed;
            window_bytes = 0;
            window_start = Instant::now();
        }

        let effective_n = per_model_n.min(probe_ceiling).min(PER_MODEL_MAX);

        // Fill sliding window
        while in_flight.len() < effective_n && issue_cursor < total_size {
            if !try_acquire_global_slot() {
                break;
            }
            let start = issue_cursor;
            let end = (start + CHUNK_SIZE).min(total_size);
            // Ensure we don't overlap completed/issued
            let end = {
                let mut e = end;
                for &(s, _) in state.completed.iter().chain(issued.iter()) {
                    if s > start && s < e {
                        e = s;
                    }
                }
                e
            };
            if end <= start {
                release_global_slot();
                issue_cursor = advance_cursor(&state.completed, &issued, start + 1, total_size);
                continue;
            }

            issued.push((start, end));
            issue_cursor = advance_cursor(&state.completed, &issued, end, total_size);

            let client = client.clone();
            let url = url.clone();
            let etag = state.etag.clone();
            let cancel = cancel_flag.clone();
            in_flight.push(tokio::spawn(async move {
                fetch_chunk(client, url, start, end, etag, cancel, MAX_CHUNK_RETRIES).await
            }));
        }

        if in_flight.is_empty() {
            if issue_cursor >= total_size && sum_completed(&state.completed) < total_size {
                // Holes remain but nothing issued — recompute cursor from holes
                issue_cursor = next_missing_offset(&state.completed, total_size);
                if issue_cursor >= total_size {
                    break;
                }
                // Could not acquire global slots — wait briefly
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                continue;
            }
            if sum_completed(&state.completed) >= total_size {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        }

        let finished = in_flight.next().await;
        let Some(joined) = finished else {
            break;
        };
        let outcome = joined.map_err(|e| SegmentedError::Failed(e.to_string()))?;

        match outcome {
            Ok(chunk) => {
                release_global_slot();
                issued.retain(|&(s, e)| !(s == chunk.start && e == chunk.end));

                let path = temp_path.clone();
                let start = chunk.start;
                let data = chunk.data.clone();
                let _guard = file_lock.lock().await;
                tokio::task::spawn_blocking(move || write_chunk_sync(&path, start, &data))
                    .await
                    .map_err(|e| SegmentedError::Failed(e.to_string()))?
                    .map_err(SegmentedError::Failed)?;
                drop(_guard);

                state.completed.push((chunk.start, chunk.end));
                merge_ranges(&mut state.completed);
                if chunk.etag.is_some() && state.etag.is_none() {
                    state.etag = chunk.etag;
                }
                save_parts_sync(&parts_file, &state).map_err(SegmentedError::Failed)?;

                let n = (chunk.end - chunk.start) as u64;
                window_bytes += n;
                rolling_bytes += n;

                let downloaded = sum_completed(&state.completed);
                if last_progress_emit.elapsed().as_millis() > 150 {
                    let roll_elapsed = rolling_start.elapsed().as_secs_f64().max(0.001);
                    let speed = (rolling_bytes as f64 / 1024.0 / 1024.0) / roll_elapsed;
                    if rolling_start.elapsed().as_secs_f64() > 2.0 {
                        rolling_bytes = 0;
                        rolling_start = Instant::now();
                    }
                    emit_progress(
                        &app,
                        &model_name,
                        downloaded,
                        total_size,
                        speed,
                        in_flight.len() + 1,
                        "downloading",
                        None,
                    );
                    last_progress_emit = Instant::now();
                }
            }
            Err(fail) => {
                release_global_slot();
                issued.retain(|&(s, e)| !(s == fail.start && e == fail.end));

                if fail.error == "cancelled" || cancel_flag.load(Ordering::Relaxed) {
                    let is_pause = take_pause_request(&model_name);
                    while let Some(joined) = in_flight.next().await {
                        release_global_slot();
                        let _ = joined;
                    }
                    issued.clear();
                    emit_stop(
                        &app,
                        &model_name,
                        sum_completed(&state.completed),
                        total_size,
                        is_pause,
                    );
                    return Err(SegmentedError::Failed(if is_pause {
                        "Download paused".into()
                    } else {
                        "Download cancelled".into()
                    }));
                }

                if fail.etag_mismatch {
                    while let Some(joined) = in_flight.next().await {
                        release_global_slot();
                        let _ = joined;
                    }
                    issued.clear();
                    let _ = std::fs::remove_file(&temp_path);
                    let _ = std::fs::remove_file(&parts_file);
                    return Err(SegmentedError::Failed(
                        "Remote file changed (ETag mismatch) — restart download".into(),
                    ));
                }

                if fail.fall_back {
                    while let Some(joined) = in_flight.next().await {
                        release_global_slot();
                        let _ = joined;
                    }
                    issued.clear();
                    return Err(SegmentedError::FallBackToSingle(fail.error));
                }

                if fail.rate_limited {
                    let old = per_model_n;
                    per_model_n = (per_model_n / 2).max(1);
                    additive_mode = true;
                    probe_ceiling = probe_ceiling.min(per_model_n.max(2));
                    cooldown = true;
                    consecutive_improves = 0;
                    logging::log_message(&format!(
                        "[downloader] Rate limited — N {}→{}, backing off",
                        old, per_model_n
                    ));
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }

                if fail.retries_left > 0 {
                    let start = fail.start;
                    let end = fail.end;
                    if try_acquire_global_slot() {
                        issued.push((start, end));
                        let client = client.clone();
                        let url = url.clone();
                        let etag = state.etag.clone();
                        let cancel = cancel_flag.clone();
                        let retries = fail.retries_left - 1;
                        logging::log_message(&format!(
                            "[downloader] Retrying chunk {}-{} ({} left): {}",
                            start, end, retries, fail.error
                        ));
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        in_flight.push(tokio::spawn(async move {
                            fetch_chunk(client, url, start, end, etag, cancel, retries).await
                        }));
                    } else {
                        // Could not get a global slot — surface as transient for queue retry.
                        return Err(SegmentedError::Failed(fail.error));
                    }
                } else {
                    return Err(SegmentedError::Failed(fail.error));
                }
            }
        }
    }

    // Drain any stragglers and release slots
    while let Some(joined) = in_flight.next().await {
        release_global_slot();
        let _ = joined;
    }
    issued.clear();

    if sum_completed(&state.completed) < total_size {
        return Err(SegmentedError::Failed(format!(
            "Incomplete download: {} / {} bytes",
            sum_completed(&state.completed),
            total_size
        )));
    }

    finalize(&app, &model_name, &temp_path, &file_path, &parts_file, &state).await
}

async fn finalize(
    app: &AppHandle,
    model_name: &str,
    temp_path: &Path,
    file_path: &Path,
    parts_file: &Path,
    state: &PartsState,
) -> Result<PathBuf, SegmentedError> {
    let downloaded = sum_completed(&state.completed);
    if downloaded < state.total || state.total == 0 {
        return Err(SegmentedError::Failed(format!(
            "Finalize size mismatch: {} / {}",
            downloaded, state.total
        )));
    }

    let temp = temp_path.to_path_buf();
    let dest = file_path.to_path_buf();
    let parts = parts_file.to_path_buf();
    let total = state.total;

    tokio::task::spawn_blocking(move || {
        let meta = std::fs::metadata(&temp).map_err(|e| e.to_string())?;
        if meta.len() < total {
            // Ensure length (sparse holes may report full len after set_len)
            let f = OpenOptions::new()
                .write(true)
                .open(&temp)
                .map_err(|e| e.to_string())?;
            f.set_len(total).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        } else {
            let f = File::open(&temp).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        std::fs::rename(&temp, &dest).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&parts);
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| SegmentedError::Failed(e.to_string()))?
    .map_err(SegmentedError::Failed)?;

    downloader::remove_expected_size(model_name);
    emit_progress(app, model_name, downloaded, state.total, 0.0, 0, "done", None);
    logging::log_message(&format!(
        "[downloader] Model {} downloaded to {} ({:.1} MB) [segmented]",
        model_name,
        file_path.display(),
        downloaded as f64 / 1024.0 / 1024.0
    ));
    Ok(file_path.to_path_buf())
}
