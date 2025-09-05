use agent_client_protocol::{self as acp, Client, SessionNotification};
use anyhow::Result;
use goose::agents::Agent;
use goose::config::{Config, ExtensionConfigManager};
use goose::conversation::message::{Message, MessageContent};
use goose::conversation::Conversation;
use goose::providers::create;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinSet;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use url::Url;

/// Represents a single Goose session for ACP
struct GooseSession {
    agent: Agent,
    messages: Conversation,
    tool_call_ids: HashMap<String, String>, // Maps internal tool IDs to ACP tool call IDs
    cancel_token: Option<CancellationToken>, // Active cancellation token for prompt processing
}

/// Goose ACP Agent implementation that connects to real Goose agents
struct GooseAcpAgent {
    session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    sessions: Arc<Mutex<HashMap<String, GooseSession>>>,
    provider: Arc<dyn goose::providers::base::Provider>,
}

fn read_resource_link(link: acp::ResourceLink) -> Option<String> {
    let url = Url::parse(&link.uri).ok()?;
    if url.scheme() == "file" {
        let path = url.to_file_path().ok()?;
        let contents = fs::read_to_string(&path).ok()?;

        Some(format!(
            "\n\n# {}\n```\n{}\n```",
            path.to_string_lossy(),
            contents
        ))
    } else {
        None
    }
}

impl GooseAcpAgent {
    async fn new(
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    ) -> Result<Self> {
        // Load config and create provider
        let config = Config::global();

        let provider_name: String = config
            .get_param("GOOSE_PROVIDER")
            .map_err(|e| anyhow::anyhow!("No provider configured: {}", e))?;

        let model_name: String = config
            .get_param("GOOSE_MODEL")
            .map_err(|e| anyhow::anyhow!("No model configured: {}", e))?;

        let model_config = goose::model::ModelConfig {
            model_name: model_name.clone(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            fast_model: None,
        };
        let provider = create(&provider_name, model_config)?;

        Ok(Self {
            session_update_tx,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            provider,
        })
    }
}

impl acp::Agent for GooseAcpAgent {
    async fn initialize(
        &self,
        arguments: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        info!("ACP: Received initialize request {:?}", arguments);

        // Advertise Goose's capabilities
        let agent_capabilities = acp::AgentCapabilities {
            load_session: false, // TODO: Implement session persistence
            prompt_capabilities: acp::PromptCapabilities {
                image: true,            // Goose supports image inputs via providers
                audio: false,           // TODO: Add audio support when providers support it
                embedded_context: true, // Goose can handle embedded context resources
            },
        };

        Ok(acp::InitializeResponse {
            protocol_version: acp::V1,
            agent_capabilities,
            auth_methods: Vec::new(),
        })
    }

    async fn authenticate(&self, arguments: acp::AuthenticateRequest) -> Result<(), acp::Error> {
        info!("ACP: Received authenticate request {:?}", arguments);
        Ok(())
    }

