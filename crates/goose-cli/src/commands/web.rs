use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use goose::agents::{Agent, AgentEvent};
use goose::conversation::message::Message as GooseMessage;
use goose::conversation::Conversation;
use goose::session;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::error;

type SessionStore = Arc<RwLock<std::collections::HashMap<String, Arc<Mutex<Conversation>>>>>;
type CancellationStore = Arc<RwLock<std::collections::HashMap<String, tokio::task::AbortHandle>>>;

#[derive(Clone)]
struct AppState {
    agent: Arc<Agent>,
    sessions: SessionStore,
    cancellations: CancellationStore,
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

pub async fn handle_web(port: u16, host: String, open: bool) -> Result<()> {
    // Setup logging
    crate::logging::setup_logging(Some("goose-web"), None)?;

    // Load config and create agent just like the CLI does
    let config = goose::config::Config::global();

    let provider_name: String = match config.get_param("GOOSE_PROVIDER") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("No provider configured. Run 'goose configure' first");
            std::process::exit(1);
        }
    };

    let model: String = match config.get_param("GOOSE_MODEL") {
        Ok(m) => m,
        Err(_) => {
            eprintln!("No model configured. Run 'goose configure' first");
            std::process::exit(1);
        }
    };

    let model_config = goose::model::ModelConfig::new(&model)?;

    // Create the agent
    let agent = Agent::new();
    let provider = goose::providers::create(&provider_name, model_config)?;
    agent.update_provider(provider).await?;

    // Load and enable extensions from config
    let extensions = goose::config::ExtensionConfigManager::get_all()?;
    for ext_config in extensions {
        if ext_config.enabled {
            if let Err(e) = agent.add_extension(ext_config.config.clone()).await {
                eprintln!(
                    "Warning: Failed to load extension {}: {}",
                    ext_config.config.name(),
                    e
                );
            }
        }
    }

    let state = AppState {
        agent: Arc::new(agent),
        sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        cancellations: Arc::new(RwLock::new(std::collections::HashMap::new())),
    };

    // Build router
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/session/{session_name}", get(serve_session))
        .route("/ws", get(websocket_handler))
        .route("/api/health", get(health_check))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{session_id}", get(get_session))
        .route("/static/{*path}", get(serve_static))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    println!("\n🪿 Starting Goose web server");
    println!("   Provider: {} | Model: {}", provider_name, model);
    println!(
        "   Working directory: {}",
        std::env::current_dir()?.display()
    );
    println!("   Server: http://{}", addr);
    println!("   Press Ctrl+C to stop\n");

    if open {
        // Open browser
        let url = format!("http://{}", addr);
        if let Err(e) = webbrowser::open(&url) {
            eprintln!("Failed to open browser: {}", e);
        }
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}

