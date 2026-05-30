use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RepoVisibility {
    Public,
    Private,
    Internal,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct GooseBotRepo {
    pub id: u64,
    pub full_name: String,
    pub name: String,
    pub owner: String,
    #[serde(default)]
    pub visibility: RepoVisibility,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub default_branch: String,
    #[serde(default)]
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct GooseBotReposResponse {
    pub total_count: u64,
    pub repos: Vec<GooseBotRepo>,
    #[serde(default)]
    pub truncated: bool,
}
