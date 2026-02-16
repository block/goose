//! ACP-IDE (Agent Client Protocol) — JSON-RPC 2.0 entrypoint for IDE integration.
//!
//! Implements the Agent Client Protocol (agentclientprotocol.com) over:
//! - POST /acp  → JSON-RPC request/response over HTTP
//! - GET  /acp  → WebSocket upgrade for bidirectional JSON-RPC
//! - DELETE /acp → session cleanup
//!
//! All methods delegate to the shared AppState (same agents, sessions, modes
//! as REST and ACP-REST endpoints).

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header::HeaderName, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use goose::agents::{AgentEvent, SessionConfig};
use goose::conversation::message::{Message, MessageContent};
use goose::prompt_template;
use goose::registry::manifest::AgentMode;

use crate::state::AppState;

// ── JSON-RPC 2.0 Types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Value,
    id: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcNotification {
    jsonrpc: String,
    method: String,
    params: Value,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }

    fn method_not_found(id: Value, method: &str) -> Self {
        Self::error(id, -32601, format!("Method not found: {method}"))
    }

    fn invalid_params(id: Value, msg: impl Into<String>) -> Self {
        Self::error(id, -32602, msg)
    }

    fn internal_error(id: Value, msg: impl Into<String>) -> Self {
        Self::error(id, -32603, msg)
    }
}

impl JsonRpcNotification {
    fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

// ── ACP-IDE Session State ───────────────────────────────────────────────

/// Maximum number of IDE sessions before evicting the oldest idle ones.
const MAX_IDE_SESSIONS: usize = 100;

struct AcpIdeSession {
    cancel_token: Option<CancellationToken>,
    current_mode_id: Option<String>,
    notification_tx: mpsc::UnboundedSender<String>,
    last_activity: std::time::Instant,
}

#[derive(Default)]
pub struct AcpIdeSessions {
    sessions: Mutex<HashMap<String, AcpIdeSession>>,
}

impl AcpIdeSessions {
    pub fn new() -> Self {
        Self::default()
    }

    async fn has_session(&self, id: &str) -> bool {
        self.sessions.lock().await.contains_key(id)
    }

    async fn remove_session(&self, id: &str) {
        self.sessions.lock().await.remove(id);
    }

    async fn touch(&self, id: &str) {
        if let Some(session) = self.sessions.lock().await.get_mut(id) {
            session.last_activity = std::time::Instant::now();
        }
    }

