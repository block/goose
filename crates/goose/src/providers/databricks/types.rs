use super::*;

#[derive(Debug, Clone)]
pub(super) struct DatabricksEndpointInfo {
    pub name: String,
    pub upstream_model_name: Option<String>,
    pub upstream_model_provider: Option<String>,
    pub reasoning: Option<bool>,
}

#[derive(Debug, Clone)]
pub(super) struct DatabricksUpstreamModel {
    pub name: String,
    pub provider: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CachedDatabricksEndpointInfo {
    pub info: DatabricksEndpointInfo,
    pub fetched_at: Instant,
}