async fn serve_session(
    axum::extract::Path(session_name): axum::extract::Path<String>,
) -> Html<String> {
    let html = include_str!("../../static/index.html");
    // Inject the session name into the HTML so JavaScript can use it
    let html_with_session = html.replace(
        "<script src=\"/static/script.js\"></script>",
        &format!(
            "<script>window.GOOSE_SESSION_NAME = '{}';</script>\n    <script src=\"/static/script.js\"></script>",
            session_name
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
    match session::list_sessions() {
        Ok(sessions) => {
            let session_info: Vec<serde_json::Value> = sessions
                .into_iter()
                .filter_map(|(name, path)| {
                    session::read_metadata(&path).ok().map(|metadata| {
                        serde_json::json!({
                            "name": name,
                            "path": path,
                            "description": metadata.description,
                            "message_count": metadata.message_count,
                            "working_dir": metadata.working_dir
                        })
                    })
                })
                .collect();

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
    let session_file = match session::get_path(session::Identifier::Name(session_id)) {
        Ok(path) => path,
        Err(e) => {
            return Json(serde_json::json!({
                "error": format!("Invalid session ID: {}", e)
            }));
        }
    };

    let error_response = |e: Box<dyn std::error::Error>| {
        Json(serde_json::json!({
            "error": e.to_string()
        }))
    };

    match session::read_messages(&session_file) {
        Ok(messages) => match session::read_metadata(&session_file) {
            Ok(metadata) => Json(serde_json::json!({
                "metadata": metadata,
                "messages": messages
            })),
            Err(e) => error_response(e.into()),
        },
        Err(e) => error_response(e.into()),
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
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
                            // Get session file path from session_id
                            let session_file = match session::get_path(session::Identifier::Name(
                                session_id.clone(),
                            )) {
                                Ok(path) => path,
                                Err(e) => {
                                    tracing::error!("Failed to get session path: {}", e);
                                    continue;
                                }
                            };

                            // Get or create session in memory (for fast access during processing)
                            let session_messages = {
                                let sessions = state.sessions.read().await;
                                if let Some(session) = sessions.get(&session_id) {
                                    session.clone()
                                } else {
                                    drop(sessions);
                                    let mut sessions = state.sessions.write().await;

                                    // Load existing messages from JSONL file if it exists
                                    let existing_messages =
                                        session::read_messages(&session_file).unwrap_or_default();

                                    let new_session = Arc::new(Mutex::new(existing_messages));
                                    sessions.insert(session_id.clone(), new_session.clone());
                                    new_session
                                }
                            };

                            // Clone sender for async processing
                            let sender_clone = sender.clone();
                            let agent = state.agent.clone();

                            // Process message in a separate task to allow streaming
                            let task_handle = tokio::spawn(async move {
                                let result = process_message_streaming(
                                    &agent,
                                    session_messages,
                                    session_file,
                                    content,
                                    sender_clone,
                                )
                                .await;

                                if let Err(e) = result {
                                    error!("Error processing message: {}", e);
                                }
                            });

                            // Store the abort handle
                            {
                                let mut cancellations = state.cancellations.write().await;
                                cancellations
                                    .insert(session_id.clone(), task_handle.abort_handle());
                            }

                            // Wait for task completion and handle abort
                            let sender_for_abort = sender.clone();
                            let session_id_for_cleanup = session_id.clone();
                            let cancellations_for_cleanup = state.cancellations.clone();

                            tokio::spawn(async move {
                                match task_handle.await {
                                    Ok(_) => {
                                        // Task completed normally
                                    }
                                    Err(e) if e.is_cancelled() => {
                                        // Task was aborted
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

                                // Clean up cancellation token
                                {
                                    let mut cancellations = cancellations_for_cleanup.write().await;
                                    cancellations.remove(&session_id_for_cleanup);
                                }
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
    session_messages: Arc<Mutex<Conversation>>,
    session_file: std::path::PathBuf,
    content: String,
    sender: Arc<Mutex<futures::stream::SplitSink<WebSocket, Message>>>,
) -> Result<()> {
    use futures::StreamExt;
    use goose::agents::SessionConfig;
    use goose::conversation::message::MessageContent;
    use goose::session;

    // Create a user message
    let user_message = GooseMessage::user().with_text(content.clone());

    // Messages will be auto-compacted in agent.reply() if needed
    let messages: Conversation = {
        let mut session_msgs = session_messages.lock().await;
        session_msgs.push(user_message.clone());
        session_msgs.clone()
    };

    // Persist messages to JSONL file with provider for automatic description generation
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

    let provider = provider.unwrap();
    let working_dir = Some(std::env::current_dir()?);
    session::persist_messages(
        &session_file,
        &messages,
        Some(provider.clone()),
        working_dir.clone(),
    )
    .await?;

    let session_config = SessionConfig {
        id: session::Identifier::Path(session_file.clone()),
        working_dir: std::env::current_dir()?,
        schedule_id: None,
        execution_mode: None,
        max_turns: None,
        retry_config: None,
    };

    match agent
        .reply(messages.clone(), Some(session_config), None)
        .await
    {
        Ok(mut stream) => {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(AgentEvent::Message(message)) => {
                        // Add message to our session
                        {
                            let mut session_msgs = session_messages.lock().await;
                            session_msgs.push(message.clone());
                        }

                        // Persist messages to JSONL file (no provider needed for assistant messages)
                        let current_messages = {
                            let session_msgs = session_messages.lock().await;
                            session_msgs.clone()
                        };
                        session::persist_messages(
                            &session_file,
                            &current_messages,
                            None,
                            working_dir.clone(),
                        )
                        .await?;
                        // Handle different message content types
                        for content in &message.content {
                            match content {
                                MessageContent::Text(text) => {
                                    // Send the text response
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
                                    // Send tool request notification
                                    let mut sender = sender.lock().await;
                                    if let Ok(tool_call) = &req.tool_call {
                                        let _ = sender
                                            .send(Message::Text(
                                                serde_json::to_string(
                                                    &WebSocketMessage::ToolRequest {
                                                        id: req.id.clone(),
                                                        tool_name: tool_call.name.clone(),
                                                        arguments: tool_call.arguments.clone(),
                                                    },
                                                )
                                                .unwrap()
                                                .into(),
                                            ))
                                            .await;
                                    }
                                }
                                MessageContent::ToolResponse(_resp) => {
                                    // Tool responses are already included in the complete message stream
                                    // and will be persisted to session history. No need to send separate
                                    // WebSocket messages as this would cause duplicates.
                                }
                                MessageContent::ToolConfirmationRequest(confirmation) => {
                                    // Send tool confirmation request
                                    let mut sender = sender.lock().await;
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(
                                                &WebSocketMessage::ToolConfirmation {
                                                    id: confirmation.id.clone(),
                                                    tool_name: confirmation.tool_name.clone(),
                                                    arguments: confirmation.arguments.clone(),
                                                    needs_confirmation: true,
                                                },
                                            )
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await;

                                    // For now, auto-approve in web mode
                                    // TODO: Implement proper confirmation UI
                                    agent.handle_confirmation(
                                        confirmation.id.clone(),
                                        goose::permission::PermissionConfirmation {
                                            principal_type: goose::permission::permission_confirmation::PrincipalType::Tool,
                                            permission: goose::permission::Permission::AllowOnce,
                                        }
                                    ).await;
                                }
                                MessageContent::Thinking(thinking) => {
                                    // Send thinking indicator
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
                                MessageContent::ContextLengthExceeded(msg) => {
                                    // Send context exceeded notification
                                    let mut sender = sender.lock().await;
                                    let _ = sender
                                        .send(Message::Text(
                                            serde_json::to_string(
                                                &WebSocketMessage::ContextExceeded {
                                                    message: msg.msg.clone(),
                                                },
                                            )
                                            .unwrap()
                                            .into(),
                                        ))
                                        .await;

                                    // For now, auto-summarize in web mode
                                    // TODO: Implement proper UI for context handling
                                    let (summarized_messages, _) =
                                        agent.summarize_context(messages.messages()).await?;
                                    {
                                        let mut session_msgs = session_messages.lock().await;
                                        *session_msgs = summarized_messages;
                                    }
                                }
                                _ => {
                                    // Handle other message types as needed
                                }
                            }
                        }
                    }
                    Ok(AgentEvent::HistoryReplaced(new_messages)) => {
                        // Replace the session's message history with the compacted messages
                        {
                            let mut session_msgs = session_messages.lock().await;
                            *session_msgs = Conversation::new_unvalidated(new_messages);
                        }

                        // Persist the updated messages to the JSONL file
                        let current_messages = {
                            let session_msgs = session_messages.lock().await;
                            session_msgs.clone()
                        };

                        if let Err(e) = session::persist_messages(
                            &session_file,
                            &current_messages,
                            None, // No provider needed for persisting
                            working_dir.clone(),
                        )
                        .await
                        {
                            error!("Failed to persist compacted messages: {}", e);
                        }
                    }
                    Ok(AgentEvent::McpNotification(_notification)) => {
                        // Handle MCP notifications if needed
                        // For now, we'll just log them
                        tracing::info!("Received MCP notification in web interface");
                    }
                    Ok(AgentEvent::ModelChange { model, mode }) => {
                        // Log model change
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

    // Send completion message
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

// Add webbrowser dependency for opening browser
use webbrowser;