    async fn evict_idle(&self) {
        let mut sessions = self.sessions.lock().await;
        if sessions.len() <= MAX_IDE_SESSIONS {
            return;
        }
        let mut entries: Vec<(String, std::time::Instant)> = sessions
            .iter()
            .map(|(k, v)| (k.clone(), v.last_activity))
            .collect();
        entries.sort_by_key(|(_, t)| *t);
        let to_remove = sessions.len() - MAX_IDE_SESSIONS;
        for (id, _) in entries.into_iter().take(to_remove) {
            sessions.remove(&id);
        }
    }
}

// ── Constants ───────────────────────────────────────────────────────────

const HEADER_SESSION_ID: &str = "acp-session-id";
const PARSE_ERROR: i32 = -32700;
const INVALID_REQUEST: i32 = -32600;

// ── Routes ──────────────────────────────────────────────────────────────

pub fn routes(state: Arc<AppState>) -> Router {
    let ide_sessions = Arc::new(AcpIdeSessions::new());

    Router::new()
        .route("/acp", post(handle_post))
        .route("/acp", get(handle_get))
        .route("/acp", delete(handle_delete))
        .with_state((state, ide_sessions))
}

type AcpState = (Arc<AppState>, Arc<AcpIdeSessions>);

// ── POST /acp — JSON-RPC over HTTP ─────────────────────────────────────

async fn handle_post(
    State((state, ide_sessions)): State<AcpState>,
    request: Request<axum::body::Body>,
) -> Response {
    let session_id = get_session_id(&request);

    let body = match axum::body::to_bytes(request.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response(),
    };

    let json_value: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            let resp = JsonRpcResponse::error(Value::Null, PARSE_ERROR, "Parse error");
            return axum::Json(resp).into_response();
        }
    };

    if json_value.is_array() {
        return (StatusCode::NOT_IMPLEMENTED, "Batch requests not supported").into_response();
    }

    let rpc_req: JsonRpcRequest = match serde_json::from_value(json_value.clone()) {
        Ok(r) => r,
        Err(_) => {
            let resp =
                JsonRpcResponse::error(Value::Null, INVALID_REQUEST, "Invalid JSON-RPC request");
            return axum::Json(resp).into_response();
        }
    };

    // Initialize creates a new session — no session ID required
    if rpc_req.method == "initialize" {
        let resp = handle_initialize(&ide_sessions, &rpc_req).await;
        // Extract session_id from result to set as response header
        let new_session_id = resp
            .result
            .as_ref()
            .and_then(|r| r.get("_session_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut response = axum::Json(resp).into_response();
        if let Some(sid) = new_session_id {
            let header_name = HeaderName::from_static(HEADER_SESSION_ID);
            if let Ok(hv) = HeaderValue::from_str(&sid) {
                response.headers_mut().insert(header_name, hv);
            }
        }
        return response;
    }

    // Handle notifications (no id field) — e.g. cancel
    if is_notification(&json_value) {
        if let Some(ref sid) = session_id {
            if rpc_req.method == "cancel" {
                let sessions = ide_sessions.sessions.lock().await;
                if let Some(session) = sessions.get(sid.as_str()) {
                    if let Some(ref token) = session.cancel_token {
                        token.cancel();
                    }
                }
            }
        }
        return StatusCode::ACCEPTED.into_response();
    }

    // All other methods require session ID
    let session_id = match session_id {
        Some(id) => id,
        None => {
            return axum::Json(JsonRpcResponse::invalid_params(
                rpc_req.id,
                "Acp-Session-Id header required",
            ))
            .into_response();
        }
    };

    if !ide_sessions.has_session(&session_id).await {
        return axum::Json(JsonRpcResponse::invalid_params(
            rpc_req.id,
            format!("Session not found: {session_id}"),
        ))
        .into_response();
    }

    let resp = dispatch(&state, &ide_sessions, &session_id, &rpc_req).await;
    axum::Json(resp).into_response()
}

// ── GET /acp — WebSocket upgrade for bidirectional JSON-RPC ─────────────

async fn handle_get(
    State((state, ide_sessions)): State<AcpState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state, ide_sessions))
}

async fn handle_websocket(
    socket: WebSocket,
    state: Arc<AppState>,
    ide_sessions: Arc<AcpIdeSessions>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (notif_tx, mut notif_rx) = mpsc::unbounded_channel::<String>();

    let mut session_id: Option<String> = None;

    // Outbound channel for JSON-RPC responses (request → response flow)
    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<String>();

    // Spawn forwarder: merges JSON-RPC responses AND streaming notifications → WebSocket
    tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = outbound_rx.recv() => match msg {
                    Some(text) => {
                        if ws_tx.send(WsMessage::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                },
                msg = notif_rx.recv() => match msg {
                    Some(text) => {
                        if ws_tx.send(WsMessage::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                },
            }
        }
    });

    // Process incoming WebSocket messages
    while let Some(Ok(msg)) = ws_rx.next().await {
        let text = match msg {
            WsMessage::Text(t) => t.to_string(),
            WsMessage::Close(_) => break,
            _ => continue,
        };

        let json_value: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                let resp = JsonRpcResponse::error(Value::Null, PARSE_ERROR, "Parse error");
                let _ = outbound_tx.send(serde_json::to_string(&resp).unwrap_or_default());
                continue;
            }
        };

        let rpc_req: JsonRpcRequest = match serde_json::from_value(json_value.clone()) {
            Ok(r) => r,
            Err(_) => {
                let resp = JsonRpcResponse::error(Value::Null, INVALID_REQUEST, "Invalid request");
                let _ = outbound_tx.send(serde_json::to_string(&resp).unwrap_or_default());
                continue;
            }
        };

        // Initialize
        if rpc_req.method == "initialize" {
            let resp = handle_initialize(&ide_sessions, &rpc_req).await;
            if let Some(ref result) = resp.result {
                if let Some(sid) = result.get("_session_id").and_then(|v| v.as_str()) {
                    session_id = Some(sid.to_string());
                    // Update notification sender for this session
                    let mut sessions = ide_sessions.sessions.lock().await;
                    if let Some(session) = sessions.get_mut(sid) {
                        session.notification_tx = notif_tx.clone();
                    }
                }
            }
            let _ = outbound_tx.send(serde_json::to_string(&resp).unwrap_or_default());
            continue;
        }

        // Notifications (no id)
        if is_notification(&json_value) {
            if let Some(ref sid) = session_id {
                if rpc_req.method == "cancel" {
                    let sessions = ide_sessions.sessions.lock().await;
                    if let Some(session) = sessions.get(sid.as_str()) {
                        if let Some(ref token) = session.cancel_token {
                            token.cancel();
                        }
                    }
                }
            }
            continue;
        }

        let sid = match &session_id {
            Some(s) => s.clone(),
            None => {
                let resp = JsonRpcResponse::invalid_params(rpc_req.id, "Not initialized");
                let _ = outbound_tx.send(serde_json::to_string(&resp).unwrap_or_default());
                continue;
            }
        };

        let resp = dispatch(&state, &ide_sessions, &sid, &rpc_req).await;
        let _ = outbound_tx.send(serde_json::to_string(&resp).unwrap_or_default());
    }

    // Cleanup on disconnect
    if let Some(sid) = session_id {
        ide_sessions.remove_session(&sid).await;
    }
}

