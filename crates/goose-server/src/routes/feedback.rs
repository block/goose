use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct SystemInfo {
    #[serde(rename = "gooseVersion")]
    goose_version: String,
    #[serde(rename = "osVersion")]
    os_version: String,
    platform: String,
    architecture: String,
    #[serde(rename = "providerType")]
    provider_type: Option<String>,
    #[serde(rename = "extensionCount")]
    extension_count: u32,
}

#[derive(Debug, Deserialize)]
struct FailureReportRequest {
    title: String,
    description: String,
    #[serde(rename = "systemInfo")]
    system_info: SystemInfo,
    #[serde(rename = "recentErrors")]
    recent_errors: Vec<String>,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct FailureReportResponse {
    success: bool,
    message: String,
    #[serde(rename = "issueUrl")]
    issue_url: Option<String>,
}

pub fn routes(state: std::sync::Arc<AppState>) -> Router {
    Router::new()
        .route("/report-failure", post(report_failure))
        .with_state(state)
}

async fn report_failure(
    State(_state): State<std::sync::Arc<AppState>>,
    Json(payload): Json<FailureReportRequest>,
) -> Result<Json<FailureReportResponse>, StatusCode> {
    info!(
        "Received failure report: {} - {}",
        payload.title,
        payload.description.chars().take(100).collect::<String>()
    );

    debug!("System info: {:?}", payload.system_info);

    log_failure_report_locally(&payload);

    Ok(Json(FailureReportResponse {
        success: true,
        message: "Failure report logged successfully".to_string(),
        issue_url: None,
    }))
}



fn log_failure_report_locally(payload: &FailureReportRequest) {
    // Log the failure report for manual processing
    error!(
        "FAILURE REPORT (Manual Processing Required): Title: {}, Description: {}, System: {}/{}/{}, Timestamp: {}",
        payload.title,
        payload.description,
        payload.system_info.platform,
        payload.system_info.architecture,
        payload.system_info.goose_version,
        payload.timestamp
    );
}
