use std::fmt::{Display, Formatter};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::Emitter;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub const MAX_CONCURRENT_DOWNLOADS: usize = 10;
pub const DEFAULT_REQUESTS_PER_MINUTE: usize = 120;
pub const DOWNLOAD_PROGRESS_EVENT: &str = "download-progress";
const MAX_TRANSIENT_RETRIES: usize = 2;
const BASE_RETRY_DELAY_MS: u64 = 400;

#[derive(Debug, Clone, Copy)]
pub struct DownloadRateLimits {
    pub requests_per_minute: usize,
    pub concurrent_downloads: usize,
}

impl Default for DownloadRateLimits {
    fn default() -> Self {
        Self {
            requests_per_minute: DEFAULT_REQUESTS_PER_MINUTE,
            concurrent_downloads: MAX_CONCURRENT_DOWNLOADS,
        }
    }
}

#[derive(Debug)]
struct RequestRateLimiter {
    min_interval: Duration,
    next_allowed_at: tokio::sync::Mutex<tokio::time::Instant>,
}

impl RequestRateLimiter {
    fn from_requests_per_minute(requests_per_minute: usize) -> Self {
        let safe_requests_per_minute = requests_per_minute.max(1);
        let seconds_per_request = 60.0 / safe_requests_per_minute as f64;
        let min_interval = Duration::from_secs_f64(seconds_per_request);

        Self {
            min_interval,
            next_allowed_at: tokio::sync::Mutex::new(tokio::time::Instant::now()),
        }
    }

    async fn wait_turn(&self) {
        let mut next_allowed_at = self.next_allowed_at.lock().await;
        let now = tokio::time::Instant::now();

        if *next_allowed_at > now {
            let delay = *next_allowed_at - now;
            println!("rate-limit wait {:?} before next request", delay);
            tokio::time::sleep_until(*next_allowed_at).await;
        }

        let baseline = std::cmp::max(*next_allowed_at, tokio::time::Instant::now());
        *next_allowed_at = baseline + self.min_interval;
    }
}

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub memory_item_id: i64,
    pub url: String,
    pub destination_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub memory_item_id: i64,
    pub source_url: String,
    pub destination_path: PathBuf,
    pub bytes_written: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressPayload {
    pub total_files: usize,
    pub completed_files: usize,
    pub successful_files: usize,
    pub failed_files: usize,
    pub memory_item_id: Option<i64>,
    pub status: String,
    pub error_code: Option<DownloadErrorCode>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadErrorCode {
    ExpiredLink,
    HttpError,
    IoError,
    ConcurrencyError,
    Stopped,
    InternalError,
}

#[derive(Debug)]
pub enum DownloadError {
    Semaphore(tokio::sync::AcquireError),
    SemaphoreClosed,
    Stopped {
        memory_item_id: i64,
    },
    Join(tokio::task::JoinError),
    Http {
        memory_item_id: i64,
        url: String,
        source: reqwest::Error,
    },
    Io {
        memory_item_id: i64,
        path: PathBuf,
        source: std::io::Error,
    },
    Emit(tauri::Error),
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Semaphore(error) => {
                write!(f, "download manager semaphore failed: {error}")
            }
            Self::SemaphoreClosed => write!(f, "download manager semaphore is closed"),
            Self::Stopped { memory_item_id } => {
                write!(
                    f,
                    "download stopped by user for memory item {memory_item_id}"
                )
            }
            Self::Join(error) => write!(f, "download manager task failed: {error}"),
            Self::Http {
                memory_item_id,
                source,
                ..
            } => write!(
                f,
                "download failed for memory item {memory_item_id}: {source}"
            ),
            Self::Io {
                memory_item_id,
                path,
                source,
            } => write!(
                f,
                "failed to write download for memory item {memory_item_id} to '{}': {source}",
                path.display()
            ),
            Self::Emit(error) => write!(f, "failed to emit download progress event: {error}"),
        }
    }
}

impl DownloadError {
    pub fn memory_item_id(&self) -> Option<i64> {
        match self {
            Self::Stopped { memory_item_id } => Some(*memory_item_id),
            Self::Http { memory_item_id, .. } => Some(*memory_item_id),
            Self::Io { memory_item_id, .. } => Some(*memory_item_id),
            Self::Semaphore(_) | Self::SemaphoreClosed | Self::Join(_) | Self::Emit(_) => None,
        }
    }