// ── DELETE /acp — session cleanup ───────────────────────────────────────

async fn handle_delete(
    State((_state, ide_sessions)): State<AcpState>,
    request: Request<axum::body::Body>,
) -> Response {
    let session_id = match get_session_id(&request) {
        Some(id) => id,
        None => return (StatusCode::BAD_REQUEST, "Acp-Session-Id header required").into_response(),
    };

    if !ide_sessions.has_session(&session_id).await {
        return (StatusCode::NOT_FOUND, "Session not found").into_response();
    }

    ide_sessions.remove_session(&session_id).await;
    StatusCode::ACCEPTED.into_response()
}

// ── Method Dispatcher ───────────────────────────────────────────────────

async fn dispatch(
    state: &Arc<AppState>,
    ide_sessions: &Arc<AcpIdeSessions>,
    session_id: &str,
    req: &JsonRpcRequest,
) -> JsonRpcResponse {
    match req.method.as_str() {
        "new_session" => handle_new_session(ide_sessions, req).await,
        "load_session" => handle_load_session(state, ide_sessions, req).await,
        "prompt" => handle_prompt(state, ide_sessions, session_id, req).await,
        "cancel" => handle_cancel(ide_sessions, session_id, req).await,
        "set_session_mode" => handle_set_mode(state, ide_sessions, session_id, req).await,
        "set_session_model" => handle_set_model(req).await,
        _ => JsonRpcResponse::method_not_found(req.id.clone(), &req.method),
    }
}

// ── Method Handlers ─────────────────────────────────────────────────────

async fn handle_initialize(
    ide_sessions: &Arc<AcpIdeSessions>,
    req: &JsonRpcRequest,
) -> JsonRpcResponse {
    debug!("ACP-IDE: initialize");

    let session_id = uuid::Uuid::new_v4().to_string();
    let (notif_tx, _notif_rx) = mpsc::unbounded_channel();

    ide_sessions.sessions.lock().await.insert(
        session_id.clone(),
        AcpIdeSession {
            cancel_token: None,
            current_mode_id: None,
            notification_tx: notif_tx,
            last_activity: std::time::Instant::now(),
        },
    );
    ide_sessions.evict_idle().await;

    let modes = collect_modes();

    // ACP-IDE standard: modes list (each mode = a persona/agent)
    let mode_list: Vec<Value> = modes
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.slug,
                "name": m.name,
                "description": m.description,
            })
        })
        .collect();

    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({
            "protocol_version": "2024-11-05",
            "_session_id": session_id,
            "agent_capabilities": {
                "load_session": true,
                "prompt_capabilities": {
                    "image": true,
                    "audio": false,
                    "embedded_context": true,
                },
            },
            // ACP-IDE recommended: Session Config Options (preferred over raw modes)
            // ACP-IDE Session Config Options: agent = role, only behavior_mode is configurable
            "config_options": [
                {
                    "id": "behavior_mode",
                    "type": "select",
                    "label": "Behavior",
                    "description": "Controls the agent's level of initiative and action style",
                    "options": [
                        { "value": "ask", "label": "Ask", "description": "Answer questions without making changes" },
                        { "value": "architect", "label": "Architect", "description": "Plan and design without implementing" },
                        { "value": "code", "label": "Code", "description": "Full autonomy to read, write, and execute" },
                    ],
                    "default": "code",
                },
            ],
            // Backward-compatible modes list
            "modes": {
                "available": mode_list,
                "default": "assistant",
            },
        }),
    )
}

