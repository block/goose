//! ACP-compatible /runs endpoints — full Agent.reply() integration.
//!
//! Implements the Agent Communication Protocol v0.2.0 run lifecycle:
//! - POST /runs — create a new run (sync, async, or streaming)
//! - GET /runs/{run_id} — get run status
//! - POST /runs/{run_id} — resume an awaiting run
//! - POST /runs/{run_id}/cancel — cancel a running run
//! - GET /runs/{run_id}/events — list stored events for a run
//! - GET /runs — list all runs

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json};
use chrono::Utc;
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use goose::acp_compat::events::{AcpEvent, AcpEventContext};
use goose::acp_compat::message::{acp_message_to_goose, goose_message_to_acp, AcpMessage};
use goose::acp_compat::types::{
    AcpError, AcpRun, AcpRunStatus, AwaitRequest, RunCreateRequest, RunMode, RunResumeRequest,
};
use goose::action_required_manager::ActionRequiredManager;
use goose::agents::{AgentEvent, SessionConfig};
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};

use crate::routes::acp_discovery::resolve_mode_to_agent;
use crate::state::AppState;

// ── RunStore ─────────────────────────────────────────────────────────

const MAX_COMPLETED_RUNS: usize = 1000;

/// Tracks the pending action that put a run into Awaiting state.
#[derive(Debug, Clone)]
pub enum AwaitMetadata {
    Elicitation {
        request_id: String,
    },
    ToolConfirmation {
        request_id: String,
        session_id: String,
    },
}

/// All mutable state for a single lock acquisition.
#[derive(Debug, Default)]
struct RunStoreInner {
    runs: HashMap<String, AcpRun>,
    events: HashMap<String, Vec<AcpEvent>>,
    cancel_tokens: HashMap<String, CancellationToken>,
    await_metadata: HashMap<String, AwaitMetadata>,
}

/// In-memory run store with event persistence, cancellation tokens, and eviction.
#[derive(Debug, Default, Clone)]
pub struct RunStore {
    inner: Arc<Mutex<RunStoreInner>>,
}

