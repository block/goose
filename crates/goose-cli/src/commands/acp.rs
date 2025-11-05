use agent_client_protocol::{
    self as acp, Client, EmbeddedResource, ImageContent, SessionNotification, TextContent,
    ToolCallContent,
};
use anyhow::Result;
use goose::agents::{Agent, SessionConfig};
use goose::config::{get_all_extensions, Config};
use goose::conversation::message::{Message, MessageContent};
use goose::conversation::Conversation;
use goose::providers::create;
use goose::session::session_manager::SessionType;
use goose::session::SessionManager;
use rmcp::model::{RawContent, ResourceContents};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinSet;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use url::Url;

struct GooseAcpSession {
    messages: Conversation,
    tool_call_ids: HashMap<String, String>, // Maps internal tool IDs to ACP tool call IDs
    cancel_token: Option<CancellationToken>, // Active cancellation token for prompt processing
}

struct GooseAcpAgent {
    session_update_tx: mpsc::UnboundedSender<(SessionNotification, oneshot::Sender<()>)>,
    sessions: Arc<Mutex<HashMap<String, GooseAcpSession>>>,
    agent: Agent, // Shared agent instance
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

/// Format a tool name to be more human-friendly by splitting extension and tool names
/// and converting underscores to spaces with proper capitalization
fn format_tool_name(tool_name: &str) -> String {
    // Split on double underscore to separate extension from tool name
    if let Some((extension, tool)) = tool_name.split_once("__") {
        let formatted_extension = extension.replace('_', " ");
        let formatted_tool = tool.replace('_', " ");

        // Capitalize first letter of each word
        let capitalize = |s: &str| {
            s.split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        };

        format!(
            "{}: {}",
            capitalize(&formatted_extension),
            capitalize(&formatted_tool)
        )
    } else {
        // Fallback for tools without double underscore
        let formatted = tool_name.replace('_', " ");
        formatted
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl GooseAcpAgent {
    async fn new(
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    ) -> Result<Self> {
        let config = Config::global();

        let provider_name: String = config
            .get_goose_provider()
            .map_err(|e| anyhow::anyhow!("No provider configured: {}", e))?;

        let model_name: String = config
            .get_goose_model()
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
        let provider = create(&provider_name, model_config).await?;

        // Create a shared agent instance
        let agent = Agent::new();
        agent.update_provider(provider.clone()).await?;

        // Load and add extensions just like the normal CLI
        let extensions_to_run: Vec<_> = get_all_extensions()
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
        let agent = Arc::try_unwrap(agent_ptr)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap agent Arc"))?;

        Ok(Self {
            session_update_tx,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            agent,
        })
    }

    fn convert_acp_prompt_to_message(&self, prompt: Vec<acp::ContentBlock>) -> Message {
        let mut user_message = Message::user();

        // Process all content blocks from the prompt
        for block in prompt {
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

        user_message
    }

    async fn handle_message_content(
        &self,
        content_item: &MessageContent,
        session_id: &acp::SessionId,
        session: &mut GooseAcpSession,
    ) -> Result<(), acp::Error> {
        match content_item {
            MessageContent::Text(text) => {
                // Stream text to the client
                let (tx, rx) = oneshot::channel();
                self.session_update_tx
                    .send((
                        SessionNotification {
                            session_id: session_id.clone(),
                            update: acp::SessionUpdate::AgentMessageChunk {
                                content: text.text.clone().into(),
                            },
                            meta: None,
                        },
                        tx,
                    ))
                    .map_err(|_| acp::Error::internal_error())?;
                rx.await.map_err(|_| acp::Error::internal_error())?;
            }
            MessageContent::ToolRequest(tool_request) => {
                self.handle_tool_request(tool_request, session_id, session)
                    .await?;
            }
            MessageContent::ToolResponse(tool_response) => {
                self.handle_tool_response(tool_response, session_id, session)
                    .await?;
            }
            MessageContent::Thinking(thinking) => {
                // Stream thinking/reasoning content as thought chunks
                let (tx, rx) = oneshot::channel();
                self.session_update_tx
                    .send((
                        SessionNotification {
                            session_id: session_id.clone(),
                            update: acp::SessionUpdate::AgentThoughtChunk {
                                content: thinking.thinking.clone().into(),
                            },
                            meta: None,
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
        Ok(())
    }

    async fn handle_tool_request(
        &self,
        tool_request: &goose::conversation::message::ToolRequest,
        session_id: &acp::SessionId,
        session: &mut GooseAcpSession,
    ) -> Result<(), acp::Error> {
        // Generate ACP tool call ID and track mapping
        let acp_tool_id = format!("tool_{}", uuid::Uuid::new_v4());
        session
            .tool_call_ids
            .insert(tool_request.id.clone(), acp_tool_id.clone());

        // Extract tool name and parameters from the ToolCall if successful
        let (tool_name, locations) = match &tool_request.tool_call {
            Ok(tool_call) => {
                // Extract file locations from certain tools for client tracking
                let mut locs = Vec::new();
                if tool_call.name == "developer__text_editor" {
                    // Try to extract the path from the arguments
                    if let Some(path_str) = tool_call
                        .arguments
                        .as_ref()
                        .and_then(|args_map| args_map.get("path"))
                        .and_then(|p| p.as_str())
                    {
                        let path = std::path::PathBuf::from(path_str);
                        if path.exists() && path.is_file() {
                            locs.push(acp::ToolCallLocation {
                                path: path_str.into(),
                                line: Some(1),
                                meta: None,
                            });
                        }
                    }
                }

                (tool_call.name.to_string(), locs)
            }
            Err(_) => ("error".to_string(), vec![]),
        };

        // Send tool call notification
        let (tx, rx) = oneshot::channel();
        self.session_update_tx
            .send((
                SessionNotification {
                    session_id: session_id.clone(),
                    update: acp::SessionUpdate::ToolCall(acp::ToolCall {
                        id: acp::ToolCallId(acp_tool_id.clone().into()),
                        title: format_tool_name(&tool_name),
                        kind: acp::ToolKind::default(),
                        status: acp::ToolCallStatus::Pending,
                        content: Vec::new(),
                        locations,
                        raw_input: None,
                        raw_output: None,
                        meta: None,
                    }),
                    meta: None,
                },
                tx,
            ))
            .map_err(|_| acp::Error::internal_error())?;
        rx.await.map_err(|_| acp::Error::internal_error())?;

        Ok(())
    }

    async fn handle_tool_response(
        &self,
        tool_response: &goose::conversation::message::ToolResponse,
        session_id: &acp::SessionId,
        session: &mut GooseAcpSession,
    ) -> Result<(), acp::Error> {
        // Look up the ACP tool call ID
        if let Some(acp_tool_id) = session.tool_call_ids.get(&tool_response.id) {
            // Determine if the tool call succeeded or failed
            let status = if tool_response.tool_result.is_ok() {
                acp::ToolCallStatus::Completed
            } else {
                acp::ToolCallStatus::Failed
            };

            let content: Vec<ToolCallContent> = match &tool_response.tool_result {
                Ok(content_items) => content_items
                    .iter()
                    .filter_map(|content| match &content.raw {
                        RawContent::Text(val) => Some(ToolCallContent::Content {
                            content: acp::ContentBlock::Text(TextContent {
                                annotations: None,
                                text: val.text.clone(),
                                meta: None,
                            }),
                        }),
                        RawContent::Image(val) => Some(ToolCallContent::Content {
                            content: acp::ContentBlock::Image(ImageContent {
                                annotations: None,
                                data: val.data.clone(),
                                mime_type: val.mime_type.clone(),
                                uri: None,
                                meta: None,
                            }),
                        }),
                        RawContent::Resource(val) => Some(ToolCallContent::Content {
                            content: acp::ContentBlock::Resource(EmbeddedResource {
                                annotations: None,
                                resource: match &val.resource {
                                    ResourceContents::TextResourceContents {
                                        mime_type,
                                        text,
                                        uri,
                                        ..
                                    } => acp::EmbeddedResourceResource::TextResourceContents(
                                        acp::TextResourceContents {
                                            mime_type: mime_type.clone(),
                                            text: text.clone(),
                                            uri: uri.clone(),
                                            meta: None,
                                        },
                                    ),
                                    ResourceContents::BlobResourceContents {
                                        mime_type,
                                        blob,
                                        uri,
                                        ..
                                    } => acp::EmbeddedResourceResource::BlobResourceContents(
                                        acp::BlobResourceContents {
                                            mime_type: mime_type.clone(),
                                            blob: blob.clone(),
                                            uri: uri.clone(),
                                            meta: None,
                                        },
                                    ),
                                },
                                meta: None,
                            }),
                        }),
                        RawContent::Audio(_) => {
                            // Audio content is not supported in ACP ContentBlock, skip it
                            None
                        }
                        RawContent::ResourceLink(_) => {
                            // ResourceLink content is not supported in ACP ContentBlock, skip it
                            None
                        }
                    })
                    .collect(),
                Err(_) => Vec::new(),
            };

            // Send status update (completed or failed)
            let (tx, rx) = oneshot::channel();
            self.session_update_tx
                .send((
                    SessionNotification {
                        session_id: session_id.clone(),
                        update: acp::SessionUpdate::ToolCallUpdate(acp::ToolCallUpdate {
                            id: acp::ToolCallId(acp_tool_id.clone().into()),
                            fields: acp::ToolCallUpdateFields {
                                status: Some(status),
                                content: Some(content),
                                ..Default::default()
                            },
                            meta: None,
                        }),
                        meta: None,
                    },
                    tx,
                ))
                .map_err(|_| acp::Error::internal_error())?;
            rx.await.map_err(|_| acp::Error::internal_error())?;
        }

        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Agent for GooseAcpAgent {
    async fn initialize(
        &self,
        args: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        info!("ACP: Received initialize request {:?}", args);

        // Advertise Goose's capabilities
        let agent_capabilities = acp::AgentCapabilities {
            load_session: false, // TODO: Implement session persistence
            prompt_capabilities: acp::PromptCapabilities {
                image: true,            // Goose supports image inputs via providers
                audio: false,           // TODO: Add audio support when providers support it
                embedded_context: true, // Goose can handle embedded context resources
                meta: None,
            },
            mcp_capabilities: acp::McpCapabilities {
                http: false, // TODO: Add MCP HTTP support if needed
                sse: false,  // TODO: Add MCP SSE support if needed
                meta: None,
            },
            meta: None,
        };

        Ok(acp::InitializeResponse {
            protocol_version: acp::V1,
            agent_capabilities,
            auth_methods: Vec::new(),
            meta: None,
        })
    }

    async fn authenticate(
        &self,
        args: acp::AuthenticateRequest,
    ) -> Result<acp::AuthenticateResponse, acp::Error> {
        info!("ACP: Received authenticate request {:?}", args);
        Ok(acp::AuthenticateResponse { meta: None })
    }

    async fn new_session(
        &self,
        args: acp::NewSessionRequest,
    ) -> Result<acp::NewSessionResponse, acp::Error> {
        info!("ACP: Received new session request {:?}", args);

        // Generate a unique session ID
        let session_id = uuid::Uuid::new_v4().to_string();

        let session = GooseAcpSession {
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
            modes: None, // TODO: Implement session modes if needed
            meta: None,
        })
    }

    async fn load_session(
        &self,
        args: acp::LoadSessionRequest,
    ) -> Result<acp::LoadSessionResponse, acp::Error> {
        info!("ACP: Received load session request {:?}", args);
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

    async fn prompt(&self, args: acp::PromptRequest) -> Result<acp::PromptResponse, acp::Error> {
        info!("ACP: Received prompt request {:?}", args);

        // Get the session
        let session_id = args.session_id.0.to_string();

        // Create and store cancellation token for this prompt
        let cancel_token = CancellationToken::new();
        {
            let mut sessions = self.sessions.lock().await;
            let session = sessions
                .get_mut(&session_id)
                .ok_or_else(acp::Error::invalid_params)?;
            session.cancel_token = Some(cancel_token.clone());
        }

        let user_message = self.convert_acp_prompt_to_message(args.prompt);

        let session = SessionManager::create_session(
            std::env::current_dir().unwrap_or_default(),
            "ACP Session".to_string(),
            SessionType::Hidden,
        )
        .await?;

        let session_config = SessionConfig {
            id: session.id.clone(),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        // Get agent's reply through the Goose agent
        let mut stream = self
            .agent
            .reply(user_message, session_config, Some(cancel_token.clone()))
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
                    // Re-acquire the lock to add message to conversation
                    let mut sessions = self.sessions.lock().await;
                    let session = sessions
                        .get_mut(&session_id)
                        .ok_or_else(acp::Error::invalid_params)?;

                    // Add to conversation
                    session.messages.push(message.clone());

                    // Process message content, including tool calls
                    for content_item in &message.content {
                        self.handle_message_content(content_item, &args.session_id, session)
                            .await?;
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
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.cancel_token = None;
        }

        Ok(acp::PromptResponse {
            stop_reason: if was_cancelled {
                acp::StopReason::Cancelled
            } else {
                acp::StopReason::EndTurn
            },
            meta: None,
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

    async fn set_session_mode(
        &self,
        _args: acp::SetSessionModeRequest,
    ) -> Result<acp::SetSessionModeResponse, acp::Error> {
        // TODO: Implement session modes if needed
        Err(acp::Error::method_not_found())
    }

    async fn ext_method(
        &self,
        _args: acp::ExtRequest,
    ) -> Result<std::sync::Arc<acp::RawValue>, acp::Error> {
        // TODO: Implement extension methods if needed
        Err(acp::Error::method_not_found())
    }

    async fn ext_notification(&self, _args: acp::ExtNotification) -> Result<(), acp::Error> {
        // TODO: Implement extension notifications if needed
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

    use crate::commands::acp::{format_tool_name, read_resource_link};

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
            meta: None,
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

    #[test]
    fn test_format_tool_name_with_extension() {
        assert_eq!(
            format_tool_name("developer__text_editor"),
            "Developer: Text Editor"
        );
        assert_eq!(
            format_tool_name("platform__manage_extensions"),
            "Platform: Manage Extensions"
        );
        assert_eq!(format_tool_name("todo__read"), "Todo: Read");
    }

    #[test]
    fn test_format_tool_name_without_extension() {
        assert_eq!(format_tool_name("simple_tool"), "Simple Tool");
        assert_eq!(format_tool_name("another_name"), "Another Name");
        assert_eq!(format_tool_name("single"), "Single");
    }

    #[test]
    fn test_format_tool_name_edge_cases() {
        assert_eq!(format_tool_name(""), "");
        assert_eq!(format_tool_name("__"), ": ");
        assert_eq!(format_tool_name("extension__"), "Extension: ");
        assert_eq!(format_tool_name("__tool"), ": Tool");
    }
}
