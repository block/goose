use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use chrono::NaiveDateTime;

use crate::routes::utils::verify_secret_key;
use crate::state::AppState;
use goose::scheduler::ScheduledJob;

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct CreateScheduleRequest {
    id: String,
    recipe_source: String,
    cron: String,
    #[serde(default)]
    execution_mode: Option<String>, // "foreground" or "background"
}

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct UpdateScheduleRequest {
    cron: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ListSchedulesResponse {
    jobs: Vec<ScheduledJob>,
}

// Response for the kill endpoint
#[derive(Serialize, utoipa::ToSchema)]
pub struct KillJobResponse {
    message: String,
}

// Response for the inspect endpoint
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InspectJobResponse {
    session_id: Option<String>,
    process_start_time: Option<String>,
    running_duration_seconds: Option<i64>,
}

// Response for the run_now endpoint
#[derive(Serialize, utoipa::ToSchema)]
pub struct RunNowResponse {
    session_id: String,
}

// Query parameters for the sessions endpoint
#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct SessionsQuery {
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    50 // Default limit for sessions listed
}

// Struct for the frontend session list
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionDisplayInfo {
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

#[utoipa::path(
    post,
    path = "/schedule/create",
    request_body = CreateScheduleRequest,
    responses(
        (status = 200, description = "Scheduled job created successfully", body = ScheduledJob),
        (status = 400, description = "Invalid cron expression or recipe file"),
        (status = 409, description = "Job ID already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "schedule"
)]
#[axum::debug_handler]
async fn create_schedule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateScheduleRequest>,
) -> Result<Json<ScheduledJob>, StatusCode> {
    verify_secret_key(&headers, &state)?;
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!(
        "Server: Calling scheduler.add_scheduled_job() for job '{}'",
        req.id
    );
    let job = ScheduledJob {
        id: req.id,
        source: req.recipe_source,
        cron: req.cron,
        last_run: None,
        currently_running: false,
        paused: false,
        current_session_id: None,
        process_start_time: None,
        execution_mode: req.execution_mode.or(Some("background".to_string())), // Default to background
    };
    scheduler
        .add_scheduled_job(job.clone())
        .await
        .map_err(|e| {
            eprintln!("Error creating schedule: {:?}", e); // Log error
            match e {
                goose::scheduler::SchedulerError::JobNotFound(_) => StatusCode::NOT_FOUND,
                goose::scheduler::SchedulerError::CronParseError(_) => StatusCode::BAD_REQUEST,
                goose::scheduler::SchedulerError::RecipeLoadError(_) => StatusCode::BAD_REQUEST,
                goose::scheduler::SchedulerError::JobIdExists(_) => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
    Ok(Json(job))
}

#[utoipa::path(
    get,
    path = "/schedule/list",
    responses(
        (status = 200, description = "A list of scheduled jobs", body = ListSchedulesResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "schedule"
)]
#[axum::debug_handler]
async fn list_schedules(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ListSchedulesResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!("Server: Calling scheduler.list_scheduled_jobs()");
    let jobs = scheduler.list_scheduled_jobs().await.map_err(|e| {
        eprintln!("Error listing schedules: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(ListSchedulesResponse { jobs }))
}

#[utoipa::path(
    delete,
    path = "/schedule/delete/{id}",
    params(
        ("id" = String, Path, description = "ID of the schedule to delete")
    ),
    responses(
        (status = 204, description = "Scheduled job deleted successfully"),
        (status = 404, description = "Scheduled job not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "schedule"
)]
#[axum::debug_handler]
async fn delete_schedule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    verify_secret_key(&headers, &state)?;
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    scheduler.remove_scheduled_job(&id).await.map_err(|e| {
        eprintln!("Error deleting schedule '{}': {:?}", id, e);
        match e {
            goose::scheduler::SchedulerError::JobNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/schedule/{id}/run_now",
    params(
        ("id" = String, Path, description = "ID of the schedule to run")
    ),
    responses(
        (status = 200, description = "Scheduled job triggered successfully, returns new session ID", body = RunNowResponse),
        (status = 404, description = "Scheduled job not found"),
        (status = 500, description = "Internal server error when trying to run the job")
    ),
    tag = "schedule"
)]
#[axum::debug_handler]
async fn run_now_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<RunNowResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!("Server: Calling scheduler.run_now() for job '{}'", id);

    match scheduler.run_now(&id).await {
        Ok(session_id) => Ok(Json(RunNowResponse { session_id })),
        Err(e) => {
            eprintln!("Error running schedule '{}' now: {:?}", id, e);
            match e {
                goose::scheduler::SchedulerError::JobNotFound(_) => Err(StatusCode::NOT_FOUND),
                goose::scheduler::SchedulerError::AnyhowError(ref err) => {
                    // Check if this is a cancellation error
                    if err.to_string().contains("was successfully cancelled") {
                        // Return a special session_id to indicate cancellation
                        Ok(Json(RunNowResponse {
                            session_id: "CANCELLED".to_string(),
                        }))
                    } else {
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

#[utoipa::path(
    get,
    path = "/schedule/{id}/sessions",
    params(
        ("id" = String, Path, description = "ID of the schedule"),
        SessionsQuery // This will automatically pick up 'limit' as a query parameter
    ),
    responses(
        (status = 200, description = "A list of session display info", body = Vec<SessionDisplayInfo>),
        (status = 500, description = "Internal server error")
    ),
    tag = "schedule"
)]
#[axum::debug_handler]
async fn sessions_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,                    // Added this line
    Path(schedule_id_param): Path<String>, // Renamed to avoid confusion with session_id
    Query(query_params): Query<SessionsQuery>,
) -> Result<Json<Vec<SessionDisplayInfo>>, StatusCode> {
    verify_secret_key(&headers, &state)?; // Added this line
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

#[utoipa::path(
    post,
    path = "/schedule/{id}/pause",
    params(
        ("id" = String, Path, description = "ID of the schedule to pause")
    ),
    responses(
        (status = 204, description = "Scheduled job paused successfully"),
        (status = 404, description = "Scheduled job not found"),
        (status = 400, description = "Cannot pause a currently running job"),
        (status = 500, description = "Internal server error")
    ),
    tag = "schedule"
)]
#[axum::debug_handler]
async fn pause_schedule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    verify_secret_key(&headers, &state)?;
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    scheduler.pause_schedule(&id).await.map_err(|e| {
        eprintln!("Error pausing schedule '{}': {:?}", id, e);
        match e {
            goose::scheduler::SchedulerError::JobNotFound(_) => StatusCode::NOT_FOUND,
            goose::scheduler::SchedulerError::AnyhowError(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/schedule/{id}/unpause",
    params(
        ("id" = String, Path, description = "ID of the schedule to unpause")
    ),
    responses(
        (status = 204, description = "Scheduled job unpaused successfully"),
        (status = 404, description = "Scheduled job not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "schedule"
)]
#[axum::debug_handler]
async fn unpause_schedule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    verify_secret_key(&headers, &state)?;
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    scheduler.unpause_schedule(&id).await.map_err(|e| {
        eprintln!("Error unpausing schedule '{}': {:?}", id, e);
        match e {
            goose::scheduler::SchedulerError::JobNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/schedule/{id}",
    params(
        ("id" = String, Path, description = "ID of the schedule to update")
    ),
    request_body = UpdateScheduleRequest,
    responses(
        (status = 200, description = "Scheduled job updated successfully", body = ScheduledJob),
        (status = 404, description = "Scheduled job not found"),
        (status = 400, description = "Cannot update a currently running job or invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "schedule"
)]
#[axum::debug_handler]
async fn update_schedule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateScheduleRequest>,
) -> Result<Json<ScheduledJob>, StatusCode> {
    verify_secret_key(&headers, &state)?;
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    scheduler
        .update_schedule(&id, req.cron)
        .await
        .map_err(|e| {
            eprintln!("Error updating schedule '{}': {:?}", id, e);
            match e {
                goose::scheduler::SchedulerError::JobNotFound(_) => StatusCode::NOT_FOUND,
                goose::scheduler::SchedulerError::AnyhowError(_) => StatusCode::BAD_REQUEST,
                goose::scheduler::SchedulerError::CronParseError(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    // Return the updated schedule
    let jobs = scheduler.list_scheduled_jobs().await.map_err(|e| {
        eprintln!("Error listing schedules after update: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let updated_job = jobs
        .into_iter()
        .find(|job| job.id == id)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(updated_job))
}

#[utoipa::path(
    post,
    path = "/schedule/{id}/kill",
    responses(
        (status = 200, description = "Running job killed successfully"),
    ),
    tag = "schedule"
)]
#[axum::debug_handler]
pub async fn kill_running_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<KillJobResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    scheduler.kill_running_job(&id).await.map_err(|e| {
        eprintln!("Error killing running job '{}': {:?}", id, e);
        match e {
            goose::scheduler::SchedulerError::JobNotFound(_) => StatusCode::NOT_FOUND,
            goose::scheduler::SchedulerError::AnyhowError(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;

    Ok(Json(KillJobResponse {
        message: format!("Successfully killed running job '{}'", id),
    }))
}

#[utoipa::path(
    get,
    path = "/schedule/{id}/inspect",
    params(
        ("id" = String, Path, description = "ID of the schedule to inspect")
    ),
    responses(
        (status = 200, description = "Running job information", body = InspectJobResponse),
        (status = 404, description = "Scheduled job not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "schedule"
)]
#[axum::debug_handler]
pub async fn inspect_running_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<InspectJobResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;
    let scheduler = state
        .scheduler()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match scheduler.get_running_job_info(&id).await {
        Ok(info) => {
            if let Some((session_id, start_time)) = info {
                let duration = chrono::Utc::now().signed_duration_since(start_time);
                Ok(Json(InspectJobResponse {
                    session_id: Some(session_id),
                    process_start_time: Some(start_time.to_rfc3339()),
                    running_duration_seconds: Some(duration.num_seconds()),
                }))
            } else {
                Ok(Json(InspectJobResponse {
                    session_id: None,
                    process_start_time: None,
                    running_duration_seconds: None,
                }))
            }
        }
        Err(e) => {
            eprintln!("Error inspecting running job '{}': {:?}", id, e);
            match e {
                goose::scheduler::SchedulerError::JobNotFound(_) => Err(StatusCode::NOT_FOUND),
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/schedule/create", post(create_schedule))
        .route("/schedule/list", get(list_schedules))
        .route("/schedule/delete/{id}", delete(delete_schedule)) // Corrected
        .route("/schedule/{id}", put(update_schedule))
        .route("/schedule/{id}/run_now", post(run_now_handler)) // Corrected
        .route("/schedule/{id}/pause", post(pause_schedule))
        .route("/schedule/{id}/unpause", post(unpause_schedule))
        .route("/schedule/{id}/kill", post(kill_running_job))
        .route("/schedule/{id}/inspect", get(inspect_running_job))
        .route("/schedule/{id}/sessions", get(sessions_handler)) // Corrected
        .with_state(state)
}