async fn handle_new_session(
    ide_sessions: &Arc<AcpIdeSessions>,
    req: &JsonRpcRequest,
) -> JsonRpcResponse {
    debug!("ACP-IDE: new_session");

    let session_id = uuid::Uuid::new_v4().to_string();
    let (notif_tx, _) = mpsc::unbounded_channel();

    ide_sessions.sessions.lock().await.insert(
        session_id.clone(),
        AcpIdeSession {
            cancel_token: None,
            current_mode_id: None,
            notification_tx: notif_tx,
            last_activity: std::time::Instant::now(),
        },
    );
    ide_sessions.evict_idle().await;

    let modes = collect_modes();
    let mode_list: Vec<Value> = modes
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.slug,
                "name": m.name,
                "description": m.description,
            })
        })
        .collect();

    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({
            "session_id": session_id,
            "modes": {
                "current_mode_id": "assistant",
                "available_modes": mode_list,
            },
        }),
    )
}

async fn handle_load_session(
    state: &Arc<AppState>,
    ide_sessions: &Arc<AcpIdeSessions>,
    req: &JsonRpcRequest,
) -> JsonRpcResponse {
    let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return JsonRpcResponse::invalid_params(req.id.clone(), "session_id required"),
    };

    match state.session_manager().get_session(&session_id, true).await {
        Ok(_session) => {
            let (notif_tx, _) = mpsc::unbounded_channel();
            ide_sessions.sessions.lock().await.insert(
                session_id.clone(),
                AcpIdeSession {
                    cancel_token: None,
                    current_mode_id: None,
                    notification_tx: notif_tx,
                    last_activity: std::time::Instant::now(),
                },
            );
            ide_sessions.evict_idle().await;

            let modes = collect_modes();
            let mode_list: Vec<Value> = modes
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "id": m.slug,
                        "name": m.name,
                        "description": m.description,
                    })
                })
                .collect();

            JsonRpcResponse::success(
                req.id.clone(),
                serde_json::json!({
                    "session_id": session_id,
                    "modes": {
                        "current_mode_id": "assistant",
                        "available_modes": mode_list,
                    },
                }),
            )
        }
        Err(_) => JsonRpcResponse::invalid_params(
            req.id.clone(),
            format!("Session not found: {session_id}"),
        ),
    }
}

async fn handle_prompt(
    state: &Arc<AppState>,
    ide_sessions: &Arc<AcpIdeSessions>,
    session_id: &str,
    req: &JsonRpcRequest,
) -> JsonRpcResponse {
    debug!(session_id, "ACP-IDE: prompt");

    ide_sessions.touch(session_id).await;
    let cancel_token = CancellationToken::new();

    // Store cancel token
    {
        let mut sessions = ide_sessions.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.cancel_token = Some(cancel_token.clone());
        }
    }

    let agent = match state.get_agent(session_id.to_string()).await {
        Ok(a) => a,
        Err(e) => {
            return JsonRpcResponse::internal_error(
                req.id.clone(),
                format!("Failed to get agent: {e}"),
            )
        }
    };

    let user_message = build_message_from_prompt(&req.params);

    let session_config = SessionConfig {
        id: session_id.to_string(),
        schedule_id: None,
        max_turns: None,
        retry_config: None,
    };

    // Get notification sender for streaming
    let notif_tx = {
        let sessions = ide_sessions.sessions.lock().await;
        sessions.get(session_id).map(|s| s.notification_tx.clone())
    };

    let mut stream = match agent
        .reply(user_message, session_config, Some(cancel_token.clone()))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return JsonRpcResponse::internal_error(
                req.id.clone(),
                format!("Failed to start reply: {e}"),
            )
        }
    };

    let mut was_cancelled = false;

    while let Some(event) = stream.next().await {
        if cancel_token.is_cancelled() {
            was_cancelled = true;
            break;
        }

        match event {
            Ok(AgentEvent::Message(message)) => {
                if let Some(ref tx) = notif_tx {
                    for content in &message.content {
                        if let Some(notif) = content_to_notification(session_id, content) {
                            let _ = tx.send(serde_json::to_string(&notif).unwrap_or_default());
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                return JsonRpcResponse::internal_error(
                    req.id.clone(),
                    format!("Agent stream error: {e}"),
                )
            }
        }
    }

    // Clear cancel token
    {
        let mut sessions = ide_sessions.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.cancel_token = None;
        }
    }

    let stop_reason = if was_cancelled {
        "cancelled"
    } else {
        "end_turn"
    };

    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({ "stop_reason": stop_reason }),
    )
}

