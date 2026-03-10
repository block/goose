use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::Query;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use goose::agents::platform_extensions::summon::{
    discover_filesystem_sources, Source, SourceKind,
};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub enum SkillScope {
    Project,
    Global,
    Builtin,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub scope: SkillScope,
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListSkillsResponse {
    pub skills: Vec<SkillInfo>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListSkillsQuery {
    pub working_dir: Option<String>,
}

fn infer_scope(source: &Source, working_dir: &str) -> SkillScope {
    if source.kind == SourceKind::BuiltinSkill {
        return SkillScope::Builtin;
    }
    if source.path.starts_with(working_dir) {
        return SkillScope::Project;
    }
    SkillScope::Global
}

#[utoipa::path(
    get,
    path = "/skills/list",
    params(
        ("working_dir" = Option<String>, Query, description = "Working directory to discover skills from")
    ),
    responses(
        (status = 200, description = "Skills listed successfully", body = ListSkillsResponse),
    ),
    tag = "Skills"
)]
pub async fn list_skills(
    Query(query): Query<ListSkillsQuery>,
) -> Json<ListSkillsResponse> {
    let working_dir = query
        .working_dir
        .unwrap_or_else(|| ".".to_string());
    let working_path = PathBuf::from(&working_dir);

    let sources = discover_filesystem_sources(&working_path);

    let skills: Vec<SkillInfo> = sources
        .into_iter()
        .filter(|s| s.kind == SourceKind::Skill || s.kind == SourceKind::BuiltinSkill)
        .map(|s| {
            let scope = infer_scope(&s, &working_dir);
            SkillInfo {
                name: s.name,
                description: s.description,
                scope,
                path: s.path.to_string_lossy().to_string(),
                content: s.content,
            }
        })
        .collect();

    Json(ListSkillsResponse { skills })
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/skills/list", get(list_skills))
        .with_state(state)
}
