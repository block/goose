use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use utoipa::IntoParams;

use goose::eval::eval_storage::{CreateDatasetRequest, EvalStorage, RunEvalRequest};
use goose::eval::tool_analytics::ToolAnalyticsStore;

use crate::state::AppState;

// ── Query params ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, IntoParams, utoipa::ToSchema)]
pub struct ListRunsQuery {
    pub dataset_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize, IntoParams, utoipa::ToSchema)]
pub struct CompareRunsQuery {
    pub baseline: String,
    pub candidate: String,
}

#[derive(Debug, Deserialize, IntoParams, utoipa::ToSchema)]
pub struct ToolAnalyticsQuery {
    #[serde(default = "default_days")]
    pub days: i32,
}

fn default_days() -> i32 {
    30
}

// ── Dataset CRUD ───────────────────────────────────────────────────

#[utoipa::path(get, path = "/analytics/datasets",
    responses((status = 200, description = "List eval datasets")),
    tag = "analytics"
)]
pub async fn list_datasets(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let datasets = store
        .list_datasets()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::to_value(datasets).unwrap()))
}

#[utoipa::path(get, path = "/analytics/datasets/{id}",
    params(("id" = String, Path, description = "Dataset ID")),
    responses((status = 200, description = "Get dataset")),
    tag = "analytics"
)]
pub async fn get_dataset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let dataset = store
        .get_dataset(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(serde_json::to_value(dataset).unwrap()))
}

#[utoipa::path(post, path = "/analytics/datasets",
    request_body = CreateDatasetRequest,
    responses((status = 200, description = "Create dataset")),
    tag = "analytics"
)]
pub async fn create_dataset(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDatasetRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let dataset = store
        .create_dataset(req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::to_value(dataset).unwrap()))
}

#[utoipa::path(put, path = "/analytics/datasets/{id}",
    params(("id" = String, Path, description = "Dataset ID")),
    request_body = CreateDatasetRequest,
    responses((status = 200, description = "Update dataset")),
    tag = "analytics"
)]
pub async fn update_dataset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateDatasetRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let dataset = store
        .update_dataset(&id, req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::to_value(dataset).unwrap()))
}

#[utoipa::path(delete, path = "/analytics/datasets/{id}",
    params(("id" = String, Path, description = "Dataset ID")),
    responses((status = 204, description = "Dataset deleted")),
    tag = "analytics"
)]
pub async fn delete_dataset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    store
        .delete_dataset(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Eval runs ──────────────────────────────────────────────────────

#[utoipa::path(get, path = "/analytics/runs",
    params(ListRunsQuery),
    responses((status = 200, description = "List eval runs")),
    tag = "analytics"
)]
pub async fn list_runs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListRunsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let runs = store
        .list_runs(q.dataset_id.as_deref(), q.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::to_value(runs).unwrap()))
}

#[utoipa::path(get, path = "/analytics/runs/{id}",
    params(("id" = String, Path, description = "Run ID")),
    responses((status = 200, description = "Get run detail")),
    tag = "analytics"
)]
pub async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let run = store
        .get_run_detail(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(serde_json::to_value(run).unwrap()))
}

// ── Store eval run (from external runner) ──────────────────────────

/// Eval case result for API requests
#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvalCaseResult {
    pub input: String,
    pub expected_agent: String,
    pub actual_agent: String,
    #[serde(default)]
    pub expected_mode: Option<String>,
    #[serde(default)]
    pub actual_mode: Option<String>,
    #[serde(default)]
    pub confidence: f64,
    pub correct_agent: bool,
    #[serde(default)]
    pub correct_mode: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl From<EvalCaseResult> for goose::eval::eval_storage::EvalCaseResult {
    fn from(r: EvalCaseResult) -> Self {
        let agent_correct = r.correct_agent;
        let mode_correct = r.correct_mode;
        Self {
            input: r.input,
            expected_agent: r.expected_agent,
            actual_agent: r.actual_agent,
            expected_mode: r.expected_mode.unwrap_or_default(),
            actual_mode: r.actual_mode.unwrap_or_default(),
            confidence: r.confidence as f32,
            agent_correct,
            mode_correct,
            fully_correct: agent_correct && mode_correct,
        }
    }
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StoreRunRequest {
    pub eval_request: RunEvalRequest,
    pub results: Vec<EvalCaseResult>,
    #[serde(default)]
    pub duration_ms: i64,
}

#[utoipa::path(post, path = "/analytics/runs",
    request_body = StoreRunRequest,
    responses((status = 200, description = "Store eval run results")),
    tag = "analytics"
)]
pub async fn store_run(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StoreRunRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let storage_results: Vec<goose::eval::eval_storage::EvalCaseResult> =
        req.results.into_iter().map(Into::into).collect();
    let run = store
        .store_eval_run(&req.eval_request, &storage_results, req.duration_ms)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::to_value(run).unwrap()))
}

// ── Analytics overview ─────────────────────────────────────────────

#[utoipa::path(get, path = "/analytics/overview",
    responses((status = 200, description = "Eval overview")),
    tag = "analytics"
)]
pub async fn get_overview(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let overview = store
        .get_overview()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::to_value(overview).unwrap()))
}

#[utoipa::path(get, path = "/analytics/topics",
    responses((status = 200, description = "Topic analytics")),
    tag = "analytics"
)]
pub async fn get_topics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let topics = store
        .get_topic_analytics()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::to_value(topics).unwrap()))
}

#[utoipa::path(get, path = "/analytics/compare",
    params(CompareRunsQuery),
    responses((status = 200, description = "Compare two runs")),
    tag = "analytics"
)]
pub async fn compare_runs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CompareRunsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = EvalStorage::new(pool);
    store
        .ensure_tables()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let comparison = store
        .compare_runs(&q.baseline, &q.candidate)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::to_value(comparison).unwrap()))
}

// ── Tool analytics ─────────────────────────────────────────────────

#[utoipa::path(get, path = "/analytics/tools",
    params(ToolAnalyticsQuery),
    responses((status = 200, description = "Tool usage analytics")),
    tag = "analytics"
)]
pub async fn get_tool_analytics(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ToolAnalyticsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let store = ToolAnalyticsStore::new(pool);
    let analytics = store
        .get_tool_analytics(q.days)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::to_value(analytics).unwrap()))
}

// ── Router ─────────────────────────────────────────────────────────

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Dataset CRUD
        .route(
            "/analytics/datasets",
            get(list_datasets).post(create_dataset),
        )
        .route(
            "/analytics/datasets/{id}",
            get(get_dataset).put(update_dataset).delete(delete_dataset),
        )
        // Eval runs
        .route("/analytics/runs", get(list_runs).post(store_run))
        .route("/analytics/runs/{id}", get(get_run))
        // Analytics
        .route("/analytics/overview", get(get_overview))
        .route("/analytics/topics", get(get_topics))
        .route("/analytics/compare", get(compare_runs))
        // Tool analytics
        .route("/analytics/tools", get(get_tool_analytics))
        .with_state(state)
}