async fn handle_cancel(
    ide_sessions: &Arc<AcpIdeSessions>,
    session_id: &str,
    req: &JsonRpcRequest,
) -> JsonRpcResponse {
    let sessions = ide_sessions.sessions.lock().await;
    if let Some(session) = sessions.get(session_id) {
        if let Some(ref token) = session.cancel_token {
            token.cancel();
            info!(session_id, "ACP-IDE: cancelled");
        }
    }
    JsonRpcResponse::success(req.id.clone(), serde_json::json!({}))
}

async fn handle_set_mode(
    state: &Arc<AppState>,
    ide_sessions: &Arc<AcpIdeSessions>,
    session_id: &str,
    req: &JsonRpcRequest,
) -> JsonRpcResponse {
    let mode_id = match req.params.get("mode_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return JsonRpcResponse::invalid_params(req.id.clone(), "mode_id required"),
    };

    let modes = collect_modes();
    let mode = match modes.iter().find(|m| m.slug == mode_id) {
        Some(m) => m,
        None => {
            return JsonRpcResponse::invalid_params(
                req.id.clone(),
                format!("Unknown mode: {mode_id}"),
            )
        }
    };

    let agent = match state.get_agent(session_id.to_string()).await {
        Ok(a) => a,
        Err(e) => {
            return JsonRpcResponse::internal_error(
                req.id.clone(),
                format!("Failed to get agent: {e}"),
            )
        }
    };

    agent.set_active_tool_groups(mode.tool_groups.clone()).await;

    let instructions = resolve_mode_instructions(mode);
    agent
        .extend_system_prompt("agent_mode".to_string(), instructions.unwrap_or_default())
        .await;

    {
        let mut sessions = ide_sessions.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.current_mode_id = Some(mode_id.to_string());
        }
    }

    info!(session_id, mode_id, "ACP-IDE: mode changed");
    ide_sessions.touch(session_id).await;
    JsonRpcResponse::success(req.id.clone(), serde_json::json!({}))
}

