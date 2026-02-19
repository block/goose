use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use goose::agents::{
    intent_router::IntentRouter,
    routing_eval::{
        compute_metrics, evaluate, load_eval_set, RoutingEvalMetrics, RoutingEvalResult,
    },
};
use goose::session::eval_storage::{
    CreateDatasetRequest, EvalDataset, EvalDatasetSummary, EvalOverview, EvalRunDetail,
    EvalRunSummary, EvalStorage, RunComparison, RunEvalRequest, TopicAnalytics,
};
use goose::session::tool_analytics::{
    AgentPerformanceMetrics, LiveMetrics, ResponseQualityMetrics, ToolAnalytics, ToolAnalyticsStore,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::routes::errors::ErrorResponse;
use crate::state::AppState;

// ── Routing Inspector types ────────────────────────────────────────

#[derive(Deserialize, ToSchema)]
struct InspectRequest {
    message: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct InspectResponse {
    decision: RoutingDecisionView,
    all_scores: Vec<AgentScoreView>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct RoutingDecisionView {
    agent_name: String,
    mode_slug: String,
    confidence: f32,
    reasoning: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct AgentScoreView {
    agent_name: String,
    enabled: bool,
    modes: Vec<ModeScoreView>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct ModeScoreView {
    slug: String,
    name: String,
    score: f32,
    matched_keywords: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
struct EvalRequest {
    yaml: String,
}

#[derive(Serialize, ToSchema)]
struct EvalResponse {
    metrics: RoutingEvalMetrics,
    results: Vec<RoutingEvalResult>,
    report: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CatalogResponse {
    agents: Vec<CatalogAgent>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CatalogAgent {
    name: String,
    enabled: bool,
    modes: Vec<CatalogMode>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CatalogMode {
    slug: String,
    name: String,
    when_to_use: String,
}

#[derive(Deserialize)]
pub struct ListRunsQuery {
    dataset_id: Option<String>,
    limit: Option<i64>,
}

// ── Routing Inspector endpoints ────────────────────────────────────

async fn inspect_routing(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<InspectRequest>,
) -> Result<Json<InspectResponse>, ErrorResponse> {
    if req.message.trim().is_empty() {
        return Err(ErrorResponse::bad_request("message must not be empty"));
    }

    let router = IntentRouter::new();
    let decision = router.route(&req.message);

    let all_scores = router
        .slots()
        .iter()
        .map(|slot| {
            let modes: Vec<ModeScoreView> = slot
                .modes
                .iter()
                .map(|mode| {
                    let (score, matched) = router.score_mode_detail(&req.message, mode);
                    ModeScoreView {
                        slug: mode.slug.clone(),
                        name: mode.name.clone(),
                        score,
                        matched_keywords: matched,
                    }
                })
                .collect();
            AgentScoreView {
                agent_name: slot.name.clone(),
                enabled: slot.enabled,
                modes,
            }
        })
        .collect();

    Ok(Json(InspectResponse {
        decision: RoutingDecisionView {
            agent_name: decision.agent_name,
            mode_slug: decision.mode_slug,
            confidence: decision.confidence,
            reasoning: decision.reasoning,
        },
        all_scores,
    }))
}

async fn eval_routing(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<EvalRequest>,
) -> Result<Json<EvalResponse>, ErrorResponse> {
    let test_set = load_eval_set(&req.yaml)
        .map_err(|e| ErrorResponse::bad_request(format!("invalid eval YAML: {e}")))?;

    let router = IntentRouter::new();
    let results = evaluate(&router, &test_set);
    let metrics = compute_metrics(&results);
    let report = goose::agents::routing_eval::format_report(&metrics, &results);

    Ok(Json(EvalResponse {
        metrics,
        results,
        report,
    }))
}

async fn catalog(State(_state): State<Arc<AppState>>) -> Json<CatalogResponse> {
    let router = IntentRouter::new();

    let agents = router
        .slots()
        .iter()
        .map(|slot| CatalogAgent {
            name: slot.name.clone(),
            enabled: slot.enabled,
            modes: slot
                .modes
                .iter()
                .map(|m| CatalogMode {
                    slug: m.slug.clone(),
                    name: m.name.clone(),
                    when_to_use: m.when_to_use.clone().unwrap_or_default(),
                })
                .collect(),
        })
        .collect();

    Json(CatalogResponse { agents })
}

// ── Eval Dataset CRUD endpoints ────────────────────────────────────

#[utoipa::path(
    get,
    path = "/analytics/eval/datasets",
    responses((status = 200, body = Vec<EvalDatasetSummary>)),
    operation_id = "listEvalDatasets"
)]
pub async fn list_datasets(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<EvalDatasetSummary>>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    let datasets = store
        .list_datasets()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(datasets))
}

#[utoipa::path(
    get,
    path = "/analytics/eval/datasets/{id}",
    params(("id" = String, Path, description = "Dataset ID")),
    responses((status = 200, body = EvalDataset)),
    operation_id = "getEvalDataset"
)]
pub async fn get_dataset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<EvalDataset>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    let dataset = store
        .get_dataset(&id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(dataset))
}

#[utoipa::path(
    post,
    path = "/analytics/eval/datasets",
    request_body = CreateDatasetRequest,
    responses((status = 200, body = EvalDataset)),
    operation_id = "createEvalDataset"
)]
pub async fn create_dataset(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDatasetRequest>,
) -> Result<Json<EvalDataset>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    let dataset = store
        .create_dataset(req)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(dataset))
}

