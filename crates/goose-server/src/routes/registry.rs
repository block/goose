use axum::{extract::Query, routing::get, Json, Router};
use goose::registry::manifest::{RegistryEntry, RegistryEntryKind};
use goose::registry::sources::local::LocalRegistrySource;
use goose::registry::RegistryManager;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::routes::errors::ErrorResponse;

#[derive(Debug, Serialize, ToSchema)]
pub struct RegistryListResponse {
    pub entries: Vec<RegistryEntry>,
    pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RegistrySourcesResponse {
    pub sources: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegistrySearchParams {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
}

fn parse_kind(kind: &Option<String>) -> Option<RegistryEntryKind> {
    kind.as_deref()
        .and_then(|k| match k.to_lowercase().as_str() {
            "tool" => Some(RegistryEntryKind::Tool),
            "skill" => Some(RegistryEntryKind::Skill),
            "agent" => Some(RegistryEntryKind::Agent),
            "recipe" => Some(RegistryEntryKind::Recipe),
            _ => None,
        })
}

fn default_manager() -> RegistryManager {
    let mut manager = RegistryManager::new();
    if let Ok(source) = LocalRegistrySource::from_default_paths() {
        manager.add_source(Box::new(source));
    }
    manager
}

#[utoipa::path(
    get,
    path = "/registry/search",
    params(
        ("query" = Option<String>, Query, description = "Search query to filter entries"),
        ("kind" = Option<String>, Query, description = "Filter by kind: tool, skill, agent, recipe")
    ),
    responses(
        (status = 200, description = "Search results", body = RegistryListResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Registry"
)]
pub async fn search_registry(
    Query(params): Query<RegistrySearchParams>,
) -> Result<Json<RegistryListResponse>, ErrorResponse> {
    let manager = default_manager();
    let kind = parse_kind(&params.kind);
    let entries = manager
        .search(params.query.as_deref(), kind)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Registry search failed: {e}")))?;
    let total = entries.len();
    Ok(Json(RegistryListResponse { entries, total }))
}

#[utoipa::path(
    get,
    path = "/registry/entries",
    params(
        ("kind" = Option<String>, Query, description = "Filter by kind: tool, skill, agent, recipe")
    ),
    responses(
        (status = 200, description = "All registry entries", body = RegistryListResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Registry"
)]
pub async fn list_registry(
    Query(params): Query<RegistrySearchParams>,
) -> Result<Json<RegistryListResponse>, ErrorResponse> {
    let manager = default_manager();
    let kind = parse_kind(&params.kind);
    let entries = manager
        .list(kind)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Registry list failed: {e}")))?;
    let total = entries.len();
    Ok(Json(RegistryListResponse { entries, total }))
}

#[utoipa::path(
    get,
    path = "/registry/entries/{name}",
    params(
        ("name" = String, Path, description = "Entry name to look up"),
        ("kind" = Option<String>, Query, description = "Filter by kind: tool, skill, agent, recipe")
    ),
    responses(
        (status = 200, description = "Registry entry details", body = RegistryEntry),
        (status = 404, description = "Entry not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Registry"
)]
pub async fn get_registry_entry(
    axum::extract::Path(name): axum::extract::Path<String>,
    Query(params): Query<RegistrySearchParams>,
) -> Result<Json<RegistryEntry>, ErrorResponse> {
    let manager = default_manager();
    let kind = parse_kind(&params.kind);
    let entry = manager
        .get(&name, kind)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Registry lookup failed: {e}")))?
        .ok_or_else(|| ErrorResponse::not_found(format!("Registry entry '{name}' not found")))?;
    Ok(Json(entry))
}

#[utoipa::path(
    get,
    path = "/registry/sources",
    responses(
        (status = 200, description = "List of configured registry sources", body = RegistrySourcesResponse)
    ),
    tag = "Registry"
)]
pub async fn list_sources() -> Json<RegistrySourcesResponse> {
    let manager = default_manager();
    Json(RegistrySourcesResponse {
        sources: manager.source_names(),
    })
}

pub fn routes() -> Router {
    Router::new()
        .route("/registry/search", get(search_registry))
        .route("/registry/entries", get(list_registry))
        .route("/registry/entries/{name}", get(get_registry_entry))
        .route("/registry/sources", get(list_sources))
}