    pub fn error_code(&self) -> DownloadErrorCode {
        match self {
            Self::Http { source, .. } => {
                if source
                    .status()
                    .map(|status| status == reqwest::StatusCode::FORBIDDEN)
                    .unwrap_or(false)
                {
                    DownloadErrorCode::ExpiredLink
                } else {
                    DownloadErrorCode::HttpError
                }
            }
            Self::Io { .. } => DownloadErrorCode::IoError,
            Self::Stopped { .. } => DownloadErrorCode::Stopped,
            Self::Semaphore(_) => DownloadErrorCode::ConcurrencyError,
            Self::SemaphoreClosed => DownloadErrorCode::ConcurrencyError,
            Self::Join(_) | Self::Emit(_) => DownloadErrorCode::InternalError,
        }
    }

    pub fn url(&self) -> Option<&str> {
        match self {
            Self::Http { url, .. } => Some(url.as_str()),
            Self::Semaphore(_)
            | Self::SemaphoreClosed
            | Self::Stopped { .. }
            | Self::Join(_)
            | Self::Io { .. }
            | Self::Emit(_) => None,
        }
    }

    pub fn http_status(&self) -> Option<reqwest::StatusCode> {
        match self {
            Self::Http { source, .. } => source.status(),
            Self::Semaphore(_)
            | Self::SemaphoreClosed
            | Self::Stopped { .. }
            | Self::Join(_)
            | Self::Io { .. }
            | Self::Emit(_) => None,
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http { source, .. } => {
                if let Some(status) = source.status() {
                    return status == reqwest::StatusCode::REQUEST_TIMEOUT
                        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
                        || status.is_server_error();
                }

                source.is_connect() || source.is_timeout() || source.is_request()
            }
            Self::Semaphore(_) | Self::SemaphoreClosed | Self::Join(_) => true,
            Self::Stopped { .. } | Self::Io { .. } | Self::Emit(_) => false,
        }
    }

    pub fn is_stopped(&self) -> bool {
        matches!(self, Self::Stopped { .. })
    }
}

impl std::error::Error for DownloadError {}

pub async fn download_tasks(
    tasks: Vec<DownloadTask>,
) -> Result<Vec<Result<DownloadResult, DownloadError>>, DownloadError> {
    download_tasks_internal(tasks, None, DownloadRateLimits::default()).await
}

pub async fn download_tasks_with_progress(
    window: &tauri::Window,
    tasks: Vec<DownloadTask>,
) -> Result<Vec<Result<DownloadResult, DownloadError>>, DownloadError> {
    download_tasks_internal(tasks, Some(window), DownloadRateLimits::default()).await
}

pub async fn download_tasks_with_progress_and_rate_limits(
    window: &tauri::Window,
    tasks: Vec<DownloadTask>,
    rate_limits: DownloadRateLimits,
) -> Result<Vec<Result<DownloadResult, DownloadError>>, DownloadError> {
    download_tasks_internal(tasks, Some(window), rate_limits).await
}