#[utoipa::path(
    put,
    path = "/analytics/eval/datasets/{id}",
    params(("id" = String, Path, description = "Dataset ID")),
    request_body = CreateDatasetRequest,
    responses((status = 200, body = EvalDataset)),
    operation_id = "updateEvalDataset"
)]
pub async fn update_dataset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateDatasetRequest>,
) -> Result<Json<EvalDataset>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    let dataset = store
        .update_dataset(&id, req)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(dataset))
}

#[utoipa::path(
    delete,
    path = "/analytics/eval/datasets/{id}",
    params(("id" = String, Path, description = "Dataset ID")),
    responses((status = 200)),
    operation_id = "deleteEvalDataset"
)]
pub async fn delete_dataset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    store
        .delete_dataset(&id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ── Eval Run endpoints ─────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/analytics/eval/run",
    request_body = RunEvalRequest,
    responses((status = 200, body = EvalRunDetail)),
    operation_id = "runEval"
)]
pub async fn run_eval(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RunEvalRequest>,
) -> Result<Json<EvalRunDetail>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    let run = store
        .run_eval(req)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(run))
}

#[utoipa::path(
    get,
    path = "/analytics/eval/runs",
    params(
        ("dataset_id" = Option<String>, Query, description = "Filter by dataset ID"),
        ("limit" = Option<i64>, Query, description = "Max results")
    ),
    responses((status = 200, body = Vec<EvalRunSummary>)),
    operation_id = "listEvalRuns"
)]
pub async fn list_runs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListRunsQuery>,
) -> Result<Json<Vec<EvalRunSummary>>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    let runs = store
        .list_runs(params.dataset_id.as_deref(), params.limit.unwrap_or(50))
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(runs))
}

#[utoipa::path(
    get,
    path = "/analytics/eval/runs/{id}",
    params(("id" = String, Path, description = "Run ID")),
    responses((status = 200, body = EvalRunDetail)),
    operation_id = "getEvalRun"
)]
pub async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<EvalRunDetail>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    let run = store
        .get_run_detail(&id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(run))
}

// ── Overview & Topics ──────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/analytics/eval/overview",
    responses((status = 200, body = EvalOverview)),
    operation_id = "getEvalOverview"
)]
pub async fn get_overview(
    State(state): State<Arc<AppState>>,
) -> Result<Json<EvalOverview>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    let overview = store
        .get_overview()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(overview))
}

#[utoipa::path(
    get,
    path = "/analytics/eval/topics",
    responses((status = 200, body = Vec<TopicAnalytics>)),
    operation_id = "getEvalTopics"
)]
pub async fn get_topics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<TopicAnalytics>>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    let topics = store
        .get_topic_analytics()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(topics))
}

// ── Tool Analytics Query Params ─────────────────────────────────────

#[derive(Deserialize, ToSchema)]
pub struct ToolAnalyticsQuery {
    /// Number of days to look back (default 30)
    days: Option<i32>,
}

// ── Comparison endpoint ──────────────────────────────────────────

#[derive(Deserialize, utoipa::IntoParams)]
pub struct CompareRunsQuery {
    pub baseline_id: String,
    pub candidate_id: String,
}

