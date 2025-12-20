use anyhow::Result;
use goose::agents::{Agent, SessionConfig};
use goose::config::{get_all_extensions, Config};
use goose::conversation::message::{Message, MessageContent};
use goose::conversation::Conversation;
use goose::mcp_utils::ToolResult;
use goose::providers::create;
use goose::session::session_manager::SessionType;
use goose::session::SessionManager;
use rmcp::model::{CallToolResult, RawContent, ResourceContents, Role};
use sacp::schema::{
    AgentCapabilities, AuthenticateRequest, AuthenticateResponse, BlobResourceContents,
    CancelNotification, ContentBlock, ContentChunk, EmbeddedResource, EmbeddedResourceResource,
    ImageContent, InitializeRequest, InitializeResponse, LoadSessionRequest, LoadSessionResponse,
    McpCapabilities, NewSessionRequest, NewSessionResponse, PromptCapabilities, PromptRequest,
    PromptResponse, ResourceLink, SessionId, SessionNotification, SessionUpdate, StopReason,
    TextContent, TextResourceContents, ToolCall, ToolCallContent, ToolCallId, ToolCallLocation,
    ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields, ToolKind,
};
use sacp::{AgentToClient, ByteStreams, Handled, JrConnectionCx, JrMessageHandler, MessageCx};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use url::Url;

struct GooseAcpSession {
    messages: Conversation,
    tool_call_ids: HashMap<String, String>, // Maps internal tool IDs to ACP tool call IDs
    tool_requests: HashMap<String, goose::conversation::message::ToolRequest>, // Store tool requests by ID for location extraction
    cancel_token: Option<CancellationToken>, // Active cancellation token for prompt processing
}

struct GooseAcpAgent {
    sessions: Arc<Mutex<HashMap<String, GooseAcpSession>>>,
    agent: Agent, // Shared agent instance
}

/// Create a ToolCallLocation with common defaults
fn create_tool_location(path: &str, line: Option<u32>) -> ToolCallLocation {
    ToolCallLocation {
        path: path.into(),
        line,
        meta: None,
    }
}

/// Extract file locations from tool request and response
fn extract_tool_locations(
    tool_request: &goose::conversation::message::ToolRequest,
    tool_response: &goose::conversation::message::ToolResponse,
) -> Vec<ToolCallLocation> {
    let mut locations = Vec::new();

    // Get the tool call details
    if let Ok(tool_call) = &tool_request.tool_call {
        // Only process text_editor tool
        if tool_call.name != "developer__text_editor" {
            return locations;
        }

        // Extract the path from arguments
        let path_str = tool_call
            .arguments
            .as_ref()
            .and_then(|args| args.get("path"))
            .and_then(|p| p.as_str());

        if let Some(path_str) = path_str {
            // Get the command type
            let command = tool_call
                .arguments
                .as_ref()
                .and_then(|args| args.get("command"))
                .and_then(|c| c.as_str());

            // Extract line numbers from the response content
            if let Ok(result) = &tool_response.tool_result {
                for content in &result.content {
                    if let RawContent::Text(text_content) = &content.raw {
                        let text = &text_content.text;

                        // Parse line numbers based on command type and response format
                        match command {
                            Some("view") => {
                                // For view command, look for "lines X-Y" pattern in header
                                let line = extract_view_line_range(text)
                                    .map(|range| range.0 as u32)
                                    .or(Some(1));
                                locations.push(create_tool_location(path_str, line));
                            }
                            Some("str_replace") | Some("insert") => {
                                // For edits, extract the first line number from the snippet
                                let line = extract_first_line_number(text)
                                    .map(|l| l as u32)
                                    .or(Some(1));
                                locations.push(create_tool_location(path_str, line));
                            }
                            Some("write") => {
                                // For write, just point to the beginning of the file
                                locations.push(create_tool_location(path_str, Some(1)));
                            }
                            _ => {
                                // For other commands or unknown, default to line 1
                                locations.push(create_tool_location(path_str, Some(1)));
                            }
                        }
                        break; // Only process first text content
                    }
                }
            }

            // If we didn't find any locations yet, add a default one
            if locations.is_empty() {
                locations.push(create_tool_location(path_str, Some(1)));
            }
        }
    }

    locations
}