async fn download_tasks_internal(
    tasks: Vec<DownloadTask>,
    window: Option<&tauri::Window>,
    rate_limits: DownloadRateLimits,
) -> Result<Vec<Result<DownloadResult, DownloadError>>, DownloadError> {
    let client = reqwest::Client::new();
    let semaphore = Arc::new(Semaphore::new(rate_limits.concurrent_downloads.max(1)));
    let request_rate_limiter = Arc::new(RequestRateLimiter::from_requests_per_minute(
        rate_limits.requests_per_minute,
    ));
    let total_files = tasks.len();

    let mut handles = Vec::with_capacity(tasks.len());

    for task in tasks {
        let semaphore = semaphore.clone();
        let client = client.clone();
        let request_rate_limiter = request_rate_limiter.clone();

        handles.push(tokio::spawn(async move {
            let _permit = acquire_permit_with_control(semaphore, task.memory_item_id).await?;
            wait_for_pause_or_stop(task.memory_item_id).await?;
            request_rate_limiter.wait_turn().await;
            wait_for_pause_or_stop(task.memory_item_id).await?;
            download_single_task(&client, task).await
        }));
    }

    let mut successful_files = 0usize;
    let mut failed_files = 0usize;
    let mut results = Vec::with_capacity(handles.len());

    for (index, handle) in handles.into_iter().enumerate() {
        let result = match handle.await {
            Ok(result) => result,
            Err(error) => Err(DownloadError::Join(error)),
        };

        let completed_files = index + 1;

        let (memory_item_id, status, error_code, error_message) = match &result {
            Ok(download_result) => {
                successful_files += 1;
                (
                    Some(download_result.memory_item_id),
                    "success".to_string(),
                    None,
                    None,
                )
            }
            Err(error) => {
                if !error.is_stopped() {
                    failed_files += 1;
                }
                (
                    error.memory_item_id(),
                    if error.is_stopped() {
                        "stopped".to_string()
                    } else {
                        "error".to_string()
                    },
                    Some(error.error_code()),
                    Some(error.to_string()),
                )
            }
        };

        if let Some(window) = window {
            let payload = DownloadProgressPayload {
                total_files,
                completed_files,
                successful_files,
                failed_files,
                memory_item_id,
                status,
                error_code,
                error_message,
            };

            window
                .emit(DOWNLOAD_PROGRESS_EVENT, payload)
                .map_err(DownloadError::Emit)?;
        }

        results.push(result);
    }

    Ok(results)
}

async fn download_single_task(
    client: &reqwest::Client,
    task: DownloadTask,
) -> Result<DownloadResult, DownloadError> {
    let mut attempt = 0usize;

    loop {
        wait_for_pause_or_stop(task.memory_item_id).await?;

        match download_single_task_once(client, &task).await {
            Ok(result) => return Ok(result),
            Err(error) => {
                let should_retry = attempt < MAX_TRANSIENT_RETRIES && error.is_retryable();
                if !should_retry {
                    eprintln!(
                        "[downloader-debug] final failure memory_item_id={} attempt={} retryable={} error={}",
                        task.memory_item_id,
                        attempt + 1,
                        error.is_retryable(),
                        error
                    );
                    return Err(error);
                }

                let backoff = BASE_RETRY_DELAY_MS * 2_u64.pow(attempt as u32);
                eprintln!(
                    "[downloader-debug] transient failure memory_item_id={} attempt={} retrying_in_ms={} error={}",
                    task.memory_item_id,
                    attempt + 1,
                    backoff,
                    error
                );
                wait_for_pause_or_stop(task.memory_item_id).await?;
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                attempt += 1;
            }
        }
    }
}

