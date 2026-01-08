use anyhow::Result;
use axum::response::Redirect;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, Request, State,
    },
    http::{StatusCode, Uri},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use base64::Engine;
use futures::{sink::SinkExt, stream::StreamExt};
use goose::agents::{Agent, AgentEvent};
use goose::conversation::message::Message as GooseMessage;
use goose::session::session_manager::SessionType;
use goose::session::SessionManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::error;
use webbrowser;

type CancellationStore = Arc<RwLock<std::collections::HashMap<String, tokio::task::AbortHandle>>>;

#[derive(Clone)]
struct AppState {
    agent: Arc<Agent>,
    cancellations: CancellationStore,
    auth_token: Option<String>,
    ws_token: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum WebSocketMessage {
    #[serde(rename = "message")]
    Message {
        content: String,
        session_id: String,
        timestamp: i64,
    },
    #[serde(rename = "cancel")]
    Cancel { session_id: String },
    #[serde(rename = "response")]
    Response {
        content: String,
        role: String,
        timestamp: i64,
    },
    #[serde(rename = "tool_request")]
    ToolRequest {
        id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },
    #[serde(rename = "tool_response")]
    ToolResponse {
        id: String,
        result: serde_json::Value,
        is_error: bool,
    },
    #[serde(rename = "tool_confirmation")]
    ToolConfirmation {
        id: String,
        tool_name: String,
        arguments: serde_json::Value,
        needs_confirmation: bool,
    },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "thinking")]
    Thinking { message: String },
    #[serde(rename = "context_exceeded")]
    ContextExceeded { message: String },
    #[serde(rename = "cancelled")]
    Cancelled { message: String },
    #[serde(rename = "complete")]
    Complete { message: String },
}

async fn auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.uri().path() == "/api/health" {
        return Ok(next.run(req).await);
    }

    let Some(ref expected_token) = state.auth_token else {
        return Ok(next.run(req).await);
    };

    if let Some(auth_header) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if token == expected_token {
                    return Ok(next.run(req).await);
                }
            }

            if let Some(basic_token) = auth_str.strip_prefix("Basic ") {
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(basic_token) {
                    if let Ok(credentials) = String::from_utf8(decoded) {
                        if credentials.ends_with(expected_token) {
                            return Ok(next.run(req).await);
                        }
                    }
                }
            }
        }
    }

    let mut response = Response::new("Authentication required".into());
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    response.headers_mut().insert(
        "WWW-Authenticate",
        "Basic realm=\"Goose Web Interface\"".parse().unwrap(),
    );
    Ok(response)
}