impl RunStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn create(&self, run: AcpRun, cancel_token: CancellationToken) {
        let mut inner = self.inner.lock().await;
        let run_id = run.run_id.clone();
        inner.runs.insert(run_id.clone(), run);
        inner.events.insert(run_id.clone(), Vec::new());
        inner.cancel_tokens.insert(run_id, cancel_token);
        Self::evict_completed(&mut inner);
    }

    pub async fn get(&self, run_id: &str) -> Option<AcpRun> {
        self.inner.lock().await.runs.get(run_id).cloned()
    }

    pub async fn get_status(&self, run_id: &str) -> Option<AcpRunStatus> {
        self.inner
            .lock()
            .await
            .runs
            .get(run_id)
            .map(|r| r.status.clone())
    }

    pub async fn update_status(&self, run_id: &str, status: AcpRunStatus) {
        let mut inner = self.inner.lock().await;
        if let Some(run) = inner.runs.get_mut(run_id) {
            run.status = status;
        }
    }

    pub async fn set_awaiting(
        &self,
        run_id: &str,
        await_request: AwaitRequest,
        metadata: AwaitMetadata,
    ) {
        let mut inner = self.inner.lock().await;
        if let Some(run) = inner.runs.get_mut(run_id) {
            run.status = AcpRunStatus::Awaiting;
            run.await_request = Some(await_request);
        }
        inner.await_metadata.insert(run_id.to_string(), metadata);
    }

    /// Atomically check that a run is Awaiting and take its metadata.
    /// Returns `None` if the run doesn't exist, isn't Awaiting, or has no metadata.
    pub async fn take_await_if_awaiting(&self, run_id: &str) -> Option<AwaitMetadata> {
        let mut inner = self.inner.lock().await;
        let is_awaiting = inner
            .runs
            .get(run_id)
            .is_some_and(|r| r.status == AcpRunStatus::Awaiting);
        if is_awaiting {
            inner.await_metadata.remove(run_id)
        } else {
            None
        }
    }

    pub async fn clear_await(&self, run_id: &str) {
        let mut inner = self.inner.lock().await;
        if let Some(run) = inner.runs.get_mut(run_id) {
            run.await_request = None;
        }
    }

    pub async fn finish(&self, run_id: &str, status: AcpRunStatus) {
        let mut inner = self.inner.lock().await;
        if let Some(run) = inner.runs.get_mut(run_id) {
            run.status = status;
            run.finished_at = Some(Utc::now());
        }
    }

    pub async fn set_error(&self, run_id: &str, error: AcpError) {
        let mut inner = self.inner.lock().await;
        if let Some(run) = inner.runs.get_mut(run_id) {
            run.error = Some(error);
        }
    }

    pub async fn append_output(&self, run_id: &str, message: AcpMessage) {
        let mut inner = self.inner.lock().await;
        if let Some(run) = inner.runs.get_mut(run_id) {
            run.output.push(message);
        }
    }

    pub async fn append_event(&self, run_id: &str, event: AcpEvent) {
        let mut inner = self.inner.lock().await;
        if let Some(events) = inner.events.get_mut(run_id) {
            events.push(event);
        }
    }

    pub async fn get_events(&self, run_id: &str) -> Option<Vec<AcpEvent>> {
        self.inner.lock().await.events.get(run_id).cloned()
    }

    pub async fn cancel(&self, run_id: &str) -> bool {
        let inner = self.inner.lock().await;
        if let Some(token) = inner.cancel_tokens.get(run_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    pub async fn list(&self, limit: usize, offset: usize) -> Vec<AcpRun> {
        let inner = self.inner.lock().await;
        inner
            .runs
            .values()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect()
    }

    fn evict_completed(inner: &mut RunStoreInner) {
        let completed: Vec<String> = inner
            .runs
            .iter()
            .filter(|(_, r)| {
                matches!(
                    r.status,
                    AcpRunStatus::Completed | AcpRunStatus::Failed | AcpRunStatus::Cancelled
                )
            })
            .map(|(id, _)| id.clone())
            .collect();

        if completed.len() <= MAX_COMPLETED_RUNS {
            return;
        }

        let mut to_evict: Vec<(String, Option<chrono::DateTime<Utc>>)> = completed
            .into_iter()
            .map(|id| {
                let finished = inner.runs.get(&id).and_then(|r| r.finished_at);
                (id, finished)
            })
            .collect();
        to_evict.sort_by_key(|(_, t)| *t);

        let evict_count = to_evict.len() - MAX_COMPLETED_RUNS;
        for (id, _) in to_evict.into_iter().take(evict_count) {
            inner.runs.remove(&id);
            inner.events.remove(&id);
            inner.cancel_tokens.remove(&id);
            inner.await_metadata.remove(&id);
        }
    }
}

fn generate_run_id() -> String {
    format!("run_{}", uuid::Uuid::new_v4().as_hyphenated())
}

// ── Routes ───────────────────────────────────────────────────────────

pub fn routes(state: Arc<crate::state::AppState>) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/runs", post(create_run).get(list_runs))
        .route("/runs/{run_id}", get(get_run).post(resume_run))
        .route("/runs/{run_id}/cancel", post(cancel_run))
        .route("/runs/{run_id}/events", get(get_run_events))
        .with_state((*state).clone())
}

// ── POST /runs ───────────────────────────────────────────────────────

#[utoipa::path(post, path = "/runs",
    tag = "ACP Runs",
    request_body = RunCreateRequest,
    responses(
        (status = 200, description = "Run created (stream/sync)", body = AcpRun),
        (status = 202, description = "Run created (async)", body = AcpRun),
    )
)]
pub async fn create_run(
    State(state): State<AppState>,
    Json(req): Json<RunCreateRequest>,
) -> impl IntoResponse {
    let run_id = generate_run_id();
    let cancel_token = CancellationToken::new();

    let session_id = req
        .session_id
        .clone()
        .unwrap_or_else(|| format!("acp-{}", uuid::Uuid::new_v4().as_hyphenated()));

    let run = AcpRun {
        run_id: run_id.clone(),
        agent_name: req.agent_name.clone(),
        status: AcpRunStatus::Created,
        session_id: Some(session_id.clone()),
        output: Vec::new(),
        await_request: None,
        error: None,
        created_at: Utc::now(),
        finished_at: None,
        metadata: None,
    };

    let store = state.clone().run_store().clone();
    store.create(run.clone(), cancel_token.clone()).await;

    match req.mode {
        RunMode::Stream => {
            let stream = create_run_stream(state, run_id, session_id, req, cancel_token);
            Sse::new(stream)
                .keep_alive(KeepAlive::default())
                .into_response()
        }
        RunMode::Async => {
            tokio::spawn(process_run(
                state,
                run_id.clone(),
                session_id,
                req,
                cancel_token,
            ));
            Json(run).into_response()
        }
        RunMode::Sync => {
            process_run(state, run_id.clone(), session_id, req, cancel_token).await;
            match store.get(&run_id).await {
                Some(r) => Json(r).into_response(),
                None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }
}

// ── GET /runs/{run_id} ──────────────────────────────────────────────

#[utoipa::path(get, path = "/runs/{run_id}",
    tag = "ACP Runs",
    params(("run_id" = String, Path, description = "Run ID")),
    responses(
        (status = 200, description = "Run details", body = AcpRun),
        (status = 404, description = "Run not found"),
    )
)]
pub async fn get_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> impl IntoResponse {
    match state.run_store().get(&run_id).await {
        Some(run) => Json(run).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "run not found"})),
        )
            .into_response(),
    }
}

