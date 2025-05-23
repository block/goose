use std::sync::Arc;

use axum::{
    extract::{Path, Query, State}, // Added Query
    http::StatusCode,
    routing::{delete, get, post},
    Json,
    Router,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use goose::scheduler::ScheduledJob;
use goose::session::storage::SessionMetadata; // For SessionMeta type

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

// Response for the run_now endpoint
#[derive(Serialize)]
struct RunNowResponse {
    session_id: String,
}

// Query parameters for the sessions endpoint
#[derive(Deserialize)]
struct SessionsQuery {
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    50 // Default limit for sessions listed
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
        .add_scheduled_job(job.clone())
        .await
        .map_err(|e| {
            eprintln!("Error creating schedule: {:?}", e); // Log error
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
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
    let jobs = scheduler.list_scheduled_jobs().await;
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
    scheduler.remove_scheduled_job(&id).await.map_err(|e| {
        eprintln!("Error deleting schedule '{}': {:?}", id, e); // Log error
        match e {
            goose::scheduler::SchedulerError::JobNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
async fn run_now_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<RunNowResponse>, StatusCode> {
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match scheduler.run_now(&id).await {
        Ok(session_id) => Ok(Json(RunNowResponse { session_id })),
        Err(e) => {
            eprintln!("Error running schedule '{}' now: {:?}", id, e); // Log error
            match e {
                goose::scheduler::SchedulerError::JobNotFound(_) => Err(StatusCode::NOT_FOUND),
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

#[axum::debug_handler]
async fn sessions_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query_params): Query<SessionsQuery>,
) -> Result<Json<Vec<SessionMetadata>>, StatusCode> {
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match scheduler.sessions(&id, query_params.limit as usize).await {
        Ok(sessions) => Ok(Json(sessions)),
        Err(e) => {
            eprintln!("Error fetching sessions for schedule '{}': {:?}", id, e); // Log error
            // Assuming JobNotFound isn't directly applicable here, as sessions can be empty for a valid job.
            // Other errors from scheduler.sessions might be SchedulerError::StorageError
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/schedule/create", post(create_schedule))
        .route("/schedule/list", get(list_schedules))
        .route("/schedule/delete/{id}", delete(delete_schedule)) // Corrected
        .route("/schedule/{id}/run_now", post(run_now_handler))    // Corrected
        .route("/schedule/{id}/sessions", get(sessions_handler))  // Corrected
        .with_state(state)
}
