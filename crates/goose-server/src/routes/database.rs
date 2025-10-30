use axum::{http::StatusCode, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use goose::session::session_manager::SessionManager;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfoSanitized {
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub size: u64,
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
    pub latest_backup: Option<BackupInfoSanitized>,
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

        BackupInfoSanitized {
            filename,
            created_at: backup.created_at,
            size: backup.size,
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

pub fn routes() -> Router {
    Router::new().route("/database/status", get(database_status))
}