    async fn new_session(
        &self,
        arguments: acp::NewSessionRequest,
    ) -> Result<acp::NewSessionResponse, acp::Error> {
        info!("ACP: Received new session request {:?}", arguments);

        // Generate a unique session ID
        let session_id = uuid::Uuid::new_v4().to_string();

        // Create a new Agent and session for this ACP session
        let agent = Agent::new();
        agent
            .update_provider(self.provider.clone())
            .await
            .map_err(|_| acp::Error::internal_error())?;

        // Load and add extensions just like the normal CLI
        // Get all enabled extensions from configuration
        let extensions_to_run: Vec<_> = ExtensionConfigManager::get_all()
            .map_err(|e| {
                error!("Failed to load extensions: {}", e);
                acp::Error::internal_error()
            })?
            .into_iter()
            .filter(|ext| ext.enabled)
            .map(|ext| ext.config)
            .collect();

        // Add extensions to the agent in parallel
        let agent_ptr = Arc::new(agent);
        let mut set = JoinSet::new();
        let mut waiting_on = HashSet::new();

        for extension in extensions_to_run {
            waiting_on.insert(extension.name());
            let agent_ptr_clone = agent_ptr.clone();
            set.spawn(async move {
                (
                    extension.name(),
                    agent_ptr_clone.add_extension(extension.clone()).await,
                )
            });
        }

        // Wait for all extensions to load
        while let Some(result) = set.join_next().await {
            match result {
                Ok((name, Ok(_))) => {
                    waiting_on.remove(&name);
                    info!("Loaded extension: {}", name);
                }
                Ok((name, Err(e))) => {
                    warn!("Failed to load extension '{}': {}", name, e);
                    waiting_on.remove(&name);
                }
                Err(e) => {
                    error!("Task error while loading extension: {}", e);
                }
            }
        }

        // Unwrap the Arc to get the agent back
        let agent = Arc::try_unwrap(agent_ptr).map_err(|_| {
            error!("Failed to unwrap agent Arc");
            acp::Error::internal_error()
        })?;

        let session = GooseSession {
            agent,
            messages: Conversation::new_unvalidated(Vec::new()),
            tool_call_ids: HashMap::new(),
            cancel_token: None,
        };

        // Store the session
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session);

        info!("Created new session with ID: {}", session_id);

        Ok(acp::NewSessionResponse {
            session_id: acp::SessionId(session_id.into()),
        })
    }

    async fn load_session(&self, arguments: acp::LoadSessionRequest) -> Result<(), acp::Error> {
        info!("ACP: Received load session request {:?}", arguments);
        // For now, will start a new session. We could use goose session storage as an enhancement
        // we would need to map ACP session IDs to goose session ids (which by default are auto generated)
        // normal goose session restore in CLI doesn't load conversation visually.
        //
        // Example flow:
        // - Load session file by session_id (might need to map ACP session IDs to Goose session paths)
        // - For each message in history:
        //   - If user message: send user_message_chunk notification
        //   - If assistant message: send agent_message_chunk notification
        //   - If tool calls/responses: send appropriate notifications

        // For now, we don't support loading previous sessions
        Err(acp::Error::method_not_found())
    }

    #[allow(clippy::too_many_lines)]
    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        info!("ACP: Received prompt request {:?}", arguments);

        // Get the session
        let session_id = arguments.session_id.0.to_string();
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(acp::Error::invalid_params)?;

        // Convert ACP prompt to Goose message
        let mut user_message = Message::user();

        // Process all content blocks from the prompt
        for block in arguments.prompt {
            match block {
                acp::ContentBlock::Text(text) => {
                    user_message = user_message.with_text(&text.text);
                }
                acp::ContentBlock::Image(image) => {
                    // Goose supports images via base64 encoded data
                    // The ACP ImageContent has data as a String directly
                    user_message = user_message.with_image(&image.data, &image.mime_type);
                }
                acp::ContentBlock::Resource(resource) => {
                    // Embed resource content as text with context
                    match &resource.resource {
                        acp::EmbeddedResourceResource::TextResourceContents(text_resource) => {
                            let header = format!("--- Resource: {} ---\n", text_resource.uri);
                            let content = format!("{}{}\n---\n", header, text_resource.text);
                            user_message = user_message.with_text(&content);
                        }
                        _ => {
                            // Ignore non-text resources for now
                        }
                    }
                }
                acp::ContentBlock::ResourceLink(link) => {
                    if let Some(text) = read_resource_link(link) {
                        user_message = user_message.with_text(text)
                    }
                }
                acp::ContentBlock::Audio(..) => (),
            }
        }

        // Add message to conversation
        session.messages.push(user_message);

        // Create and store cancellation token for this prompt
        let cancel_token = CancellationToken::new();
        session.cancel_token = Some(cancel_token.clone());

        // Get agent's reply through the Goose agent
        let mut stream = session
            .agent
            .reply(session.messages.clone(), None, Some(cancel_token.clone()))
            .await
            .map_err(|e| {
                error!("Error getting agent reply: {}", e);
                acp::Error::internal_error()
            })?;

