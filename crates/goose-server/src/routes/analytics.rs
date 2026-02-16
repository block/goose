use axum::{extract::State, routing::get, routing::post, Json, Router};
use goose::agents::intent_router::IntentRouter;
use goose::agents::routing_eval::{compute_metrics, evaluate, load_eval_set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::routes::errors::ErrorResponse;
use crate::state::AppState;

// ── Request / Response types ───────────────────────────────────────

#[derive(Deserialize)]
pub struct InspectRequest {
    pub message: String,
}

#[derive(Serialize)]
pub struct InspectResponse {
    pub decision: RoutingDecisionView,
    pub all_scores: Vec<AgentScoreView>,
}

#[derive(Serialize)]
pub struct RoutingDecisionView {
    pub agent_name: String,
    pub mode_slug: String,
    pub confidence: f32,
    pub reasoning: String,
}

#[derive(Serialize)]
pub struct AgentScoreView {
    pub agent_name: String,
    pub enabled: bool,
    pub modes: Vec<ModeScoreView>,
}

#[derive(Serialize)]
pub struct ModeScoreView {
    pub slug: String,
    pub name: String,
    pub score: f32,
    pub matched_keywords: Vec<String>,
}

#[derive(Deserialize)]
pub struct EvalRequest {
    pub yaml: String,
}

#[derive(Serialize)]
pub struct EvalResponse {
    pub metrics: goose::agents::routing_eval::RoutingEvalMetrics,
    pub results: Vec<goose::agents::routing_eval::RoutingEvalResult>,
    pub report: String,
}

#[derive(Serialize)]
pub struct CatalogResponse {
    pub agents: Vec<CatalogAgent>,
}

#[derive(Serialize)]
pub struct CatalogAgent {
    pub name: String,
    pub enabled: bool,
    pub modes: Vec<CatalogMode>,
}

#[derive(Serialize)]
pub struct CatalogMode {
    pub slug: String,
    pub name: String,
    pub when_to_use: Option<String>,
}

// ── Handlers ───────────────────────────────────────────────────────

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
                    when_to_use: m.when_to_use.clone(),
                })
                .collect(),
        })
        .collect();

    Json(CatalogResponse { agents })
}

// ── Router ─────────────────────────────────────────────────────────

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/analytics/routing/inspect", post(inspect_routing))
        .route("/analytics/routing/eval", post(eval_routing))
        .route("/analytics/routing/catalog", get(catalog))
        .with_state(state)
}