#[utoipa::path(
    get,
    path = "/analytics/eval/compare",
    params(CompareRunsQuery),
    responses((status = 200, body = RunComparison)),
    operation_id = "compareEvalRuns"
)]
pub async fn compare_runs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CompareRunsQuery>,
) -> Result<Json<RunComparison>, ErrorResponse> {
    let pool = state
        .session_manager()
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = EvalStorage::new(pool);
    let comparison = store
        .compare_runs(&params.baseline_id, &params.candidate_id)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(comparison))
}

// ── Tool Analytics endpoints ───────────────────────────────────────

#[utoipa::path(
    get,
    path = "/analytics/tools",
    params(("days" = Option<i32>, Query, description = "Days to look back")),
    responses((status = 200, body = ToolAnalytics)),
    operation_id = "getToolAnalytics"
)]
pub async fn get_tool_analytics(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ToolAnalyticsQuery>,
) -> Result<Json<ToolAnalytics>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = ToolAnalyticsStore::new(pool);
    let analytics = store
        .get_tool_analytics(params.days.unwrap_or(30))
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(analytics))
}

#[utoipa::path(
    get,
    path = "/analytics/tools/agents",
    params(("days" = Option<i32>, Query, description = "Days to look back")),
    responses((status = 200, body = AgentPerformanceMetrics)),
    operation_id = "getAgentPerformance"
)]
pub async fn get_agent_performance(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ToolAnalyticsQuery>,
) -> Result<Json<AgentPerformanceMetrics>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = ToolAnalyticsStore::new(pool);
    let metrics = match store.get_agent_performance(params.days.unwrap_or(30)).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Analytics agent performance query failed: {e}");
            AgentPerformanceMetrics::default()
        }
    };
    Ok(Json(metrics))
}

// ── Live Monitoring ────────────────────────────────────────────────

/// Get live monitoring metrics (recent activity snapshot)
#[utoipa::path(
    get,
    path = "/analytics/monitoring/live",
    responses(
        (status = 200, description = "Live monitoring metrics", body = LiveMetrics)
    )
)]
pub async fn get_live_monitoring(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LiveMetrics>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = ToolAnalyticsStore::new(pool);
    let metrics = store
        .get_live_metrics()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(metrics))
}

/// Get response quality metrics
#[utoipa::path(
    get,
    path = "/analytics/quality",
    params(("days" = Option<i32>, Query, description = "Number of days to analyze")),
    responses((status = 200, body = ResponseQualityMetrics))
)]
pub async fn get_response_quality(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ToolAnalyticsQuery>,
) -> Result<Json<ResponseQualityMetrics>, ErrorResponse> {
    let sm = state.session_manager();
    let pool = sm
        .storage()
        .pool()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    let store = ToolAnalyticsStore::new(pool);
    let metrics = match store.get_response_quality(params.days.unwrap_or(30)).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Analytics response quality query failed: {e}");
            ResponseQualityMetrics::default()
        }
    };
    Ok(Json(metrics))
}

// ── Router ─────────────────────────────────────────────────────────

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Routing inspector (existing)
        .route("/analytics/routing/inspect", post(inspect_routing))
        .route("/analytics/routing/eval", post(eval_routing))
        .route("/analytics/routing/catalog", get(catalog))
        // Eval datasets CRUD
        .route("/analytics/eval/datasets", get(list_datasets))
        .route("/analytics/eval/datasets", post(create_dataset))
        .route("/analytics/eval/datasets/{id}", get(get_dataset))
        .route("/analytics/eval/datasets/{id}", put(update_dataset))
        .route("/analytics/eval/datasets/{id}", delete(delete_dataset))
        // Eval runs
        .route("/analytics/eval/run", post(run_eval))
        .route("/analytics/eval/runs", get(list_runs))
        .route("/analytics/eval/runs/{id}", get(get_run))
        // Overview & topics
        .route("/analytics/eval/overview", get(get_overview))
        .route("/analytics/eval/topics", get(get_topics))
        // Comparison
        .route("/analytics/eval/compare", get(compare_runs))
        // Tool analytics
        .route("/analytics/tools", get(get_tool_analytics))
        .route("/analytics/tools/agents", get(get_agent_performance))
        // Live monitoring
        .route("/analytics/monitoring/live", get(get_live_monitoring))
        // Response quality
        .route("/analytics/quality", get(get_response_quality))
        .with_state(state)
}