async fn wait_for_pause_or_stop(memory_item_id: i64) -> Result<(), DownloadError> {
    loop {
        let state = crate::core::state::snapshot();

        if state.is_stopped {
            return Err(DownloadError::Stopped { memory_item_id });
        }

        if !state.is_paused {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn acquire_permit_with_control(
    semaphore: Arc<Semaphore>,
    memory_item_id: i64,
) -> Result<OwnedSemaphorePermit, DownloadError> {
    loop {
        wait_for_pause_or_stop(memory_item_id).await?;

        match semaphore.clone().try_acquire_owned() {
            Ok(permit) => return Ok(permit),
            Err(tokio::sync::TryAcquireError::NoPermits) => {
                tokio::time::sleep(Duration::from_millis(30)).await;
            }
            Err(tokio::sync::TryAcquireError::Closed) => {
                return Err(DownloadError::SemaphoreClosed);
            }
        }
    }
}

async fn download_single_task_once(
    client: &reqwest::Client,
    task: &DownloadTask,
) -> Result<DownloadResult, DownloadError> {
    let bytes = download_media(client, task.memory_item_id, &task.url).await?;

    if let Some(parent_dir) = task.destination_path.parent() {
        tokio::fs::create_dir_all(parent_dir)
            .await
            .map_err(|source| DownloadError::Io {
                memory_item_id: task.memory_item_id,
                path: parent_dir.to_path_buf(),
                source,
            })?;
    }

    let (resolved_destination_path, bytes_written) = persist_download_payload(task, &bytes).await?;

    Ok(DownloadResult {
        memory_item_id: task.memory_item_id,
        source_url: task.url.clone(),
        destination_path: resolved_destination_path,
        bytes_written,
    })
}

async fn persist_download_payload(
    task: &DownloadTask,
    bytes: &[u8],
) -> Result<(PathBuf, usize), DownloadError> {
    if looks_like_zip_payload(bytes) {
        match extract_zip_payload_for_memory(task.memory_item_id, bytes, &task.destination_path) {
            Ok((main_path, bytes_written)) => {
                return Ok((main_path, bytes_written));
            }
            Err(error) => {
                eprintln!(
                    "[downloader-debug] ZIP payload extraction failed for memory_item_id={} (url={}): {}. Falling back to raw payload write.",
                    task.memory_item_id,
                    task.url,
                    error
                );
            }
        }
    }

    tokio::fs::write(&task.destination_path, bytes)
        .await
        .map_err(|source| DownloadError::Io {
            memory_item_id: task.memory_item_id,
            path: task.destination_path.clone(),
            source,
        })?;

    Ok((task.destination_path.clone(), bytes.len()))
}

fn looks_like_zip_payload(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }

    bytes[0] == b'P'
        && bytes[1] == b'K'
        && matches!(bytes[2], 3 | 5 | 7)
        && matches!(bytes[3], 4 | 6 | 8)
}

fn extract_zip_payload_for_memory(
    memory_item_id: i64,
    bytes: &[u8],
    destination_path: &Path,
) -> Result<(PathBuf, usize), std::io::Error> {
    let Some(parent_dir) = destination_path.parent() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination path has no parent directory",
        ));
    };

    std::fs::create_dir_all(parent_dir)?;
    cleanup_existing_main_file(parent_dir, memory_item_id)?;
    cleanup_existing_overlay_files(parent_dir, memory_item_id)?;

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zip_error_to_io_error)?;

    let mut main_index: Option<usize> = None;
    let mut main_name: Option<String> = None;
    let mut overlay_index: Option<usize> = None;
    let mut overlay_name: Option<String> = None;
    let mut fallback_main_index: Option<usize> = None;
    let mut fallback_main_name: Option<String> = None;
    let mut fallback_overlay_index: Option<usize> = None;
    let mut fallback_overlay_name: Option<String> = None;

    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(zip_error_to_io_error)?;
        if entry.is_dir() {
            continue;
        }

        let name = entry.name().to_string();
        let lower_file_name = file_name_lower(&name);
        let is_media = is_supported_media_file(&lower_file_name);
        let is_overlay_media = is_supported_overlay_file(&lower_file_name);

        if !is_media {
            continue;
        }

        if fallback_main_index.is_none() {
            fallback_main_index = Some(index);
            fallback_main_name = Some(name.clone());
        }

        if is_overlay_media && fallback_overlay_index.is_none() {
            fallback_overlay_index = Some(index);
            fallback_overlay_name = Some(name.clone());
        }

        if main_index.is_none() && lower_file_name.contains("-main") {
            main_index = Some(index);
            main_name = Some(name.clone());
            continue;
        }

        if overlay_index.is_none() && is_overlay_media && lower_file_name.contains("-overlay") {
            overlay_index = Some(index);
            overlay_name = Some(name);
        }
    }

    let (selected_main_index, selected_main_name) =
        if let (Some(index), Some(name)) = (main_index, main_name) {
            (index, name)
        } else if let (Some(index), Some(name)) = (fallback_main_index, fallback_main_name) {
            (index, name)
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ZIP payload did not contain a supported media file",
            ));
        };

    let (selected_overlay_index, selected_overlay_name) =
        if let (Some(index), Some(name)) = (overlay_index, overlay_name) {
            (Some(index), Some(name))
        } else if let (Some(index), Some(name)) = (fallback_overlay_index, fallback_overlay_name) {
            if index != selected_main_index {
                (Some(index), Some(name))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

    let mut main_entry = archive
        .by_index(selected_main_index)
        .map_err(zip_error_to_io_error)?;
    let mut main_bytes = Vec::new();
    main_entry.read_to_end(&mut main_bytes)?;
    drop(main_entry);

    let main_extension = extension_from_entry_name(&selected_main_name)
        .or_else(|| {
            destination_path
                .extension()
                .and_then(|value| value.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "bin".to_string());
    let main_output_path = parent_dir.join(format!("{memory_item_id}.{main_extension}"));
    std::fs::write(&main_output_path, &main_bytes)?;

    let mut bytes_written = main_bytes.len();

    if let (Some(overlay_index), Some(overlay_name)) =
        (selected_overlay_index, selected_overlay_name)
    {
        let mut overlay_entry = archive
            .by_index(overlay_index)
            .map_err(zip_error_to_io_error)?;
        let mut overlay_bytes = Vec::new();
        overlay_entry.read_to_end(&mut overlay_bytes)?;

        let overlay_extension =
            extension_from_entry_name(&overlay_name).unwrap_or_else(|| "png".to_string());
        let overlay_output_path =
            parent_dir.join(format!("{memory_item_id}.overlay.{overlay_extension}"));
        std::fs::write(overlay_output_path, &overlay_bytes)?;
        bytes_written += overlay_bytes.len();
    }

    Ok((main_output_path, bytes_written))
}

fn cleanup_existing_main_file(
    parent_dir: &Path,
    memory_item_id: i64,
) -> Result<(), std::io::Error> {
    if !parent_dir.exists() {
        return Ok(());
    }

    let main_stem = memory_item_id.to_string();
    for entry in std::fs::read_dir(parent_dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }

        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };

        if stem == main_stem {
            let _ = std::fs::remove_file(path);
        }
    }

    Ok(())
}

fn cleanup_existing_overlay_files(
    parent_dir: &Path,
    memory_item_id: i64,
) -> Result<(), std::io::Error> {
    if !parent_dir.exists() {
        return Ok(());
    }

    let overlay_prefix = format!("{memory_item_id}.overlay");
    for entry in std::fs::read_dir(parent_dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }

        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };

        if stem == overlay_prefix || stem.starts_with(&(overlay_prefix.clone() + ".")) {
            let _ = std::fs::remove_file(path);
        }
    }

    Ok(())
}

