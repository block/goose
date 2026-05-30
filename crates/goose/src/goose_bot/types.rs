use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::GooseBotPrefs;

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct GooseBotReviewRequest {
    pub github_token: String,
    pub repo: String,
    pub pr_number: u64,
    pub head_sha: String,
    pub pr_url: String,
    #[serde(default)]
    pub check_run_id: Option<u64>,
    #[serde(default)]
    pub comment_id: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GooseBotReviewResponse {
    pub accepted: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GooseBotSetupResponse {
    pub installation_id: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GooseBotStatusResponse {
    pub installation_id: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GooseBotDisconnectResponse {
    pub disconnected: bool,
}

#[derive(Debug, Deserialize, Clone, ToSchema)]
pub struct GooseBotCommentRequest {
    pub github_token: String,
    pub repo: String,
    pub pr_number: u64,
    pub pr_url: String,
    pub comment_body: String,
    pub commenter: String,
    #[serde(default)]
    pub head_ref: String,
    #[serde(default)]
    pub head_repo: String,
    #[serde(default)]
    pub comment_id: Option<u64>,
    /// Omitted in older switchboard payloads (treated as PR); current switchboard always sends this.
    #[serde(default = "is_pr_absent_legacy_default")]
    pub is_pr: bool,
}

fn is_pr_absent_legacy_default() -> bool {
    true
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GooseBotPrefsResponse {
    pub prefs: GooseBotPrefs,
    pub switchboard_synced: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub switchboard_error: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GooseBotPrefsRequest {
    pub prefs: GooseBotPrefs,
}
