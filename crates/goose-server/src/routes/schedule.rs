use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{scheduler::ScheduledJob, state::AppState};

#[derive(Deserialize)]
struct CreateScheduleRequest {
    id: String,
    recipe_source: String,
    cron: String,
}

#[derive(Serialize)]
struct ListSchedulesResponse {
    jobs: Vec<ScheduledJob>,
}

#[axum::debug_handler]
async fn create_schedule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateScheduleRequest>,
) -> Result<Json<ScheduledJob>, StatusCode> {
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let job = ScheduledJob {
        id: req.id,
        source: req.recipe_source,
        cron: req.cron,
        last_run: None,
    };
    scheduler
        .add(job.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(job))
}

#[axum::debug_handler]
async fn list_schedules(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ListSchedulesResponse>, StatusCode> {
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let jobs = scheduler.list().await;
    Ok(Json(ListSchedulesResponse { jobs }))
}

#[axum::debug_handler]
async fn delete_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    scheduler
        .remove(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/schedule/create", post(create_schedule))
        .route("/schedule/list", get(list_schedules))
        .route("/schedule/delete/{id}", delete(delete_schedule))
        .with_state(state)
}