fn file_name_lower(path_like: &str) -> String {
    Path::new(path_like)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn extension_from_entry_name(path_like: &str) -> Option<String> {
    Path::new(path_like)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
}

fn is_supported_media_file(file_name_lower: &str) -> bool {
    matches!(
        extension_from_entry_name(file_name_lower).as_deref(),
        Some("jpg")
            | Some("jpeg")
            | Some("png")
            | Some("webp")
            | Some("mp4")
            | Some("mov")
            | Some("m4v")
            | Some("webm")
    )
}

fn is_supported_overlay_file(file_name_lower: &str) -> bool {
    matches!(
        extension_from_entry_name(file_name_lower).as_deref(),
        Some("png") | Some("webp") | Some("jpg") | Some("jpeg")
    )
}

fn zip_error_to_io_error(error: zip::result::ZipError) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("zip parse failed: {error}"),
    )
}

pub async fn download_media(
    client: &reqwest::Client,
    memory_item_id: i64,
    url: &str,
) -> Result<Vec<u8>, DownloadError> {
    let get_response = client
        .get(url)
        .send()
        .await
        .map_err(|source| DownloadError::Http {
            memory_item_id,
            url: url.to_string(),
            source,
        })?;

    let response = if get_response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
        eprintln!(
            "[downloader-debug] GET returned 405 for memory_item_id={} url='{}'; retrying with POST",
            memory_item_id,
            url
        );

        client
            .post(url)
            .send()
            .await
            .map_err(|source| DownloadError::Http {
                memory_item_id,
                url: url.to_string(),
                source,
            })?
    } else {
        get_response
    };

    let response = response
        .error_for_status()
        .map_err(|source| DownloadError::Http {
            memory_item_id,
            url: url.to_string(),
            source,
        })?;

    let bytes = response
        .bytes()
        .await
        .map_err(|source| DownloadError::Http {
            memory_item_id,
            url: url.to_string(),
            source,
        })?;

    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};
    use std::sync::Arc;
    use std::time::Duration;

    use crate::core::parser::import_memories_history_file;
    use sqlx::Row;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::{oneshot, Mutex};

    use super::{
        download_tasks, extract_zip_payload_for_memory, DownloadTask, MAX_CONCURRENT_DOWNLOADS,
    };

    #[derive(Default)]
    struct ServerStats {
        active_requests: usize,
        max_active_requests: usize,
    }

    #[test]
    fn extracts_main_and_overlay_from_zip_payload() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let destination_path = temp_dir.path().join("42.bin");

        let zip_bytes = {
            let cursor = Cursor::new(Vec::<u8>::new());
            let mut writer = zip::ZipWriter::new(cursor);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            writer
                .start_file("memories/2026-04-12_mid-main.mp4", options)
                .expect("main entry should be created");
            writer
                .write_all(b"main-video-bytes")
                .expect("main entry should be written");

            writer
                .start_file("memories/2026-04-12_mid-overlay.png", options)
                .expect("overlay entry should be created");
            writer
                .write_all(b"overlay-bytes")
                .expect("overlay entry should be written");

            writer
                .finish()
                .expect("zip payload should finish")
                .into_inner()
        };

        let (main_path, bytes_written) =
            extract_zip_payload_for_memory(42, &zip_bytes, &destination_path)
                .expect("zip payload extraction should succeed");

        assert_eq!(main_path, temp_dir.path().join("42.mp4"));
        assert_eq!(
            std::fs::read(&main_path).expect("main file should exist"),
            b"main-video-bytes"
        );

        let overlay_path = temp_dir.path().join("42.overlay.png");
        assert_eq!(
            std::fs::read(&overlay_path).expect("overlay file should exist"),
            b"overlay-bytes"
        );

        assert_eq!(
            bytes_written,
            b"main-video-bytes".len() + b"overlay-bytes".len()
        );
    }

    #[tokio::test]
    async fn downloads_with_max_10_parallel_requests() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test server listener should bind");
        let server_address = listener
            .local_addr()
            .expect("test server should expose local address");

        let stats = Arc::new(Mutex::new(ServerStats::default()));
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let server_stats = stats.clone();

        let server = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    accept_result = listener.accept() => {
                        let Ok((mut socket, _)) = accept_result else {
                            break;
                        };

                        let connection_stats = server_stats.clone();
                        tokio::spawn(async move {
                            {
                                let mut stats = connection_stats.lock().await;
                                stats.active_requests += 1;
                                if stats.active_requests > stats.max_active_requests {
                                    stats.max_active_requests = stats.active_requests;
                                }
                            }

                            tokio::time::sleep(Duration::from_millis(100)).await;

                            let body = b"mock-media-bytes";
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            );

                            let _ = socket.write_all(response.as_bytes()).await;
                            let _ = socket.write_all(body).await;
                            let _ = socket.shutdown().await;

                            let mut stats = connection_stats.lock().await;
                            stats.active_requests = stats.active_requests.saturating_sub(1);
                        });
                    }
                }
            }
        });

        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let mut tasks = Vec::new();
        for index in 0..25 {
            tasks.push(DownloadTask {
                memory_item_id: index,
                url: format!("http://{server_address}/media/{index}"),
                destination_path: temp_dir.path().join(format!("{index}.bin")),
            });
        }

        let results = download_tasks(tasks)
            .await
            .expect("download manager should return results");

        for result in &results {
            assert!(result.is_ok(), "download result should succeed");
        }

        let max_observed = {
            let stats = stats.lock().await;
            stats.max_active_requests
        };

        let _ = shutdown_tx.send(());
        let _ = server.await;

        assert!(
            max_observed <= MAX_CONCURRENT_DOWNLOADS,
            "max observed concurrency should be <= {} but was {}",
            MAX_CONCURRENT_DOWNLOADS,
            max_observed
        );

        assert_eq!(results.len(), 25);
    }

    #[tokio::test]
    async fn retries_with_post_when_get_returns_405() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test server listener should bind");
        let server_address = listener
            .local_addr()
            .expect("test server should expose local address");

        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener
                    .accept()
                    .await
                    .expect("test server should accept connection");

                let mut request_buffer = [0_u8; 2048];
                let read_count = socket
                    .read(&mut request_buffer)
                    .await
                    .expect("test server should read request bytes");
                let request = String::from_utf8_lossy(&request_buffer[..read_count]);

                if request.starts_with("GET ") {
                    let response =
                        "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                    continue;
                }

                if request.starts_with("POST ") {
                    let body = b"post-fallback-success";
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.write_all(body).await;
                    let _ = socket.shutdown().await;
                }
            }
        });

        let bytes = super::download_media(
            &reqwest::Client::new(),
            42,
            &format!("http://{server_address}/media/42"),
        )
        .await
        .expect("download should succeed via POST fallback");

        let _ = server.await;

        assert_eq!(bytes, b"post-fallback-success");
    }

    #[tokio::test]
    async fn imports_mock_json_and_downloads_10_files_to_temp_folder() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("verification server listener should bind");
        let server_address = listener
            .local_addr()
            .expect("verification server should expose local address");

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    accept_result = listener.accept() => {
                        let Ok((mut socket, _)) = accept_result else {
                            break;
                        };

                        tokio::spawn(async move {
                            let body = b"mock-ten-file-download";
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            );

                            let _ = socket.write_all(response.as_bytes()).await;
                            let _ = socket.write_all(body).await;
                            let _ = socket.shutdown().await;
                        });
                    }
                }
            }
        });

        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let db_path = temp_dir.path().join("memories.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());
        let output_dir = temp_dir.path().join("downloads");

        let pool = sqlx::SqlitePool::connect(&db_url)
            .await
            .expect("sqlite pool should open for verification test");
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS MemoryItem (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                date_time TEXT,
                location TEXT,
                location_resolved TEXT,
                media_url TEXT NOT NULL,
                media_download_url TEXT,
                overlay_url TEXT,
                status TEXT NOT NULL
            )
            ",
        )
        .execute(&pool)
        .await
        .expect("memory table should be created for verification test");

        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS Memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash TEXT NOT NULL UNIQUE,
                date TEXT NOT NULL,
                status TEXT NOT NULL,
                job_id TEXT,
                mid TEXT,
                content_hash TEXT,
                relative_path TEXT,
                thumbnail_path TEXT
            )
            ",
        )
        .execute(&pool)
        .await
        .expect("memories table should be created for verification test");

        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS MediaChunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                memory_id INTEGER NOT NULL,
                url TEXT NOT NULL,
                overlay_url TEXT,
                order_index INTEGER NOT NULL,
                FOREIGN KEY (memory_id) REFERENCES Memories(id)
            )
            ",
        )
        .execute(&pool)
        .await
        .expect("media chunks table should be created for verification test");
        pool.close().await;

        let mock_items = (1..=10)
            .map(|index| {
                serde_json::json!({
                    "download_url": format!("http://{server_address}/media/{index}"),
                    "date": format!("2024-01-{index:02}T00:00:00Z")
                })
            })
            .collect::<Vec<_>>();

        let json_path = temp_dir.path().join("memories_history.json");
        tokio::fs::write(
            &json_path,
            serde_json::json!({
                "Saved Media": mock_items
            })
            .to_string(),
        )
        .await
        .expect("mock memories_history json should be written");

        let imported_count = import_memories_history_file(&db_url, &json_path)
            .await
            .expect("mock memories should be imported");
        assert_eq!(imported_count, 10);

        let verification_pool = sqlx::SqlitePool::connect(&db_url)
            .await
            .expect("sqlite pool should re-open for queue read");

        let rows = sqlx::query("SELECT id, media_url FROM MemoryItem ORDER BY id ASC")
            .fetch_all(&verification_pool)
            .await
            .expect("memory rows should be fetched");

        let tasks = rows
            .iter()
            .map(|row| DownloadTask {
                memory_item_id: row.get::<i64, _>("id"),
                url: row.get::<String, _>("media_url"),
                destination_path: output_dir.join(format!("{}.bin", row.get::<i64, _>("id"))),
            })
            .collect::<Vec<_>>();

        let results = download_tasks(tasks)
            .await
            .expect("download manager should return verification results");

        let _ = shutdown_tx.send(());
        let _ = server.await;

        assert_eq!(results.len(), 10);
        assert!(results.iter().all(Result::is_ok));

        let mut downloaded_files = 0usize;
        let mut directory_entries = tokio::fs::read_dir(&output_dir)
            .await
            .expect("output directory should exist after downloads");
        while directory_entries
            .next_entry()
            .await
            .expect("directory entries should be readable")
            .is_some()
        {
            downloaded_files += 1;
        }

        verification_pool.close().await;

        assert_eq!(downloaded_files, 10);
    }
}
