use std::sync::Arc;

use axum::{
    extract::{Path, Query, State}, // Added Query
    http::StatusCode,
    routing::{delete, get, post},
    Json,
    Router,
};
use serde::{Deserialize, Serialize};

// Added for parsing session_name to created_at
use chrono::NaiveDateTime;

use crate::state::AppState;
use goose::scheduler::ScheduledJob;

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

// Struct for the frontend session list
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionDisplayInfo {
    id: String,          // Derived from session_name (filename)
    name: String,        // From metadata.description
    created_at: String,  // Derived from session_name, in ISO 8601 format
    working_dir: String, // from metadata.working_dir (as String)
    schedule_id: Option<String>,
    message_count: usize,
    total_tokens: Option<i32>,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    accumulated_total_tokens: Option<i32>,
    accumulated_input_tokens: Option<i32>,
    accumulated_output_tokens: Option<i32>,
}

fn parse_session_name_to_iso(session_name: &str) -> String {
    NaiveDateTime::parse_from_str(session_name, "%Y%m%d_%H%M%S")
        .map(|dt| dt.and_utc().to_rfc3339())
        .unwrap_or_else(|_| String::new()) // Fallback to empty string if parsing fails
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
    Path(schedule_id_param): Path<String>, // Renamed to avoid confusion with session_id
    Query(query_params): Query<SessionsQuery>,
) -> Result<Json<Vec<SessionDisplayInfo>>, StatusCode> {
    // Changed return type
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match scheduler
        .sessions(&schedule_id_param, query_params.limit as usize)
        .await
    {
        Ok(session_tuples) => {
            // Expecting Vec<(String, goose::session::storage::SessionMetadata)>
            let display_infos: Vec<SessionDisplayInfo> = session_tuples
                .into_iter()
                .map(|(session_name, metadata)| SessionDisplayInfo {
                    id: session_name.clone(),
                    name: metadata.description, // Use description as name
                    created_at: parse_session_name_to_iso(&session_name),
                    working_dir: metadata.working_dir.to_string_lossy().into_owned(),
                    schedule_id: metadata.schedule_id, // This is the ID of the schedule itself
                    message_count: metadata.message_count,
                    total_tokens: metadata.total_tokens,
                    input_tokens: metadata.input_tokens,
                    output_tokens: metadata.output_tokens,
                    accumulated_total_tokens: metadata.accumulated_total_tokens,
                    accumulated_input_tokens: metadata.accumulated_input_tokens,
                    accumulated_output_tokens: metadata.accumulated_output_tokens,
                })
                .collect();
            Ok(Json(display_infos))
        }
        Err(e) => {
            eprintln!(
                "Error fetching sessions for schedule '{}': {:?}",
                schedule_id_param, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/schedule/create", post(create_schedule))
        .route("/schedule/list", get(list_schedules))
        .route("/schedule/delete/{id}", delete(delete_schedule)) // Corrected
        .route("/schedule/{id}/run_now", post(run_now_handler)) // Corrected
        .route("/schedule/{id}/sessions", get(sessions_handler)) // Corrected
        .with_state(state)
}