// ── POST /runs/{run_id} (resume) ────────────────────────────────────

#[utoipa::path(post, path = "/runs/{run_id}",
    tag = "ACP Runs",
    params(("run_id" = String, Path, description = "Run ID")),
    request_body = RunResumeRequest,
    responses(
        (status = 200, description = "Run resumed", body = AcpRun),
        (status = 404, description = "Run not found"),
        (status = 409, description = "Run not in awaiting state"),
    )
)]
pub async fn resume_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    Json(req): Json<RunResumeRequest>,
) -> impl IntoResponse {
    let store = state.run_store();

    // Check existence first for a proper 404.
    let status = match store.get_status(&run_id).await {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "run not found"})),
            )
                .into_response()
        }
    };

    if status != AcpRunStatus::Awaiting {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "run is not in awaiting state",
                "current_status": status
            })),
        )
            .into_response();
    }

    // Atomically verify Awaiting status and take the metadata in one lock.
    let metadata = match store.take_await_if_awaiting(&run_id).await {
        Some(m) => m,
        None => {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "run is no longer in awaiting state (concurrent resume)"})),
            )
                .into_response()
        }
    };

    let resume_data = req.await_resume.data.unwrap_or(serde_json::Value::Null);

    let result = match metadata {
        AwaitMetadata::Elicitation { request_id } => {
            ActionRequiredManager::global()
                .submit_response(request_id, resume_data)
                .await
        }
        AwaitMetadata::ToolConfirmation {
            request_id,
            session_id,
        } => {
            let permission = parse_permission(&resume_data);
            let agent = match state.get_agent(session_id).await {
                Ok(a) => a,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": format!("Failed to get agent: {}", e)})),
                    )
                        .into_response()
                }
            };
            agent
                .handle_confirmation(
                    request_id,
                    PermissionConfirmation {
                        principal_type: PrincipalType::Tool,
                        permission,
                    },
                )
                .await;
            Ok(())
        }
    };

    match result {
        Ok(()) => {
            store.clear_await(&run_id).await;
            store.update_status(&run_id, AcpRunStatus::InProgress).await;

            match store.get(&run_id).await {
                Some(r) => {
                    let event = AcpEvent::run_in_progress(&r);
                    store.append_event(&run_id, event).await;
                    Json(r).into_response()
                }
                None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to submit resume: {}", e)})),
        )
            .into_response(),
    }
}

/// Parse an ACP resume data value into a Permission.
fn parse_permission(data: &serde_json::Value) -> Permission {
    match data.as_str() {
        Some("allow_once") | Some("AllowOnce") => Permission::AllowOnce,
        Some("always_allow") | Some("AlwaysAllow") => Permission::AlwaysAllow,
        Some("deny_once") | Some("DenyOnce") => Permission::DenyOnce,
        Some("always_deny") | Some("AlwaysDeny") => Permission::AlwaysDeny,
        Some("cancel") | Some("Cancel") => Permission::Cancel,
        _ => {
            if let Some(action) = data.get("action").and_then(|a| a.as_str()) {
                match action {
                    "allow_once" | "AllowOnce" => Permission::AllowOnce,
                    "always_allow" | "AlwaysAllow" => Permission::AlwaysAllow,
                    "deny_once" | "DenyOnce" => Permission::DenyOnce,
                    "always_deny" | "AlwaysDeny" => Permission::AlwaysDeny,
                    "cancel" | "Cancel" => Permission::Cancel,
                    _ => Permission::AllowOnce,
                }
            } else {
                Permission::AllowOnce
            }
        }
    }
}

// ── POST /runs/{run_id}/cancel ──────────────────────────────────────