/// Extract line range from view command output (e.g., "### path/to/file.rs (lines 10-20)")
fn extract_view_line_range(text: &str) -> Option<(usize, usize)> {
    // Look for pattern like "(lines X-Y)" or "(lines X-end)"
    let re = regex::Regex::new(r"\(lines (\d+)-(\d+|end)\)").ok()?;
    if let Some(caps) = re.captures(text) {
        let start = caps.get(1)?.as_str().parse::<usize>().ok()?;
        let end = if caps.get(2)?.as_str() == "end" {
            start // Use start as a reasonable default
        } else {
            caps.get(2)?.as_str().parse::<usize>().ok()?
        };
        return Some((start, end));
    }
    None
}

/// Extract the first line number from code snippet (e.g., "123: some code")
fn extract_first_line_number(text: &str) -> Option<usize> {
    // Look for pattern like "123: " at the start of a line within a code block
    let re = regex::Regex::new(r"```[^\n]*\n(\d+):").ok()?;
    if let Some(caps) = re.captures(text) {
        return caps.get(1)?.as_str().parse::<usize>().ok();
    }
    None
}

fn read_resource_link(link: ResourceLink) -> Option<String> {
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
    async fn new() -> Result<Self> {
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

        let session = SessionManager::create_session(
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            "ACP Session".to_string(),
            SessionType::Hidden,
        )
        .await?;

        let agent = Agent::new();
        agent.update_provider(provider.clone(), &session.id).await?;

        let extensions_to_run: Vec<_> = get_all_extensions()
            .into_iter()
            .filter(|ext| ext.enabled)
            .map(|ext| ext.config)
            .collect();

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

        let agent = Arc::try_unwrap(agent_ptr)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap agent Arc"))?;

        Ok(Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            agent,
        })
    }

    fn convert_acp_prompt_to_message(&self, prompt: Vec<ContentBlock>) -> Message {
        let mut user_message = Message::user();

        // Process all content blocks from the prompt
        for block in prompt {
            match block {
                ContentBlock::Text(text) => {
                    user_message = user_message.with_text(&text.text);
                }
                ContentBlock::Image(image) => {
                    // Goose supports images via base64 encoded data
                    // The ACP ImageContent has data as a String directly
                    user_message = user_message.with_image(&image.data, &image.mime_type);
                }
                ContentBlock::Resource(resource) => {
                    // Embed resource content as text with context
                    match &resource.resource {
                        EmbeddedResourceResource::TextResourceContents(text_resource) => {
                            let header = format!("--- Resource: {} ---\n", text_resource.uri);
                            let content = format!("{}{}\n---\n", header, text_resource.text);
                            user_message = user_message.with_text(&content);
                        }
                        _ => {
                            // Ignore non-text resources for now
                        }
                    }
                }
                ContentBlock::ResourceLink(link) => {
                    if let Some(text) = read_resource_link(link) {
                        user_message = user_message.with_text(text)
                    }
                }
                ContentBlock::Audio(..) => (),
            }
        }

        user_message
    }

    async fn handle_message_content(
        &self,
        content_item: &MessageContent,
        session_id: &SessionId,
        session: &mut GooseAcpSession,
        cx: &JrConnectionCx<AgentToClient>,
    ) -> Result<(), sacp::Error> {
        match content_item {
            MessageContent::Text(text) => {
                // Stream text to the client
                cx.send_notification(SessionNotification {
                    session_id: session_id.clone(),
                    update: SessionUpdate::AgentMessageChunk(ContentChunk {
                        content: ContentBlock::Text(TextContent {
                            text: text.text.clone(),
                            annotations: None,
                            meta: None,
                        }),
                        meta: None,
                    }),
                    meta: None,
                })?;
            }
            MessageContent::ToolRequest(tool_request) => {
                self.handle_tool_request(tool_request, session_id, session, cx)
                    .await?;
            }
            MessageContent::ToolResponse(tool_response) => {
                self.handle_tool_response(tool_response, session_id, session, cx)
                    .await?;
            }
            MessageContent::Thinking(thinking) => {
                // Stream thinking/reasoning content as thought chunks
                cx.send_notification(SessionNotification {
                    session_id: session_id.clone(),
                    update: SessionUpdate::AgentThoughtChunk(ContentChunk {
                        content: ContentBlock::Text(TextContent {
                            text: thinking.thinking.clone(),
                            annotations: None,
                            meta: None,
                        }),
                        meta: None,
                    }),
                    meta: None,
                })?;
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
        session_id: &SessionId,
        session: &mut GooseAcpSession,
        cx: &JrConnectionCx<AgentToClient>,
    ) -> Result<(), sacp::Error> {
        // Generate ACP tool call ID and track mapping
        let acp_tool_id = format!("tool_{}", uuid::Uuid::new_v4());
        session
            .tool_call_ids
            .insert(tool_request.id.clone(), acp_tool_id.clone());

        // Store the tool request for later use in response handling
        session
            .tool_requests
            .insert(tool_request.id.clone(), tool_request.clone());

        // Extract tool name from the ToolCall if successful
        let tool_name = match &tool_request.tool_call {
            Ok(tool_call) => tool_call.name.to_string(),
            Err(_) => "error".to_string(),
        };

        // Send tool call notification with empty locations initially
        // We'll update with real locations when we get the response
        cx.send_notification(SessionNotification {
            session_id: session_id.clone(),
            update: SessionUpdate::ToolCall(ToolCall {
                id: ToolCallId(acp_tool_id.clone().into()),
                title: format_tool_name(&tool_name),
                kind: ToolKind::default(),
                status: ToolCallStatus::Pending,
                content: vec![],
                locations: vec![],
                raw_input: None,
                raw_output: None,
                meta: None,
            }),
            meta: None,
        })?;

        Ok(())
    }

    async fn handle_tool_response(
        &self,
        tool_response: &goose::conversation::message::ToolResponse,
        session_id: &SessionId,
        session: &mut GooseAcpSession,
        cx: &JrConnectionCx<AgentToClient>,
    ) -> Result<(), sacp::Error> {
        // Look up the ACP tool call ID
        if let Some(acp_tool_id) = session.tool_call_ids.get(&tool_response.id) {
            // Determine if the tool call succeeded or failed
            let status = if tool_response.tool_result.is_ok() {
                ToolCallStatus::Completed
            } else {
                ToolCallStatus::Failed
            };

            let content = build_tool_call_content(&tool_response.tool_result);

            // Extract locations from the tool request and response
            let locations = if let Some(tool_request) = session.tool_requests.get(&tool_response.id)
            {
                extract_tool_locations(tool_request, tool_response)
            } else {
                Vec::new()
            };

            // Send status update (completed or failed) with locations
            cx.send_notification(SessionNotification {
                session_id: session_id.clone(),
                update: SessionUpdate::ToolCallUpdate(ToolCallUpdate {
                    id: ToolCallId(acp_tool_id.clone().into()),
                    fields: ToolCallUpdateFields {
                        status: Some(status),
                        content: Some(content),
                        locations: if locations.is_empty() {
                            None
                        } else {
                            Some(locations)
                        },
                        title: None,
                        kind: None,
                        raw_input: None,
                        raw_output: None,
                    },
                    meta: None,
                }),
                meta: None,
            })?;
        }

        Ok(())
    }
}

/// Build tool call content from tool result
fn build_tool_call_content(tool_result: &ToolResult<CallToolResult>) -> Vec<ToolCallContent> {
    match tool_result {
        Ok(result) => result
            .content
            .iter()
            .filter_map(|content| match &content.raw {
                RawContent::Text(val) => Some(ToolCallContent::Content {
                    content: ContentBlock::Text(TextContent {
                        text: val.text.clone(),
                        annotations: None,
                        meta: None,
                    }),
                }),
                RawContent::Image(val) => Some(ToolCallContent::Content {
                    content: ContentBlock::Image(ImageContent {
                        data: val.data.clone(),
                        mime_type: val.mime_type.clone(),
                        uri: None,
                        annotations: None,
                        meta: None,
                    }),
                }),
                RawContent::Resource(val) => Some(ToolCallContent::Content {
                    content: ContentBlock::Resource(EmbeddedResource {
                        resource: match &val.resource {
                            ResourceContents::TextResourceContents {
                                mime_type,
                                text,
                                uri,
                                ..
                            } => EmbeddedResourceResource::TextResourceContents(
                                TextResourceContents {
                                    text: text.clone(),
                                    uri: uri.clone(),
                                    mime_type: mime_type.clone(),
                                    meta: None,
                                },
                            ),
                            ResourceContents::BlobResourceContents {
                                mime_type,
                                blob,
                                uri,
                                ..
                            } => EmbeddedResourceResource::BlobResourceContents(
                                BlobResourceContents {
                                    blob: blob.clone(),
                                    uri: uri.clone(),
                                    mime_type: mime_type.clone(),
                                    meta: None,
                                },
                            ),
                        },
                        annotations: None,
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
    }
}

/// Handler methods for ACP requests - these are called from the message dispatch loop
impl GooseAcpAgent {
    async fn on_initialize(
        &self,
        args: InitializeRequest,
    ) -> Result<InitializeResponse, sacp::Error> {
        info!("ACP: Received initialize request {:?}", args);

        // Advertise Goose's capabilities
        Ok(InitializeResponse {
            protocol_version: args.protocol_version,
            agent_capabilities: AgentCapabilities {
                load_session: true,
                prompt_capabilities: PromptCapabilities {
                    image: true,
                    audio: false,
                    embedded_context: true,
                    meta: None,
                },
                mcp_capabilities: McpCapabilities {
                    http: false,
                    sse: false,
                    meta: None,
                },
                meta: None,
            },
            auth_methods: vec![],
            agent_info: None,
            meta: None,
        })
    }

    async fn on_new_session(
        &self,
        args: NewSessionRequest,
    ) -> Result<NewSessionResponse, sacp::Error> {
        info!("ACP: Received new session request {:?}", args);

        let goose_session = SessionManager::create_session(
            std::env::current_dir().unwrap_or_default(),
            "ACP Session".to_string(), // just an initial name - may be replaced by maybe_update_name
            SessionType::User,
        )
        .await
        .map_err(|e| {
            error!("Failed to create session: {}", e);
            sacp::Error::internal_error()
        })?;

        let session = GooseAcpSession {
            messages: Conversation::new_unvalidated(Vec::new()),
            tool_call_ids: HashMap::new(),
            tool_requests: HashMap::new(),
            cancel_token: None,
        };

        let mut sessions = self.sessions.lock().await;
        sessions.insert(goose_session.id.clone(), session);

        info!("Created new ACP/goose session {}", goose_session.id);

        Ok(NewSessionResponse {
            session_id: SessionId(goose_session.id.into()),
            modes: None,
            meta: None,
        })
    }

    async fn on_load_session(
        &self,
        args: LoadSessionRequest,
        cx: &JrConnectionCx<AgentToClient>,
    ) -> Result<LoadSessionResponse, sacp::Error> {
        info!("ACP: Received load session request {:?}", args);

        let session_id = args.session_id.0.to_string();

        let goose_session = SessionManager::get_session(&session_id, true)
            .await
            .map_err(|e| {
                error!("Failed to load session {}: {}", session_id, e);
                sacp::Error::invalid_params()
            })?;

        let conversation = goose_session.conversation.ok_or_else(|| {
            error!("Session {} has no conversation data", session_id);
            sacp::Error::internal_error()
        })?;

        SessionManager::update_session(&session_id)
            .working_dir(args.cwd.clone())
            .apply()
            .await
            .map_err(|e| {
                error!("Failed to update session working directory: {}", e);
                sacp::Error::internal_error()
            })?;

        let mut session = GooseAcpSession {
            messages: conversation.clone(),
            tool_call_ids: HashMap::new(),
            tool_requests: HashMap::new(),
            cancel_token: None,
        };

        // Replay conversation history to client
        for message in conversation.messages() {
            // Only replay user-visible messages
            if !message.metadata.user_visible {
                continue;
            }

            for content_item in &message.content {
                match content_item {
                    MessageContent::Text(text) => {
                        let chunk = ContentChunk {
                            content: ContentBlock::Text(TextContent {
                                annotations: None,
                                text: text.text.clone(),
                                meta: None,
                            }),
                            meta: None,
                        };
                        let update = match message.role {
                            Role::User => SessionUpdate::UserMessageChunk(chunk),
                            Role::Assistant => SessionUpdate::AgentMessageChunk(chunk),
                        };
                        cx.send_notification(SessionNotification {
                            session_id: args.session_id.clone(),
                            update,
                            meta: None,
                        })?;
                    }
                    MessageContent::ToolRequest(tool_request) => {
                        self.handle_tool_request(tool_request, &args.session_id, &mut session, cx)
                            .await?;
                    }
                    MessageContent::ToolResponse(tool_response) => {
                        self.handle_tool_response(
                            tool_response,
                            &args.session_id,
                            &mut session,
                            cx,
                        )
                        .await?;
                    }
                    MessageContent::Thinking(thinking) => {
                        cx.send_notification(SessionNotification {
                            session_id: args.session_id.clone(),
                            update: SessionUpdate::AgentThoughtChunk(ContentChunk {
                                content: ContentBlock::Text(TextContent {
                                    annotations: None,
                                    text: thinking.thinking.clone(),
                                    meta: None,
                                }),
                                meta: None,
                            }),
                            meta: None,
                        })?;
                    }
                    _ => {
                        // Ignore other content types
                    }
                }
            }
        }

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session);

        info!("Loaded ACP session {}", session_id);

        Ok(LoadSessionResponse {
            modes: None,
            meta: None,
        })
    }

    async fn on_prompt(
        &self,
        args: PromptRequest,
        cx: &JrConnectionCx<AgentToClient>,
    ) -> Result<PromptResponse, sacp::Error> {
        let session_id = args.session_id.0.to_string();
        let cancel_token = CancellationToken::new();

        {
            let mut sessions = self.sessions.lock().await;
            let session = sessions
                .get_mut(&session_id)
                .ok_or_else(sacp::Error::invalid_params)?;
            session.cancel_token = Some(cancel_token.clone());
        }

        let user_message = self.convert_acp_prompt_to_message(args.prompt);

        let session_config = SessionConfig {
            id: session_id.clone(),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        let mut stream = self
            .agent
            .reply(user_message, session_config, Some(cancel_token.clone()))
            .await
            .map_err(|e| {
                error!("Error getting agent reply: {}", e);
                sacp::Error::internal_error()
            })?;

        use futures::StreamExt;

        let mut was_cancelled = false;

        while let Some(event) = stream.next().await {
            if cancel_token.is_cancelled() {
                was_cancelled = true;
                break;
            }

            match event {
                Ok(goose::agents::AgentEvent::Message(message)) => {
                    let mut sessions = self.sessions.lock().await;
                    let session = sessions
                        .get_mut(&session_id)
                        .ok_or_else(sacp::Error::invalid_params)?;

                    session.messages.push(message.clone());

                    for content_item in &message.content {
                        self.handle_message_content(content_item, &args.session_id, session, cx)
                            .await?;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    error!("Error in agent response stream: {}", e);
                    return Err(sacp::Error::internal_error());
                }
            }
        }

        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.cancel_token = None;
        }

        Ok(PromptResponse {
            stop_reason: if was_cancelled {
                StopReason::Cancelled
            } else {
                StopReason::EndTurn
            },
            meta: None,
        })
    }

    async fn on_cancel(&self, args: CancelNotification) -> Result<(), sacp::Error> {
        info!("ACP: Received cancel request {:?}", args);

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

struct GooseAcpHandler {
    agent: Arc<GooseAcpAgent>,
}

impl JrMessageHandler for GooseAcpHandler {
    type Role = AgentToClient;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        "goose-acp"
    }

    async fn handle_message(
        &mut self,
        message: MessageCx,
        cx: JrConnectionCx<AgentToClient>,
    ) -> Result<Handled<MessageCx>, sacp::Error> {
        use sacp::util::MatchMessageFrom;
        use sacp::JrRequestCx;

        MatchMessageFrom::new(message, &cx)
            .if_request(
                |req: InitializeRequest, req_cx: JrRequestCx<InitializeResponse>| async {
                    req_cx.respond(self.agent.on_initialize(req).await?)
                },
            )
            .await
            .if_request(
                |_req: AuthenticateRequest, req_cx: JrRequestCx<AuthenticateResponse>| async {
                    req_cx.respond(AuthenticateResponse { meta: None })
                },
            )
            .await
            .if_request(
                |req: NewSessionRequest, req_cx: JrRequestCx<NewSessionResponse>| async {
                    req_cx.respond(self.agent.on_new_session(req).await?)
                },
            )
            .await
            .if_request(
                |req: LoadSessionRequest, req_cx: JrRequestCx<LoadSessionResponse>| async {
                    req_cx.respond(self.agent.on_load_session(req, &cx).await?)
                },
            )
            .await
            .if_request(
                |req: PromptRequest, req_cx: JrRequestCx<PromptResponse>| async {
                    req_cx.respond(self.agent.on_prompt(req, &cx).await?)
                },
            )
            .await
            .if_notification(|notif: CancelNotification| async {
                self.agent.on_cancel(notif).await
            })
            .await
            .done()
    }
}

pub async fn run_acp_agent() -> Result<()> {
    info!("Starting Goose ACP agent server on stdio");
    eprintln!("Goose ACP agent started. Listening on stdio...");

    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    let agent = Arc::new(GooseAcpAgent::new().await?);
    let handler = GooseAcpHandler { agent };

    AgentToClient::builder()
        .name("goose-acp")
        .with_handler(handler)
        .serve(ByteStreams::new(outgoing, incoming))
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use sacp::schema::ResourceLink;
    use std::io::Write;
    use tempfile::NamedTempFile;

    use crate::commands::acp::{format_tool_name, read_resource_link};

    fn new_resource_link(content: &str) -> anyhow::Result<(ResourceLink, NamedTempFile)> {
        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;

        let link = ResourceLink {
            name: file
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            uri: format!("file://{}", file.path().to_str().unwrap()),
            annotations: None,
            description: None,
            mime_type: None,
            size: None,
            title: None,
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
        assert_eq!(format_tool_name("todo__write"), "Todo: Write");
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
