use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    // Create GitHub issue content
    let issue_body = create_github_issue_body(&payload);
    
    // Try to create GitHub issue
    match create_github_issue(&payload.title, &issue_body).await {
        Ok(issue_url) => {
            info!("Successfully created GitHub issue: {}", issue_url);
            Ok(Json(FailureReportResponse {
                success: true,
                message: "Failure report submitted successfully".to_string(),
                issue_url: Some(issue_url),
            }))
        }
        Err(e) => {
            error!("Failed to create GitHub issue: {}", e);
            // Fallback: Log the issue locally for manual processing
            log_failure_report_locally(&payload);
            
            Ok(Json(FailureReportResponse {
                success: false,
                message: "Failed to submit report automatically. Please report manually on GitHub.".to_string(),
                issue_url: None,
            }))
        }
    }
}

fn create_github_issue_body(payload: &FailureReportRequest) -> String {
    format!(
        r#"**Describe the bug**

{}

**Please provide following information:**
- **OS & Arch:** {} {}
- **Interface:** UI (Desktop App)  
- **Version:** {}
- **Provider & Model:** {}

**Recent Errors/Logs:**
```
{}
```

**Additional context**
- **Timestamp:** {}
- **Reported via:** Goose Desktop App automated failure reporting

---
*This issue was automatically created via the "Report a Failure" feature.*"#,
        payload.description,
        payload.system_info.platform,
        payload.system_info.architecture,
        payload.system_info.goose_version,
        match &payload.system_info.provider_type {
            Some(provider) => provider.clone(),
            None => "Unknown".to_string(),
        },
        payload.recent_errors.join("\n"),
        payload.timestamp
    )
}

async fn create_github_issue(title: &str, body: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // GitHub API configuration
    let github_token = std::env::var("GITHUB_TOKEN").map_err(|_| "GITHUB_TOKEN not set")?;
    let repo_owner = "block";
    let repo_name = "goose";
    
    let client = reqwest::Client::new();
    
    let mut issue_data = HashMap::new();
    issue_data.insert("title", format!("[FAILURE REPORT] {}", title));
    issue_data.insert("body", body.to_string());
    issue_data.insert("labels", "[\"bug\", \"needs-triage\", \"failure-report\"]".to_string());
    
    let url = format!("https://api.github.com/repos/{}/{}/issues", repo_owner, repo_name);
    
    let response = client
        .post(&url)
        .header("Authorization", format!("token {}", github_token))
        .header("User-Agent", "goose-app")
        .header("Accept", "application/vnd.github.v3+json")
        .json(&issue_data)
        .send()
        .await?;
    
    if response.status().is_success() {
        let issue: serde_json::Value = response.json().await?;
        let issue_url = issue["html_url"].as_str()
            .ok_or("GitHub response missing html_url")?;
        Ok(issue_url.to_string())
    } else {
        let error_text = response.text().await?;
        Err(format!("GitHub API error: {}", error_text).into())
    }
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