#[utoipa::path(post, path = "/runs/{run_id}/cancel",
    tag = "ACP Runs",
    params(("run_id" = String, Path, description = "Run ID")),
    responses(
        (status = 200, description = "Run cancelled", body = AcpRun),
        (status = 404, description = "Run not found"),
    )
)]
pub async fn cancel_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> impl IntoResponse {
    let store = state.run_store();

    let run = match store.get(&run_id).await {
        Some(r) => r,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "run not found"})),
            )
                .into_response()
        }
    };

    match run.status {
        AcpRunStatus::InProgress | AcpRunStatus::Awaiting => {
            store.cancel(&run_id).await;
            store.finish(&run_id, AcpRunStatus::Cancelled).await;

            let cancelled = store.get(&run_id).await.unwrap();
            let event = AcpEvent::run_cancelled(&cancelled);
            store.append_event(&run_id, event).await;

            Json(cancelled).into_response()
        }
        _ => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "run cannot be cancelled in current state",
                "current_status": run.status
            })),
        )
            .into_response(),
    }
}

// ── GET /runs/{run_id}/events ───────────────────────────────────────

#[utoipa::path(get, path = "/runs/{run_id}/events",
    tag = "ACP Runs",
    params(("run_id" = String, Path, description = "Run ID")),
    responses(
        (status = 200, description = "Run events", body = Vec<serde_json::Value>),
        (status = 404, description = "Run not found"),
    )
)]
pub async fn get_run_events(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> impl IntoResponse {
    match state.run_store().get_events(&run_id).await {
        Some(events) => Json(serde_json::json!({ "events": events })).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "run not found"})),
        )
            .into_response(),
    }
}

// ── GET /runs ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListRunsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    100
}

#[utoipa::path(get, path = "/runs",
    tag = "ACP Runs",
    params(
        ("limit" = Option<usize>, Query, description = "Max results"),
        ("offset" = Option<usize>, Query, description = "Offset"),
    ),
    responses(
        (status = 200, description = "List of runs", body = Vec<AcpRun>),
    )
)]
pub async fn list_runs(
    State(state): State<AppState>,
    Query(query): Query<ListRunsQuery>,
) -> impl IntoResponse {
    let runs = state.run_store().list(query.limit, query.offset).await;
    Json(runs)
}

// ── Mode + Extension binding ────────────────────────────────────────

async fn apply_agent_bindings(state: &AppState, agent: &goose::agents::Agent, agent_name: &str) {
    if let Some((slot_name, mode_slug)) = resolve_mode_to_agent(agent_name) {
        // Apply bound extensions from the AgentSlotRegistry
        let bound = state
            .agent_slot_registry
            .get_bound_extensions(&slot_name)
            .await;
        if !bound.is_empty() {
            agent
                .set_allowed_extensions(bound.into_iter().collect())
                .await;
        }

        // Apply mode-specific tool_groups using OrchestratorAgent (same as reply.rs)
        let provider = std::sync::Arc::new(tokio::sync::Mutex::new(None));
        let orchestrator = goose::agents::orchestrator_agent::OrchestratorAgent::new(provider);
        let tool_groups = orchestrator.get_tool_groups_for_routing(&slot_name, &mode_slug);
        if !tool_groups.is_empty() {
            agent.set_active_tool_groups(tool_groups).await;
        }

        // Apply mode-recommended extensions
        let recommended =
            orchestrator.get_recommended_extensions_for_routing(&slot_name, &mode_slug);
        if !recommended.is_empty() {
            agent.set_allowed_extensions(recommended).await;
        }
    }
}

// ── Core: non-streaming (sync/async) path ───────────────────────────

async fn process_run(
    state: AppState,
    run_id: String,
    session_id: String,
    req: RunCreateRequest,
    cancel_token: CancellationToken,
) {
    let store = state.run_store();

    store.update_status(&run_id, AcpRunStatus::InProgress).await;

    let user_message = match build_user_message(&req) {
        Some(msg) => msg,
        None => {
            let error = AcpError {
                code: "invalid_input".to_string(),
                message: "No user message provided".to_string(),
                data: None,
            };
            store.set_error(&run_id, error).await;
            store.finish(&run_id, AcpRunStatus::Failed).await;
            return;
        }
    };

    let agent = match state.get_agent(session_id.clone()).await {
        Ok(a) => a,
        Err(e) => {
            let error = AcpError {
                code: "agent_error".to_string(),
                message: format!("Failed to get agent: {}", e),
                data: None,
            };
            store.set_error(&run_id, error).await;
            store.finish(&run_id, AcpRunStatus::Failed).await;
            return;
        }
    };

    apply_agent_bindings(&state, &agent, &req.agent_name).await;

    let session_config = SessionConfig {
        id: session_id.clone(),
        schedule_id: None,
        max_turns: None,
        retry_config: None,
    };

    let mut agent_stream = match agent
        .reply(user_message, session_config, Some(cancel_token))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            let error = AcpError {
                code: "reply_error".to_string(),
                message: e.to_string(),
                data: None,
            };
            store.set_error(&run_id, error).await;
            store.finish(&run_id, AcpRunStatus::Failed).await;
            return;
        }
    };

    while let Some(result) = agent_stream.next().await {
        match result {
            Ok(AgentEvent::Message(ref msg)) => {
                if let Some((await_req, metadata)) = extract_await_request(msg, &session_id) {
                    store.set_awaiting(&run_id, await_req, metadata).await;
                    let run = store.get(&run_id).await.unwrap();
                    let event = AcpEvent::run_awaiting(&run);
                    store.append_event(&run_id, event).await;
                    // Don't break — the agent stream continues when resumed
                    continue;
                }
                let acp_msg = goose_message_to_acp(msg);
                store.append_output(&run_id, acp_msg).await;
            }
            Err(e) => {
                let error = AcpError {
                    code: "stream_error".to_string(),
                    message: e.to_string(),
                    data: None,
                };
                store.set_error(&run_id, error).await;
                store.finish(&run_id, AcpRunStatus::Failed).await;
                return;
            }
            _ => {}
        }
    }

    store.finish(&run_id, AcpRunStatus::Completed).await;
}