pub async fn handle_web(
    port: u16,
    host: String,
    open: bool,
    auth_token: Option<String>,
) -> Result<()> {
    crate::logging::setup_logging(Some("goose-web"), None)?;

    let config = goose::config::Config::global();

    let provider_name: String = match config.get_goose_provider() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("No provider configured. Run 'goose configure' first");
            std::process::exit(1);
        }
    };

    let model: String = match config.get_goose_model() {
        Ok(m) => m,
        Err(_) => {
            eprintln!("No model configured. Run 'goose configure' first");
            std::process::exit(1);
        }
    };

    let model_config = goose::model::ModelConfig::new(&model)?;

    let init_session = SessionManager::create_session(
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        "Web Agent Initialization".to_string(),
        SessionType::Hidden,
    )
    .await?;

    let agent = Agent::new();
    let provider = goose::providers::create(&provider_name, model_config).await?;
    agent.update_provider(provider, &init_session.id).await?;

    let enabled_configs = goose::config::get_enabled_extensions();
    for config in enabled_configs {
        if let Err(e) = agent.add_extension(config.clone()).await {
            eprintln!("Warning: Failed to load extension {}: {}", config.name(), e);
        }
    }

    let ws_token = if auth_token.is_none() {
        uuid::Uuid::new_v4().to_string()
    } else {
        String::new()
    };

    let state = AppState {
        agent: Arc::new(agent),
        cancellations: Arc::new(RwLock::new(std::collections::HashMap::new())),
        auth_token: auth_token.clone(),
        ws_token,
    };

    let cors_layer = if auth_token.is_none() {
        let allowed_origins = [
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
            format!("http://{}:{}", host, port).parse().unwrap(),
        ];
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(allowed_origins))
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/session/{session_name}", get(serve_session))
        .route("/ws", get(websocket_handler))
        .route("/api/health", get(health_check))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{session_id}", get(get_session))
        .route("/static/{*path}", get(serve_static))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(cors_layer)
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    println!("\n🪿 Starting goose web server");
    println!("   Provider: {} | Model: {}", provider_name, model);
    println!(
        "   Working directory: {}",
        std::env::current_dir()?.display()
    );
    println!("   Server: http://{}", addr);
    println!("   Press Ctrl+C to stop\n");

    if open {
        let url = format!("http://{}", addr);
        if let Err(e) = webbrowser::open(&url) {
            eprintln!("Failed to open browser: {}", e);
        }
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_index(uri: Uri) -> Result<Redirect, (http::StatusCode, String)> {
    let session = SessionManager::create_session(
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        "Web session".to_string(),
        SessionType::User,
    )
    .await
    .map_err(|err| (http::StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let redirect_url = if let Some(query) = uri.query() {
        format!("/session/{}?{}", session.id, query)
    } else {
        format!("/session/{}", session.id)
    };

    Ok(Redirect::to(&redirect_url))
}

async fn serve_session(
    axum::extract::Path(session_name): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> Html<String> {
    let html = include_str!("../../static/index.html");
    let html_with_session = html.replace(
        "<script src=\"/static/script.js\"></script>",
        &format!(
            "<script>window.GOOSE_SESSION_NAME = '{}'; window.GOOSE_WS_TOKEN = '{}';</script>\n    <script src=\"/static/script.js\"></script>",
            session_name,
            state.ws_token
        )
    );
    Html(html_with_session)
}

async fn serve_static(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    match path.as_str() {
        "style.css" => (
            [("content-type", "text/css")],
            include_str!("../../static/style.css"),
        )
            .into_response(),
        "script.js" => (
            [("content-type", "application/javascript")],
            include_str!("../../static/script.js"),
        )
            .into_response(),
        "img/logo_dark.png" => (
            [("content-type", "image/png")],
            include_bytes!("../../../../documentation/static/img/logo_dark.png").to_vec(),
        )
            .into_response(),
        "img/logo_light.png" => (
            [("content-type", "image/png")],
            include_bytes!("../../../../documentation/static/img/logo_light.png").to_vec(),
        )
            .into_response(),
        _ => (http::StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "goose-web"
    }))
}

async fn list_sessions() -> Json<serde_json::Value> {
    match SessionManager::list_sessions().await {
        Ok(sessions) => {
            let mut session_info = Vec::new();

            for session in sessions {
                session_info.push(serde_json::json!({
                    "name": session.id,
                    "path": session.id,
                    "description": session.name,
                    "message_count": session.message_count,
                    "working_dir": session.working_dir
                }));
            }
            Json(serde_json::json!({
                "sessions": session_info
            }))
        }
        Err(e) => Json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}
async fn get_session(
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    match SessionManager::get_session(&session_id, true).await {
        Ok(session) => Json(serde_json::json!({
            "metadata": session,
            "messages": session.conversation.unwrap_or_default().messages()
        })),
        Err(e) => Json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[derive(Deserialize)]
struct WsQuery {
    token: Option<String>,
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    if state.auth_token.is_none() {
        let provided_token = query.token.as_deref().unwrap_or("");
        if provided_token != state.ws_token {
            tracing::warn!("WebSocket connection rejected: invalid token");
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(ws.on_upgrade(|socket| handle_socket(socket, state)))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<WebSocketMessage>(&text.to_string()) {
                        Ok(WebSocketMessage::Message {
                            content,
                            session_id,
                            ..
                        }) => {
                            let sender_clone = sender.clone();
                            let agent = state.agent.clone();
                            let session_id_clone = session_id.clone();

                            let task_handle = tokio::spawn(async move {
                                let result = process_message_streaming(
                                    &agent,
                                    session_id_clone,
                                    content,
                                    sender_clone,
                                )
                                .await;

                                if let Err(e) = result {
                                    error!("Error processing message: {}", e);
                                }
                            });

                            {
                                let mut cancellations = state.cancellations.write().await;
                                cancellations
                                    .insert(session_id.clone(), task_handle.abort_handle());
                            }

                            // Handle task completion and cleanup
                            let sender_for_abort = sender.clone();
                            let session_id_for_cleanup = session_id.clone();
                            let cancellations_for_cleanup = state.cancellations.clone();

                            tokio::spawn(async move {
                                match task_handle.await {
                                    Ok(_) => {}
                                    Err(e) if e.is_cancelled() => {
                                        let mut sender = sender_for_abort.lock().await;
                                        let _ = sender
                                            .send(Message::Text(
                                                serde_json::to_string(
                                                    &WebSocketMessage::Cancelled {
                                                        message: "Operation cancelled by user"
                                                            .to_string(),
                                                    },
                                                )
                                                .unwrap()
                                                .into(),
                                            ))
                                            .await;
                                    }
                                    Err(e) => {
                                        error!("Task error: {}", e);
                                    }
                                }

                                let mut cancellations = cancellations_for_cleanup.write().await;
                                cancellations.remove(&session_id_for_cleanup);
                            });
                        }
                        Ok(WebSocketMessage::Cancel { session_id }) => {
                            // Cancel the active operation for this session
                            let abort_handle = {
                                let mut cancellations = state.cancellations.write().await;
                                cancellations.remove(&session_id)
                            };

                            if let Some(handle) = abort_handle {
                                handle.abort();

                                // Send cancellation confirmation
                                let mut sender = sender.lock().await;
                                let _ = sender
                                    .send(Message::Text(
                                        serde_json::to_string(&WebSocketMessage::Cancelled {
                                            message: "Operation cancelled".to_string(),
                                        })
                                        .unwrap()
                                        .into(),
                                    ))
                                    .await;
                            }
                        }
                        Ok(_) => {
                            // Ignore other message types
                        }
                        Err(e) => {
                            error!("Failed to parse WebSocket message: {}", e);
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }
}

async fn process_message_streaming(
    agent: &Agent,
    session_id: String,
    content: String,
    sender: Arc<Mutex<futures::stream::SplitSink<WebSocket, Message>>>,
) -> Result<()> {
    use futures::StreamExt;
    use goose::agents::SessionConfig;
    use goose::conversation::message::MessageContent;

    let user_message = GooseMessage::user().with_text(content.clone());

    let provider = agent.provider().await;
    if provider.is_err() {
        let error_msg = "I'm not properly configured yet. Please configure a provider through the CLI first using `goose configure`.".to_string();
        let mut sender = sender.lock().await;
        let _ = sender
            .send(Message::Text(
                serde_json::to_string(&WebSocketMessage::Response {
                    content: error_msg,
                    role: "assistant".to_string(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                })
                .unwrap()
                .into(),
            ))
            .await;
        return Ok(());
    }

    let session = SessionManager::get_session(&session_id, true).await?;
    let mut messages = session.conversation.unwrap_or_default();
    messages.push(user_message.clone());

    let session_config = SessionConfig {
        id: session.id.clone(),
        schedule_id: None,
        max_turns: None,
        retry_config: None,
    };

    match agent.reply(user_message, session_config, None).await {
        Ok(mut stream) => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(AgentEvent::Message(message)) => {
                        for content in &message.content {
                            match content {
                                MessageContent::Text(text) => {
                                    let mut sender = sender.lock().await;
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(&WebSocketMessage::Response {
                                                content: text.text.clone(),
                                                role: "assistant".to_string(),
                                                timestamp: chrono::Utc::now().timestamp_millis(),
                                            })
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await;
                                }
                                MessageContent::ToolRequest(req) => {
                                    let mut sender = sender.lock().await;
                                    if let Ok(tool_call) = &req.tool_call {
                                        let _ = sender
                                            .send(Message::Text(
                                                serde_json::to_string(
                                                    &WebSocketMessage::ToolRequest {
                                                        id: req.id.clone(),
                                                        tool_name: tool_call.name.to_string(),
                                                        arguments: Value::from(
                                                            tool_call.arguments.clone(),
                                                        ),
                                                    },
                                                )
                                                .unwrap()
                                                .into(),
                                            ))
                                            .await;
                                    }
                                }
                                MessageContent::ToolResponse(_resp) => {}
                                MessageContent::ToolConfirmationRequest(confirmation) => {
                                    let mut sender = sender.lock().await;
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(
                                                &WebSocketMessage::ToolConfirmation {
                                                    id: confirmation.id.clone(),
                                                    tool_name: confirmation
                                                        .tool_name
                                                        .to_string()
                                                        .clone(),
                                                    arguments: Value::from(
                                                        confirmation.arguments.clone(),
                                                    ),
                                                    needs_confirmation: true,
                                                },
                                            )
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await;

                                    agent.handle_confirmation(
                                        confirmation.id.clone(),
                                        goose::permission::PermissionConfirmation {
                                            principal_type: goose::permission::permission_confirmation::PrincipalType::Tool,
                                            permission: goose::permission::Permission::AllowOnce,
                                        }
                                    ).await;
                                }
                                MessageContent::Thinking(thinking) => {
                                    let mut sender = sender.lock().await;
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(&WebSocketMessage::Thinking {
                                                message: thinking.thinking.clone(),
                                            })
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await;
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(AgentEvent::HistoryReplaced(_new_messages)) => {
                        tracing::info!("History replaced, compacting happened in reply");
                    }
                    Ok(AgentEvent::McpNotification(_notification)) => {
                        tracing::info!("Received MCP notification in web interface");
                    }
                    Ok(AgentEvent::ModelChange { model, mode }) => {
                        tracing::info!("Model changed to {} in {} mode", model, mode);
                    }
                    Err(e) => {
                        error!("Error in message stream: {}", e);
                        let mut sender = sender.lock().await;
                        let _ = sender
                            .send(Message::Text(
                                serde_json::to_string(&WebSocketMessage::Error {
                                    message: format!("Error: {}", e),
                                })
                                .unwrap()
                                .into(),
                            ))
                            .await;
                        break;
                    }
                }
            }
        }
        Err(e) => {
            error!("Error calling agent: {}", e);
            let mut sender = sender.lock().await;
            let _ = sender
                .send(Message::Text(
                    serde_json::to_string(&WebSocketMessage::Error {
                        message: format!("Error: {}", e),
                    })
                    .unwrap()
                    .into(),
                ))
                .await;
        }
    }

    let mut sender = sender.lock().await;
    let _ = sender
        .send(Message::Text(
            serde_json::to_string(&WebSocketMessage::Complete {
                message: "Response complete".to_string(),
            })
            .unwrap()
            .into(),
        ))
        .await;

    Ok(())
}
