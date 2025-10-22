use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::state::AppState;

/// Response containing log directory size information
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct LogSizeResponse {
    /// Total size in bytes
    pub total_bytes: u64,
    /// Total size in megabytes (for display)
    pub total_mb: f64,
    /// Total size in gigabytes (for display)
    pub total_gb: f64,
    /// Number of log files found
    pub file_count: usize,
    /// Log directory path
    pub log_path: String,
}

/// Response after clearing logs
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct ClearLogsResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Number of files removed/archived
    pub files_cleared: usize,
    /// Bytes reclaimed
    pub bytes_cleared: u64,
    /// Megabytes reclaimed
    pub mb_cleared: f64,
    /// Optional error message
    pub message: Option<String>,
}

/// Response containing log directory path
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct LogPathResponse {
    /// Log directory path
    pub log_path: String,
}

/// Get the base log directory path
fn get_log_base_dir() -> Result<PathBuf> {
    let log_dir = goose::logging::get_log_directory("server", false)?;
    // Navigate up to the logs root directory
    let base_dir = log_dir
        .parent()
        .context("Failed to get parent of log directory")?;
    Ok(base_dir.to_path_buf())
}

/// Recursively compute directory size and count log files
fn compute_directory_size(dir: &Path) -> Result<(u64, usize)> {
    let mut total_size: u64 = 0;
    let mut file_count: usize = 0;

    if !dir.exists() {
        return Ok((0, 0));
    }

    let entries = fs::read_dir(dir).context("Failed to read directory")?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            // Check if it's a log file
            if let Some(extension) = path.extension() {
                if extension == "log" {
                    if let Ok(metadata) = fs::metadata(&path) {
                        total_size += metadata.len();
                        file_count += 1;
                    }
                }
            }
        } else if path.is_dir() {
            // Recursively process subdirectories
            let (sub_size, sub_count) = compute_directory_size(&path)?;
            total_size += sub_size;
            file_count += sub_count;
        }
    }

    Ok((total_size, file_count))
}

/// Find all log files in a directory recursively
fn find_log_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut log_files = Vec::new();

    if !dir.exists() {
        return Ok(log_files);
    }

    let entries = fs::read_dir(dir).context("Failed to read directory")?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            // Check if it's a log file
            if let Some(extension) = path.extension() {
                if extension == "log" {
                    log_files.push(path);
                }
            }
        } else if path.is_dir() {
            // Recursively process subdirectories
            let mut sub_files = find_log_files(&path)?;
            log_files.append(&mut sub_files);
        }
    }

    Ok(log_files)
}

/// Archive a single log file by moving it to an archive directory
fn archive_log_file(file_path: &Path, archive_dir: &Path) -> Result<()> {
    // Create archive directory if it doesn't exist
    fs::create_dir_all(archive_dir).context("Failed to create archive directory")?;

    // Generate archive filename with timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let original_name = file_path
        .file_name()
        .context("Failed to get filename")?
        .to_string_lossy();

    let archive_name = format!("archived-{}-{}", timestamp, original_name);
    let archive_path = archive_dir.join(archive_name);

    // Move file to archive
    fs::rename(file_path, &archive_path)
        .or_else(|_| {
            // If rename fails (cross-device), try copy and delete
            fs::copy(file_path, &archive_path)?;
            fs::remove_file(file_path)?;
            Ok::<(), std::io::Error>(())
        })
        .with_context(|| format!("Failed to archive file: {}", file_path.display()))?;

    Ok(())
}

/// Clear log files safely by archiving them
fn clear_logs_safely(log_base_dir: &Path) -> Result<(usize, u64)> {
    // Create archive directory in the same location
    let archive_dir = log_base_dir.join("archived");

    // Find all log files
    let log_files = find_log_files(log_base_dir)?;

    let mut files_cleared = 0;
    let mut bytes_cleared: u64 = 0;

    for file_path in log_files {
        // Skip files in the archive directory itself
        if file_path.starts_with(&archive_dir) {
            continue;
        }

        // Get file size before archiving
        if let Ok(metadata) = fs::metadata(&file_path) {
            let file_size = metadata.len();

            match archive_log_file(&file_path, &archive_dir) {
                Ok(_) => {
                    files_cleared += 1;
                    bytes_cleared += file_size;
                    info!("Archived log file: {}", file_path.display());
                }
                Err(e) => {
                    warn!("Failed to archive file {}: {}", file_path.display(), e);
                }
            }
        }
    }

    Ok((files_cleared, bytes_cleared))
}