// ── Core: streaming path ────────────────────────────────────────────

fn create_run_stream(
    state: AppState,
    run_id: String,
    session_id: String,
    req: RunCreateRequest,
    cancel_token: CancellationToken,
) -> impl Stream<Item = Result<SseEvent, std::convert::Infallible>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<SseEvent, std::convert::Infallible>>(100);

    tokio::spawn(async move {
        let store = state.run_store();
        let agent_name = req.agent_name.clone();

        let make_run = |status: AcpRunStatus| AcpRun {
            run_id: run_id.clone(),
            agent_name: agent_name.clone(),
            status,
            session_id: Some(session_id.clone()),
            output: Vec::new(),
            await_request: None,
            error: None,
            created_at: Utc::now(),
            finished_at: None,
            metadata: None,
        };

        // Emit run.created
        let created_run = make_run(AcpRunStatus::Created);
        let created_event = AcpEvent::run_created(&created_run);
        send_acp_sse(&tx, &created_event).await;
        store.append_event(&run_id, created_event).await;

        let user_message = match build_user_message(&req) {
            Some(msg) => msg,
            None => {
                let error = AcpError {
                    code: "invalid_input".to_string(),
                    message: "No user message provided".to_string(),
                    data: None,
                };
                store.set_error(&run_id, error.clone()).await;
                store.finish(&run_id, AcpRunStatus::Failed).await;
                let failed_run = make_run(AcpRunStatus::Failed);
                let event = AcpEvent::run_failed(&failed_run);
                send_acp_sse(&tx, &event).await;
                store.append_event(&run_id, event).await;
                return;
            }
        };

        // Emit run.in-progress
        store.update_status(&run_id, AcpRunStatus::InProgress).await;
        let ip_event = AcpEvent::run_in_progress(&make_run(AcpRunStatus::InProgress));
        send_acp_sse(&tx, &ip_event).await;
        store.append_event(&run_id, ip_event).await;

        let agent = match state.get_agent(session_id.clone()).await {
            Ok(a) => a,
            Err(e) => {
                let error = AcpError {
                    code: "agent_error".to_string(),
                    message: format!("Failed to get agent: {}", e),
                    data: None,
                };
                store.set_error(&run_id, error).await;
                store.finish(&run_id, AcpRunStatus::Failed).await;
                let event = AcpEvent::run_failed(&make_run(AcpRunStatus::Failed));
                send_acp_sse(&tx, &event).await;
                store.append_event(&run_id, event).await;
                return;
            }
        };

        apply_agent_bindings(&state, &agent, &agent_name).await;

        let session_config = SessionConfig {
            id: session_id.clone(),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        let mut agent_stream = match agent
            .reply(user_message, session_config, Some(cancel_token.clone()))
            .await
        {
            Ok(s) => s,
            Err(e) => {
                let error = AcpError {
                    code: "reply_error".to_string(),
                    message: e.to_string(),
                    data: None,
                };
                store.set_error(&run_id, error).await;
                store.finish(&run_id, AcpRunStatus::Failed).await;
                let event = AcpEvent::run_failed(&make_run(AcpRunStatus::Failed));
                send_acp_sse(&tx, &event).await;
                store.append_event(&run_id, event).await;
                return;
            }
        };

        let ctx = AcpEventContext {
            run_id: run_id.clone(),
            agent_name: agent_name.clone(),
            session_id: Some(session_id.clone()),
            created_at: Utc::now(),
        };

        // Stream agent events → ACP SSE events
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    store.finish(&run_id, AcpRunStatus::Cancelled).await;
                    let event = AcpEvent::run_cancelled(&make_run(AcpRunStatus::Cancelled));
                    send_acp_sse(&tx, &event).await;
                    store.append_event(&run_id, event).await;
                    return;
                }
                next = agent_stream.next() => {
                    match next {
                        Some(Ok(ref agent_event)) => {
                            // Check for ActionRequired → ACP Awaiting
                            if let AgentEvent::Message(ref msg) = agent_event {
                                if let Some((await_req, metadata)) = extract_await_request(msg, &session_id) {
                                    store.set_awaiting(&run_id, await_req, metadata).await;
                                    let awaiting_run = store.get(&run_id).await.unwrap_or_else(|| make_run(AcpRunStatus::Awaiting));
                                    let event = AcpEvent::run_awaiting(&awaiting_run);
                                    send_acp_sse(&tx, &event).await;
                                    store.append_event(&run_id, event).await;
                                    // Don't break — the agent stream stays open
                                    // (the tool/elicitation is blocked on a oneshot channel)
                                    continue;
                                }
                            }

                            let acp_events = agent_event_to_acp(agent_event, &ctx);
                            for acp_evt in &acp_events {
                                send_acp_sse(&tx, acp_evt).await;
                                store.append_event(&run_id, acp_evt.clone()).await;
                            }

                            if let AgentEvent::Message(ref msg) = agent_event {
                                let acp_msg = goose_message_to_acp(msg);
                                store.append_output(&run_id, acp_msg).await;
                            }
                        }
                        Some(Err(e)) => {
                            let error = AcpError {
                                code: "stream_error".to_string(),
                                message: e.to_string(),
                                data: None,
                            };
                            store.set_error(&run_id, error).await;
                            store.finish(&run_id, AcpRunStatus::Failed).await;
                            let event = AcpEvent::run_failed(&make_run(AcpRunStatus::Failed));
                            send_acp_sse(&tx, &event).await;
                            store.append_event(&run_id, event).await;
                            return;
                        }
                        None => break,
                    }
                }
            }
        }

        // Stream ended successfully
        store.finish(&run_id, AcpRunStatus::Completed).await;
        let event = AcpEvent::run_completed(&make_run(AcpRunStatus::Completed));
        send_acp_sse(&tx, &event).await;
        store.append_event(&run_id, event).await;
    });

    tokio_stream::wrappers::ReceiverStream::new(rx)
}