        use futures::StreamExt;

        // Track if we were cancelled
        let mut was_cancelled = false;

        // Process the agent's response stream
        while let Some(event) = stream.next().await {
            // Check if we've been cancelled
            if cancel_token.is_cancelled() {
                was_cancelled = true;
                break;
            }

            match event {
                Ok(goose::agents::AgentEvent::Message(message)) => {
                    // Add to conversation
                    session.messages.push(message.clone());

                    // Process message content, including tool calls
                    for content_item in &message.content {
                        match content_item {
                            MessageContent::Text(text) => {
                                // Stream text to the client
                                let (tx, rx) = oneshot::channel();
                                self.session_update_tx
                                    .send((
                                        SessionNotification {
                                            session_id: arguments.session_id.clone(),
                                            update: acp::SessionUpdate::AgentMessageChunk {
                                                content: text.text.clone().into(),
                                            },
                                        },
                                        tx,
                                    ))
                                    .map_err(|_| acp::Error::internal_error())?;
                                rx.await.map_err(|_| acp::Error::internal_error())?;
                            }
                            MessageContent::ToolRequest(tool_request) => {
                                // Generate ACP tool call ID and track mapping
                                let acp_tool_id = format!("tool_{}", uuid::Uuid::new_v4());
                                session
                                    .tool_call_ids
                                    .insert(tool_request.id.clone(), acp_tool_id.clone());

                                // Extract tool name and parameters from the ToolCall if successful
                                let (tool_name, locations) = match &tool_request.tool_call {
                                    Ok(tool_call) => {
                                        let name = tool_call.name.clone();

                                        // Extract file locations from certain tools for client tracking
                                        let mut locs = Vec::new();
                                        if name == "developer__text_editor" {
                                            // Try to extract the path from the arguments
                                            let args = &tool_call.arguments;
                                            if let Some(path_str) =
                                                args.get("path").and_then(|p| p.as_str())
                                            {
                                                locs.push(acp::ToolCallLocation {
                                                    path: path_str.into(),
                                                    line: Some(1),
                                                });
                                            }
                                        }
                                        (name, locs)
                                    }
                                    Err(_) => ("unknown".to_string(), Vec::new()),
                                };

                                // Send tool call notification
                                let (tx, rx) = oneshot::channel();
                                self.session_update_tx
                                    .send((
                                        SessionNotification {
                                            session_id: arguments.session_id.clone(),
                                            update: acp::SessionUpdate::ToolCall(acp::ToolCall {
                                                id: acp::ToolCallId(acp_tool_id.clone().into()),
                                                title: format!("Calling tool: {}", tool_name),
                                                kind: acp::ToolKind::default(),
                                                status: acp::ToolCallStatus::Pending,
                                                content: Vec::new(),
                                                locations,
                                                raw_input: None,
                                                raw_output: None,
                                            }),
                                        },
                                        tx,
                                    ))
                                    .map_err(|_| acp::Error::internal_error())?;
                                rx.await.map_err(|_| acp::Error::internal_error())?;

                                // No need for separate update - status is already set to Pending
                            }
                            MessageContent::ToolResponse(tool_response) => {
                                // Look up the ACP tool call ID
                                if let Some(acp_tool_id) =
                                    session.tool_call_ids.get(&tool_response.id)
                                {
                                    // Determine if the tool call succeeded or failed
                                    let status = if tool_response.tool_result.is_ok() {
                                        acp::ToolCallStatus::Completed
                                    } else {
                                        acp::ToolCallStatus::Failed
                                    };

                                    // Send status update (completed or failed)
                                    let (tx, rx) = oneshot::channel();
                                    self.session_update_tx
                                        .send((
                                            SessionNotification {
                                                session_id: arguments.session_id.clone(),
                                                update: acp::SessionUpdate::ToolCallUpdate(
                                                    acp::ToolCallUpdate {
                                                        id: acp::ToolCallId(
                                                            acp_tool_id.clone().into(),
                                                        ),
                                                        fields: acp::ToolCallUpdateFields {
                                                            status: Some(status),
                                                            ..Default::default()
                                                        },
                                                    },
                                                ),
                                            },
                                            tx,
                                        ))
                                        .map_err(|_| acp::Error::internal_error())?;
                                    rx.await.map_err(|_| acp::Error::internal_error())?;
                                }
                            }
                            MessageContent::Thinking(thinking) => {
                                // Stream thinking/reasoning content as thought chunks
                                let (tx, rx) = oneshot::channel();
                                self.session_update_tx
                                    .send((
                                        SessionNotification {
                                            session_id: arguments.session_id.clone(),
                                            update: acp::SessionUpdate::AgentThoughtChunk {
                                                content: thinking.thinking.clone().into(),
                                            },
                                        },
                                        tx,
                                    ))
                                    .map_err(|_| acp::Error::internal_error())?;
                                rx.await.map_err(|_| acp::Error::internal_error())?;
                            }
                            _ => {
                                // Ignore other content types for now
                            }
                        }
                    }
                }
                Ok(_) => {
                    // Ignore other events for now
                }
                Err(e) => {
                    error!("Error in agent response stream: {}", e);
                    return Err(acp::Error::internal_error());
                }
            }
        }