/// Handler to get log directory size
pub async fn get_log_size(_state: State<AppState>) -> impl IntoResponse {
    match get_log_base_dir() {
        Ok(log_dir) => match compute_directory_size(&log_dir) {
            Ok((total_bytes, file_count)) => {
                let total_mb = total_bytes as f64 / (1024.0 * 1024.0);
                let total_gb = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

                let response = LogSizeResponse {
                    total_bytes,
                    total_mb,
                    total_gb,
                    file_count,
                    log_path: log_dir.display().to_string(),
                };

                info!(
                    "Log size check: {} files, {:.2} MB ({:.2} GB)",
                    file_count, total_mb, total_gb
                );

                (StatusCode::OK, Json(response))
            }
            Err(e) => {
                error!("Failed to compute directory size: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(LogSizeResponse {
                        total_bytes: 0,
                        total_mb: 0.0,
                        total_gb: 0.0,
                        file_count: 0,
                        log_path: "".to_string(),
                    }),
                )
            }
        },
        Err(e) => {
            error!("Failed to get log directory: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LogSizeResponse {
                    total_bytes: 0,
                    total_mb: 0.0,
                    total_gb: 0.0,
                    file_count: 0,
                    log_path: "".to_string(),
                }),
            )
        }
    }
}

/// Handler to clear log files
pub async fn clear_logs(_state: State<AppState>) -> impl IntoResponse {
    match get_log_base_dir() {
        Ok(log_dir) => {
            info!("Attempting to clear logs in: {}", log_dir.display());

            match clear_logs_safely(&log_dir) {
                Ok((files_cleared, bytes_cleared)) => {
                    let mb_cleared = bytes_cleared as f64 / (1024.0 * 1024.0);

                    info!(
                        "Successfully cleared {} log files, reclaimed {:.2} MB",
                        files_cleared, mb_cleared
                    );

                    let response = ClearLogsResponse {
                        success: true,
                        files_cleared,
                        bytes_cleared,
                        mb_cleared,
                        message: Some(format!(
                            "Successfully cleared {} log file(s)",
                            files_cleared
                        )),
                    };

                    (StatusCode::OK, Json(response))
                }
                Err(e) => {
                    error!("Failed to clear logs: {}", e);
                    let response = ClearLogsResponse {
                        success: false,
                        files_cleared: 0,
                        bytes_cleared: 0,
                        mb_cleared: 0.0,
                        message: Some(format!("Failed to clear logs: {}", e)),
                    };

                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                }
            }
        }
        Err(e) => {
            error!("Failed to get log directory: {}", e);
            let response = ClearLogsResponse {
                success: false,
                files_cleared: 0,
                bytes_cleared: 0,
                mb_cleared: 0.0,
                message: Some(format!("Failed to get log directory: {}", e)),
            };

            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Handler to get log directory path
pub async fn get_log_path(_state: State<AppState>) -> impl IntoResponse {
    match get_log_base_dir() {
        Ok(log_dir) => {
            let response = LogPathResponse {
                log_path: log_dir.display().to_string(),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            error!("Failed to get log directory: {}", e);
            let response = LogPathResponse {
                log_path: "".to_string(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// Configure log management routes
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/logs/size", get(get_log_size))
        .route("/logs/clear", post(clear_logs))
        .route("/logs/path", get(get_log_path))
        .with_state(state.as_ref().clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_compute_directory_size_empty() {
        let temp_dir = TempDir::new().unwrap();
        let (size, count) = compute_directory_size(temp_dir.path()).unwrap();
        assert_eq!(size, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_compute_directory_size_with_logs() {
        let temp_dir = TempDir::new().unwrap();

        // Create test log files
        let log1 = temp_dir.path().join("test1.log");
        let mut file1 = File::create(&log1).unwrap();
        file1.write_all(b"test content").unwrap();

        let log2 = temp_dir.path().join("test2.log");
        let mut file2 = File::create(&log2).unwrap();
        file2.write_all(b"more test content").unwrap();

        let (size, count) = compute_directory_size(temp_dir.path()).unwrap();
        assert!(size > 0);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_find_log_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let log1 = temp_dir.path().join("test1.log");
        File::create(&log1).unwrap();

        let not_log = temp_dir.path().join("test.txt");
        File::create(&not_log).unwrap();

        let log2 = temp_dir.path().join("test2.log");
        File::create(&log2).unwrap();

        let files = find_log_files(temp_dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_archive_log_file() {
        let temp_dir = TempDir::new().unwrap();
        let archive_dir = temp_dir.path().join("archive");

        // Create a test log file
        let log_file = temp_dir.path().join("test.log");
        let mut file = File::create(&log_file).unwrap();
        file.write_all(b"test content").unwrap();
        drop(file);

        // Archive the file
        archive_log_file(&log_file, &archive_dir).unwrap();

        // Verify original file is gone and archived file exists
        assert!(!log_file.exists());
        assert!(archive_dir.exists());

        let archived_files = fs::read_dir(&archive_dir).unwrap();
        assert_eq!(archived_files.count(), 1);
    }
}