// ── Helpers ─────────────────────────────────────────────────────────

fn build_user_message(req: &RunCreateRequest) -> Option<Message> {
    req.input
        .iter()
        .rev()
        .find(|m| m.role == goose::acp_compat::message::AcpRole::User)
        .map(acp_message_to_goose)
}

/// Inspect a Message for ActionRequired content and return an ACP AwaitRequest if found.
fn extract_await_request(msg: &Message, session_id: &str) -> Option<(AwaitRequest, AwaitMetadata)> {
    for content in msg.content.iter() {
        match content {
            MessageContent::ActionRequired(action) => match &action.data {
                ActionRequiredData::Elicitation {
                    id,
                    message,
                    requested_schema,
                } => {
                    let await_req = AwaitRequest {
                        request_type: "elicitation".to_string(),
                        message: Some(message.clone()),
                        schema: Some(requested_schema.clone()),
                        metadata: Some(serde_json::json!({ "request_id": id })),
                    };
                    let metadata = AwaitMetadata::Elicitation {
                        request_id: id.clone(),
                    };
                    return Some((await_req, metadata));
                }
                ActionRequiredData::ToolConfirmation {
                    id,
                    tool_name,
                    arguments,
                    prompt,
                } => {
                    let await_req = AwaitRequest {
                        request_type: "tool_confirmation".to_string(),
                        message: prompt.clone(),
                        schema: Some(serde_json::json!({
                            "tool_name": tool_name,
                            "arguments": arguments,
                        })),
                        metadata: Some(serde_json::json!({ "request_id": id })),
                    };
                    let metadata = AwaitMetadata::ToolConfirmation {
                        request_id: id.clone(),
                        session_id: session_id.to_string(),
                    };
                    return Some((await_req, metadata));
                }
                ActionRequiredData::ElicitationResponse { .. } => {}
            },
            _ => continue,
        }
    }
    None
}