        // Clear the cancel token since we're done
        session.cancel_token = None;

        Ok(acp::PromptResponse {
            stop_reason: if was_cancelled {
                acp::StopReason::Cancelled
            } else {
                acp::StopReason::EndTurn
            },
        })
    }

    async fn cancel(&self, args: acp::CancelNotification) -> Result<(), acp::Error> {
        info!("ACP: Received cancel request {:?}", args);

        // Get the session and cancel its active operation
        let session_id = args.session_id.0.to_string();
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get_mut(&session_id) {
            if let Some(ref token) = session.cancel_token {
                info!("Cancelling active prompt for session {}", session_id);
                token.cancel();
            }
        } else {
            warn!("Cancel request for non-existent session: {}", session_id);
        }

        Ok(())
    }
}

/// Run the ACP agent server
pub async fn run_acp_agent() -> Result<()> {
    info!("Starting Goose ACP agent server on stdio");
    eprintln!("Goose ACP agent started. Listening on stdio...");

    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    // The AgentSideConnection will spawn futures onto our Tokio runtime.
    // LocalSet and spawn_local are used because the futures from the
    // agent-client-protocol crate are not Send.
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            // Start up the GooseAcpAgent connected to stdio.
            let agent = GooseAcpAgent::new(tx)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create ACP agent: {}", e))?;
            let (conn, handle_io) =
                acp::AgentSideConnection::new(agent, outgoing, incoming, |fut| {
                    tokio::task::spawn_local(fut);
                });

            // Kick off a background task to send the agent's session notifications to the client.
            tokio::task::spawn_local(async move {
                while let Some((session_notification, tx)) = rx.recv().await {
                    let result = conn.session_notification(session_notification).await;
                    if let Err(e) = result {
                        error!("ACP session notification error: {}", e);
                        break;
                    }
                    tx.send(()).ok();
                }
            });

            // Run until stdin/stdout are closed.
            handle_io.await
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use agent_client_protocol::ResourceLink;
    use std::io::Write;
    use tempfile::NamedTempFile;

    use crate::commands::acp::read_resource_link;

    fn new_resource_link(content: &str) -> anyhow::Result<(ResourceLink, NamedTempFile)> {
        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;

        let link = ResourceLink {
            annotations: None,
            description: None,
            mime_type: None,
            name: file
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            size: None,
            title: None,
            uri: format!("file://{}", file.path().to_str().unwrap()),
        };
        Ok((link, file))
    }

    #[test]
    fn test_read_resource_link_non_file_scheme() {
        let (link, file) = new_resource_link("print(\"hello, world\")").unwrap();

        let result = read_resource_link(link).unwrap();
        let expected = format!(
            "

# {}
```
print(\"hello, world\")
```",
            file.path().to_str().unwrap(),
        );

        assert_eq!(result, expected,)
    }
}
