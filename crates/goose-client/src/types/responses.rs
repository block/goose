use goose::agents::ExtensionConfig;
use goose::agents::ExtensionLoadResult;
use goose::goose_apps::GooseApp;
use goose::recipe::Recipe;
use goose::session::Session;
use rmcp::model::Content;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResponse {
    pub sessions: Vec<Session>,
}

#[derive(Debug, Deserialize)]
pub struct ResumeAgentResponse {
    pub session: Session,
    pub extension_results: Option<Vec<ExtensionLoadResult>>,
}

#[derive(Debug, Deserialize)]
pub struct RestartAgentResponse {
    pub extension_results: Vec<ExtensionLoadResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkResponse {
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CallToolResponse {
    pub content: Vec<Content>,
    pub structured_content: Option<serde_json::Value>,
    pub is_error: bool,
    #[serde(rename = "_meta")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResponse {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: String,
    #[serde(rename = "_meta")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAppsResponse {
    pub apps: Vec<GooseApp>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAppResponse {
    pub name: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionExtensionsResponse {
    pub extensions: Vec<ExtensionConfig>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRecipeValuesResponse {
    pub recipe: Recipe,
}