fn agent_event_to_acp(event: &AgentEvent, _ctx: &AcpEventContext) -> Vec<AcpEvent> {
    match event {
        AgentEvent::Message(msg) => {
            let acp_msg = goose_message_to_acp(msg);
            let mut events = Vec::new();
            events.push(AcpEvent::message_created(&acp_msg));
            for part in &acp_msg.parts {
                events.push(AcpEvent::message_part(part));
            }
            events.push(AcpEvent::message_completed(&acp_msg));
            events
        }
        AgentEvent::ModelChange { model, mode } => {
            vec![AcpEvent::generic(serde_json::json!({
                "goose.model_change": { "model": model, "mode": mode }
            }))]
        }
        AgentEvent::RoutingDecision {
            agent_name,
            mode_slug,
            confidence,
            reasoning,
        } => {
            vec![AcpEvent::generic(serde_json::json!({
                "goose.routing_decision": {
                    "agent_name": agent_name,
                    "mode_slug": mode_slug,
                    "confidence": confidence,
                    "reasoning": reasoning,
                }
            }))]
        }
        AgentEvent::McpNotification((request_id, notification)) => {
            vec![AcpEvent::generic(serde_json::json!({
                "goose.notification": {
                    "request_id": request_id,
                    "notification": format!("{:?}", notification),
                }
            }))]
        }
        AgentEvent::HistoryReplaced(_) => {
            vec![AcpEvent::generic(serde_json::json!({
                "goose.history_replaced": {}
            }))]
        }
        AgentEvent::ToolAvailabilityChange {
            previous_count,
            current_count,
        } => {
            vec![AcpEvent::generic(serde_json::json!({
                "goose.tool_availability_change": {
                    "previous_count": previous_count,
                    "current_count": current_count,
                }
            }))]
        }
    }
}

