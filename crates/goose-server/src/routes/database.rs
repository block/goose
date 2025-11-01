use axum::{
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use goose::config::paths::Paths;
use goose::session::session_manager::SessionManager;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use utoipa::ToSchema;

use crate::routes::errors::ErrorResponse;

static BACKUP_MUTEX: Lazy<Arc<Mutex<()>>> = Lazy::new(|| Arc::new(Mutex::new(())));
static RESTORE_MUTEX: Lazy<Arc<Mutex<()>>> = Lazy::new(|| Arc::new(Mutex::new(())));

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub size: u64,
    pub schema_version: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateBackupRequest {
    /// Optional custom name for the backup
    pub name: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateBackupResponse {
    /// Filename of the created backup
    pub filename: String,
    /// Size of the backup file in bytes
    pub size: u64,
    /// When the backup was created
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupsListResponse {
    /// List of available backups
    pub backups: Vec<BackupInfo>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RestoreBackupRequest {
    /// Filename of the backup to restore
    pub filename: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RestoreBackupResponse {
    /// Success message
    pub message: String,
    /// Backup that was restored
    pub restored_from: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteBackupsRequest {
    /// List of backup filenames to delete
    pub filenames: Vec<String>,
    /// Delete all backups if true
    pub delete_all: bool,
    /// Clean up orphaned WAL/SHM files
    pub cleanup_orphaned: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteBackupsResponse {
    /// Successfully deleted files
    pub deleted: Vec<String>,
    /// Files that failed to delete
    pub failed: Vec<String>,
    /// Number of orphaned files cleaned
    pub orphaned_cleaned: usize,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatusResponse {
    pub db_size: u64,
    pub schema_version: i32,
    pub is_latest_version: bool,
    pub session_count: usize,
    pub message_count: usize,
    pub total_tokens: i64,
    pub latest_backup: Option<BackupInfo>,
    pub backup_count: usize,
    pub timestamp: DateTime<Utc>,
}

#[utoipa::path(
    get,
    path = "/database/status",
    tag = "Database Management",
    responses(
        (status = 200, description = "Database status retrieved successfully",
         body = DatabaseStatusResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn database_status() -> Result<Json<DatabaseStatusResponse>, StatusCode> {
    let stats = SessionManager::get_database_stats()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let latest_backup = stats.latest_backup.map(|backup| {
        let filename = backup
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        BackupInfo {
            filename,
            created_at: backup.created_at,
            size: backup.size,
            schema_version: backup.schema_version,
        }
    });

    Ok(Json(DatabaseStatusResponse {
        db_size: stats.db_size,
        schema_version: stats.schema_version,
        is_latest_version: stats.is_latest_version,
        session_count: stats.session_count,
        message_count: stats.message_count,
        total_tokens: stats.total_tokens,
        latest_backup,
        backup_count: stats.backup_count,
        timestamp: Utc::now(),
    }))
}

#[utoipa::path(
    post,
    path = "/database/backup",
    request_body = CreateBackupRequest,
    responses(
        (status = 201, description = "Backup created successfully", body = CreateBackupResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Database Management"
)]
pub async fn create_backup(
    Json(request): Json<CreateBackupRequest>,
) -> Result<(StatusCode, Json<CreateBackupResponse>), ErrorResponse> {
    let _lock = BACKUP_MUTEX.try_lock().map_err(|_| ErrorResponse {
        message: "A backup operation is already in progress".to_string(),
        status: StatusCode::CONFLICT,
    })?;

    let backup_path = SessionManager::create_backup(request.name)
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Failed to create backup: {}", e),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let filename = backup_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let metadata = tokio::fs::metadata(&backup_path)
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Failed to read backup file metadata: {}", e),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let size = metadata.len();
    let created_at = Utc::now();

    Ok((
        StatusCode::CREATED,
        Json(CreateBackupResponse {
            filename,
            size,
            created_at,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/database/backups",
    responses(
        (status = 200, description = "List of backups retrieved successfully", body = BackupsListResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Database Management"
)]
pub async fn list_backups() -> Result<Json<BackupsListResponse>, ErrorResponse> {
    let backups = SessionManager::list_backups()
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Failed to list backups: {}", e),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let sanitized_backups = backups
        .into_iter()
        .map(|backup| {
            let filename = backup
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            BackupInfo {
                filename,
                created_at: backup.created_at,
                size: backup.size,
                schema_version: backup.schema_version,
            }
        })
        .collect();

    Ok(Json(BackupsListResponse {
        backups: sanitized_backups,
    }))
}

#[utoipa::path(
    post,
    path = "/database/restore",
    request_body = RestoreBackupRequest,
    responses(
        (status = 200, description = "Database restored successfully", body = RestoreBackupResponse),
        (status = 400, description = "Bad request - Invalid backup filename"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 404, description = "Backup not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Database Management"
)]
pub async fn restore_backup(
    Json(request): Json<RestoreBackupRequest>,
) -> Result<Json<RestoreBackupResponse>, ErrorResponse> {
    let _lock = RESTORE_MUTEX.try_lock().map_err(|_| ErrorResponse {
        message: "A restore operation is already in progress".to_string(),
        status: StatusCode::CONFLICT,
    })?;

    let filename = request.filename.trim();
    if filename.is_empty() {
        return Err(ErrorResponse {
            message: "Backup filename cannot be empty".to_string(),
            status: StatusCode::BAD_REQUEST,
        });
    }

    let backup_dir = Paths::backup_dir();
    let backup_path = backup_dir.join(filename);

    let canonical_backup = backup_path.canonicalize().map_err(|_| ErrorResponse {
        message: format!("Backup '{}' not found", filename),
        status: StatusCode::NOT_FOUND,
    })?;
    let canonical_backup_dir = backup_dir.canonicalize().map_err(|e| ErrorResponse {
        message: format!("Failed to access backup directory: {}", e),
        status: StatusCode::INTERNAL_SERVER_ERROR,
    })?;

    if !canonical_backup.starts_with(&canonical_backup_dir) {
        tracing::warn!("Path traversal attempt detected: {}", filename);
        return Err(ErrorResponse {
            message: "Invalid backup path".to_string(),
            status: StatusCode::FORBIDDEN,
        });
    }

    if !canonical_backup.exists() {
        return Err(ErrorResponse {
            message: format!("Backup '{}' not found", filename),
            status: StatusCode::NOT_FOUND,
        });
    }

    SessionManager::restore_backup(&canonical_backup)
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Failed to restore backup: {}", e),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(Json(RestoreBackupResponse {
        message: "Database restored successfully. Server restart required.".to_string(),
        restored_from: filename.to_string(),
    }))
}

#[utoipa::path(
    delete,
    path = "/database/backups/delete",
    request_body = DeleteBackupsRequest,
    responses(
        (status = 200, description = "Backups deleted successfully", body = DeleteBackupsResponse),
        (status = 400, description = "Bad request - Invalid parameters"),
        (status = 401, description = "Unauthorized - Invalid or missing API key"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "Database Management"
)]
pub async fn delete_backups(
    Json(request): Json<DeleteBackupsRequest>,
) -> Result<Json<DeleteBackupsResponse>, ErrorResponse> {
    let _lock = BACKUP_MUTEX.try_lock().map_err(|_| ErrorResponse {
        message: "A backup operation is in progress, cannot delete".to_string(),
        status: StatusCode::CONFLICT,
    })?;

    if request.delete_all && !request.filenames.is_empty() {
        return Err(ErrorResponse {
            message: "Cannot specify both deleteAll and specific filenames".to_string(),
            status: StatusCode::BAD_REQUEST,
        });
    }

    if !request.delete_all && request.filenames.is_empty() {
        return Err(ErrorResponse {
            message: "Must specify either deleteAll or at least one filename".to_string(),
            status: StatusCode::BAD_REQUEST,
        });
    }

    let backup_dir = Paths::backup_dir();

    let files_to_delete: Vec<PathBuf> = if request.delete_all {
        let backups = SessionManager::list_backups()
            .await
            .map_err(|e| ErrorResponse {
                message: format!("Failed to list backups: {}", e),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        backups.into_iter().map(|b| b.path).collect()
    } else {
        let mut resolved_paths = Vec::new();
        for filename in &request.filenames {
            let path = backup_dir.join(filename);
            if path.exists() {
                let canonical = path.canonicalize().map_err(|e| ErrorResponse {
                    message: format!("Failed to resolve backup path: {}", e),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
                let canonical_backup_dir =
                    backup_dir.canonicalize().map_err(|e| ErrorResponse {
                        message: format!("Failed to access backup directory: {}", e),
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                    })?;

                if !canonical.starts_with(&canonical_backup_dir) {
                    tracing::warn!("Path traversal attempt detected: {}", filename);
                    return Err(ErrorResponse {
                        message: "Invalid backup path".to_string(),
                        status: StatusCode::FORBIDDEN,
                    });
                }
                resolved_paths.push(canonical);
            }
        }
        resolved_paths
    };

    let mut deleted = Vec::new();
    let mut failed = Vec::new();

    for path in files_to_delete {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        match tokio::fs::remove_file(&path).await {
            Ok(_) => {
                deleted.push(filename.clone());

                let wal_path = path.with_extension("db-wal");
                if wal_path.exists() {
                    let _ = tokio::fs::remove_file(&wal_path).await;
                }

                let shm_path = path.with_extension("db-shm");
                if shm_path.exists() {
                    let _ = tokio::fs::remove_file(&shm_path).await;
                }
            }
            Err(_) => {
                failed.push(filename);
            }
        }
    }

    let mut orphaned_cleaned = 0;
    if request.cleanup_orphaned {
        orphaned_cleaned = cleanup_orphaned_files(&backup_dir).await;
    }

    Ok(Json(DeleteBackupsResponse {
        deleted,
        failed,
        orphaned_cleaned,
    }))
}

async fn cleanup_orphaned_files(backup_dir: &Path) -> usize {
    let mut cleaned = 0;

    if let Ok(mut entries) = tokio::fs::read_dir(backup_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if let Some(extension) = path.extension() {
                let ext_str = extension.to_string_lossy();
                if ext_str == "db-wal" || ext_str == "db-shm" {
                    let db_path = path.with_extension("db");
                    if !db_path.exists() && tokio::fs::remove_file(&path).await.is_ok() {
                        cleaned += 1;
                    }
                }
            }
        }
    }

    cleaned
}

pub fn routes() -> Router {
    Router::new()
        .route("/database/status", get(database_status))
        .route("/database/backup", post(create_backup))
        .route("/database/backups", get(list_backups))
        .route("/database/backups/delete", delete(delete_backups))
        .route("/database/restore", post(restore_backup))
}