async fn handle_set_model(req: &JsonRpcRequest) -> JsonRpcResponse {
    let model_id = req
        .params
        .get("model_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    warn!(model_id, "ACP-IDE: set_session_model not yet implemented");
    JsonRpcResponse::success(req.id.clone(), serde_json::json!({}))
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn get_session_id<B>(request: &Request<B>) -> Option<String> {
    request
        .headers()
        .get(HEADER_SESSION_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn is_notification(value: &Value) -> bool {
    value.get("method").is_some() && value.get("id").is_none()
}

fn collect_modes() -> Vec<AgentMode> {
    use goose::agents::coding_agent::CodingAgent;
    use goose::agents::goose_agent::GooseAgent;

    let goose = GooseAgent::new();
    let coding = CodingAgent::new();

    let mut modes = goose.to_public_agent_modes();
    modes.extend(coding.to_agent_modes());
    modes
}

fn resolve_mode_instructions(mode: &AgentMode) -> Option<String> {
    if let Some(ref instructions) = mode.instructions {
        return Some(instructions.clone());
    }
    if let Some(ref file) = mode.instructions_file {
        match prompt_template::render_template(file, &HashMap::<String, String>::new()) {
            Ok(rendered) => return Some(rendered),
            Err(e) => {
                warn!(mode = %mode.slug, file = %file, error = %e,
                      "Failed to render mode instructions_file");
            }
        }
    }
    None
}

fn build_message_from_prompt(params: &Value) -> Message {
    let mut contents = Vec::new();

    if let Some(prompt) = params.get("prompt") {
        if let Some(blocks) = prompt.as_array() {
            for block in blocks {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    contents.push(MessageContent::text(text));
                } else if let Some(text) = block.get("content").and_then(|t| t.as_str()) {
                    contents.push(MessageContent::text(text));
                }
            }
        } else if let Some(text) = prompt.as_str() {
            contents.push(MessageContent::text(text));
        }
    }

    if contents.is_empty() {
        if let Some(text) = params.get("text").and_then(|t| t.as_str()) {
            contents.push(MessageContent::text(text));
        }
    }

    if contents.is_empty() {
        contents.push(MessageContent::text(""));
    }

    // Build message by chaining with_content calls
    let mut msg = Message::user();
    for content in contents {
        msg = msg.with_content(content);
    }
    msg
}

fn content_to_notification(
    session_id: &str,
    content: &MessageContent,
) -> Option<JsonRpcNotification> {
    match content {
        MessageContent::Text(tc) => Some(JsonRpcNotification::new(
            "session/update",
            serde_json::json!({
                "session_id": session_id,
                "update": { "type": "text", "text": tc.text }
            }),
        )),
        MessageContent::ToolRequest(tr) => {
            let tool_name = match &tr.tool_call {
                Ok(params) => params.name.to_string(),
                Err(_) => "unknown".to_string(),
            };
            Some(JsonRpcNotification::new(
                "session/update",
                serde_json::json!({
                    "session_id": session_id,
                    "update": {
                        "type": "tool_call",
                        "tool_call": { "id": tr.id, "name": tool_name, "status": "running" }
                    }
                }),
            ))
        }
        MessageContent::ToolResponse(tr) => Some(JsonRpcNotification::new(
            "session/update",
            serde_json::json!({
                "session_id": session_id,
                "update": { "type": "tool_result", "tool_call_id": tr.id }
            }),
        )),
        MessageContent::Thinking(tc) => Some(JsonRpcNotification::new(
            "session/update",
            serde_json::json!({
                "session_id": session_id,
                "update": { "type": "thinking", "text": tc.thinking }
            }),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_response_success() {
        let resp =
            JsonRpcResponse::success(Value::Number(1.into()), serde_json::json!({"status": "ok"}));
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let resp = JsonRpcResponse::error(Value::Number(1.into()), -32601, "Method not found");
        assert!(resp.result.is_none());
        assert_eq!(resp.error.as_ref().unwrap().code, -32601);
    }

    #[test]
    fn test_collect_modes() {
        let modes = collect_modes();
        // GooseAgent: 4 public modes; CodingAgent: 5 modes → 9 total for IDE switching
        assert!(
            modes.len() >= 9,
            "Expected at least 9 public modes, got {}",
            modes.len()
        );
        let slugs: Vec<&str> = modes.iter().map(|m| m.slug.as_str()).collect();
        assert!(slugs.contains(&"assistant"));
        assert!(slugs.contains(&"code"));

        // Internal modes must not be exposed to IDE clients
        assert!(!slugs.contains(&"judge"), "Internal mode 'judge' leaked");
        assert!(
            !slugs.contains(&"planner"),
            "Internal mode 'planner' leaked"
        );
        assert!(
            !slugs.contains(&"recipe_maker"),
            "Internal mode 'recipe_maker' leaked"
        );
    }

    #[test]
    fn test_build_message_array_prompt() {
        let params = serde_json::json!({ "prompt": [{ "text": "Hello world" }] });
        let msg = build_message_from_prompt(&params);
        assert_eq!(msg.content.len(), 1);
    }

    #[test]
    fn test_build_message_string_prompt() {
        let params = serde_json::json!({ "prompt": "Hello world" });
        let msg = build_message_from_prompt(&params);
        assert_eq!(msg.content.len(), 1);
    }

    #[test]
    fn test_is_notification() {
        let notif = serde_json::json!({"jsonrpc": "2.0", "method": "cancel"});
        assert!(is_notification(&notif));

        let req = serde_json::json!({"jsonrpc": "2.0", "method": "prompt", "id": 1});
        assert!(!is_notification(&req));
    }

    #[test]
    fn test_content_to_notification_text() {
        let content = MessageContent::text("hello");
        let notif = content_to_notification("sess-1", &content);
        assert!(notif.is_some());
        assert_eq!(notif.unwrap().method, "session/update");
    }
}