async fn send_acp_sse(
    tx: &tokio::sync::mpsc::Sender<Result<SseEvent, std::convert::Infallible>>,
    event: &AcpEvent,
) {
    if let Ok(json) = serde_json::to_string(&event.data) {
        let sse_event = SseEvent::default()
            .event(event.event_type.as_str())
            .data(json);
        let _ = tx.send(Ok(sse_event)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::acp_compat::message::AcpRole;

    fn make_run(id: &str, status: AcpRunStatus) -> AcpRun {
        AcpRun {
            run_id: id.to_string(),
            agent_name: "test-agent".to_string(),
            status,
            session_id: Some("test-session".to_string()),
            output: vec![],
            await_request: None,
            error: None,
            created_at: Utc::now(),
            finished_at: None,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn test_run_lifecycle_create_and_get() {
        let store = RunStore::new();
        let run = make_run("run-1", AcpRunStatus::Created);
        store.create(run, CancellationToken::new()).await;

        let fetched = store.get("run-1").await;
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().status, AcpRunStatus::Created);
    }

    #[tokio::test]
    async fn test_run_lifecycle_status_transitions() {
        let store = RunStore::new();
        let run = make_run("run-2", AcpRunStatus::Created);
        store.create(run, CancellationToken::new()).await;

        // Created → InProgress
        store.update_status("run-2", AcpRunStatus::InProgress).await;
        assert_eq!(
            store.get_status("run-2").await,
            Some(AcpRunStatus::InProgress)
        );

        // InProgress → Completed
        store.finish("run-2", AcpRunStatus::Completed).await;
        let finished = store.get("run-2").await.unwrap();
        assert_eq!(finished.status, AcpRunStatus::Completed);
        assert!(finished.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_run_lifecycle_awaiting_with_elicitation() {
        let store = RunStore::new();
        let run = make_run("run-3", AcpRunStatus::InProgress);
        store.create(run, CancellationToken::new()).await;

        let await_req = AwaitRequest {
            request_type: "elicitation".to_string(),
            message: Some("What is your name?".to_string()),
            schema: None,
            metadata: Some(serde_json::json!({"request_id": "req-1"})),
        };
        let metadata = AwaitMetadata::Elicitation {
            request_id: "req-1".to_string(),
        };
        store.set_awaiting("run-3", await_req, metadata).await;

        assert_eq!(
            store.get_status("run-3").await,
            Some(AcpRunStatus::Awaiting)
        );

        // Atomic take — should return metadata and transition away from Awaiting
        let taken = store.take_await_if_awaiting("run-3").await;
        assert!(taken.is_some());
        match taken.unwrap() {
            AwaitMetadata::Elicitation { request_id, .. } => {
                assert_eq!(request_id, "req-1");
            }
            _ => panic!("Expected Elicitation metadata"),
        }

        // Second take — should return None (already consumed)
        let taken_again = store.take_await_if_awaiting("run-3").await;
        assert!(taken_again.is_none());
    }

    #[tokio::test]
    async fn test_run_lifecycle_cancel() {
        let store = RunStore::new();
        let run = make_run("run-4", AcpRunStatus::InProgress);
        let cancel = CancellationToken::new();
        store.create(run, cancel.clone()).await;

        assert!(!cancel.is_cancelled());
        let cancelled = store.cancel("run-4").await;
        assert!(cancelled);
        assert!(cancel.is_cancelled());

        // cancel() fires the token; status update is done by the handler
        // Simulate what the handler does:
        store.update_status("run-4", AcpRunStatus::Cancelled).await;
        assert_eq!(
            store.get_status("run-4").await,
            Some(AcpRunStatus::Cancelled)
        );
    }

    #[tokio::test]
    async fn test_run_lifecycle_cancel_nonexistent() {
        let store = RunStore::new();
        assert!(!store.cancel("nonexistent").await);
    }

    #[tokio::test]
    async fn test_run_events_append_and_retrieve() {
        let store = RunStore::new();
        let run = make_run("run-5", AcpRunStatus::InProgress);
        store.create(run, CancellationToken::new()).await;

        let dummy = make_run("run-5", AcpRunStatus::InProgress);
        let event = AcpEvent::run_in_progress(&dummy);
        store.append_event("run-5", event).await;

        let events = store.get_events("run-5").await;
        assert!(events.is_some());
        assert_eq!(events.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_run_output_append() {
        let store = RunStore::new();
        let run = make_run("run-6", AcpRunStatus::InProgress);
        store.create(run, CancellationToken::new()).await;

        let msg = AcpMessage {
            role: AcpRole::Agent,
            parts: vec![],
        };
        store.append_output("run-6", msg).await;

        let r = store.get("run-6").await.unwrap();
        assert_eq!(r.output.len(), 1);
    }

    #[tokio::test]
    async fn test_run_error_handling() {
        let store = RunStore::new();
        let run = make_run("run-7", AcpRunStatus::InProgress);
        store.create(run, CancellationToken::new()).await;

        // set_error stores the error; status update is done by the handler
        let error = AcpError {
            code: "500".to_string(),
            message: "Something went wrong".to_string(),
            data: None,
        };
        store.set_error("run-7", error).await;
        store.finish("run-7", AcpRunStatus::Failed).await;

        let r = store.get("run-7").await.unwrap();
        assert_eq!(r.status, AcpRunStatus::Failed);
        assert!(r.error.is_some());
        assert_eq!(r.error.unwrap().code, "500");
        assert!(r.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_run_list_with_pagination() {
        let store = RunStore::new();
        for i in 0..5 {
            let run = make_run(&format!("run-{i}"), AcpRunStatus::Completed);
            store.create(run, CancellationToken::new()).await;
        }

        assert_eq!(store.list(10, 0).await.len(), 5);
        assert_eq!(store.list(2, 1).await.len(), 2);
        assert!(store.list(10, 10).await.is_empty());
    }

    #[tokio::test]
    async fn test_eviction_caps_completed_runs() {
        let store = RunStore::new();
        for i in 0..(MAX_COMPLETED_RUNS + 50) {
            let mut run = make_run(&format!("evict-{i}"), AcpRunStatus::Completed);
            run.finished_at = Some(Utc::now());
            store.create(run, CancellationToken::new()).await;
        }

        let all = store.list(MAX_COMPLETED_RUNS + 100, 0).await;
        assert!(
            all.len() <= MAX_COMPLETED_RUNS,
            "Expected <= {MAX_COMPLETED_RUNS}, got {}",
            all.len()
        );
    }

    #[tokio::test]
    async fn test_eviction_preserves_in_progress_runs() {
        let store = RunStore::new();

        // Create an in-progress run first
        let active = make_run("active-run", AcpRunStatus::InProgress);
        store.create(active, CancellationToken::new()).await;

        // Fill with completed runs to trigger eviction
        for i in 0..(MAX_COMPLETED_RUNS + 10) {
            let mut run = make_run(&format!("done-{i}"), AcpRunStatus::Completed);
            run.finished_at = Some(Utc::now());
            store.create(run, CancellationToken::new()).await;
        }

        // Active run must survive eviction
        assert!(store.get("active-run").await.is_some());
    }

    #[tokio::test]
    async fn test_get_nonexistent_run() {
        let store = RunStore::new();
        assert!(store.get("nonexistent").await.is_none());
        assert!(store.get_status("nonexistent").await.is_none());
        assert!(store.get_events("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_take_await_if_awaiting_wrong_status() {
        let store = RunStore::new();
        let run = make_run("run-wrong", AcpRunStatus::InProgress);
        store.create(run, CancellationToken::new()).await;

        // Not in Awaiting status — should return None
        assert!(store.take_await_if_awaiting("run-wrong").await.is_none());
    }
}
