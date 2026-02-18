use crate::custom_requests::*;
use anyhow::Result;
use fs_err as fs;
use goose::agents::extension::{Envs, PLATFORM_EXTENSIONS};
use goose::agents::{Agent, AgentConfig, ExtensionConfig, GoosePlatform, SessionConfig};
use goose::builtin_extension::register_builtin_extensions;
use goose::config::base::CONFIG_YAML_NAME;
use goose::config::extensions::get_enabled_extensions_with_config;
use goose::config::paths::Paths;
use goose::config::permission::PermissionManager;
use goose::config::Config;
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::conversation::Conversation;
use goose::mcp_utils::ToolResult;
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};
use goose::providers::base::Provider;
use goose::providers::provider_registry::ProviderConstructor;
use goose::session::session_manager::SessionType;
use goose::session::{Session, SessionManager};
use goose_acp_macros::custom_methods;
use rmcp::model::{CallToolResult, RawContent, ResourceContents, Role};
use sacp::schema::{
    AgentCapabilities, AuthMethod, AuthenticateRequest, AuthenticateResponse, BlobResourceContents,
    CancelNotification, Content, ContentBlock, ContentChunk, EmbeddedResource,
    EmbeddedResourceResource, ImageContent, InitializeRequest, InitializeResponse,
    LoadSessionRequest, LoadSessionResponse, McpCapabilities, McpServer, ModelId, ModelInfo,
    NewSessionRequest, NewSessionResponse, PermissionOption, PermissionOptionKind,
    PromptCapabilities, PromptRequest, PromptResponse, RequestPermissionOutcome,
    RequestPermissionRequest, ResourceLink, SessionId, SessionModelState, SessionNotification,
    SessionUpdate, SetSessionModelRequest, SetSessionModelResponse, StopReason, TextContent,
    TextResourceContents, ToolCall, ToolCallContent, ToolCallId, ToolCallLocation, ToolCallStatus,
    ToolCallUpdate, ToolCallUpdateFields, ToolKind,
};
use sacp::{AgentToClient, ByteStreams, Handled, JrConnectionCx, JrMessageHandler, MessageCx};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use url::Url;

// Agent binds provider, extensions, and permission channels to a single session.
// ACP has no session/close, so sessions accumulate until transport closes.
struct GooseAcpSession {
    agent: Arc<Agent>,
    messages: Conversation,
    tool_requests: HashMap<String, goose::conversation::message::ToolRequest>,
    cancel_token: Option<CancellationToken>,
}

pub struct GooseAcpAgent {
    sessions: Arc<Mutex<HashMap<String, GooseAcpSession>>>,
    provider_factory: ProviderConstructor,
    config_dir: std::path::PathBuf,
    session_manager: Arc<SessionManager>,
    permission_manager: Arc<PermissionManager>,
    goose_mode: goose::config::GooseMode,
    disable_session_naming: bool,
    builtins: Vec<String>,
}

fn mcp_server_to_extension_config(mcp_server: McpServer) -> Result<ExtensionConfig, String> {
    match mcp_server {
        McpServer::Stdio(stdio) => Ok(ExtensionConfig::Stdio {
            name: stdio.name,
            description: String::new(),
            cmd: stdio.command.to_string_lossy().to_string(),
            args: stdio.args,
            envs: Envs::new(stdio.env.into_iter().map(|e| (e.name, e.value)).collect()),
            env_keys: vec![],
            timeout: None,
            bundled: Some(false),
            available_tools: vec![],
        }),
        McpServer::Http(http) => Ok(ExtensionConfig::StreamableHttp {
            name: http.name,
            description: String::new(),
            uri: http.url,
            envs: Envs::default(),
            env_keys: vec![],
            headers: http
                .headers
                .into_iter()
                .map(|h| (h.name, h.value))
                .collect(),
            timeout: None,
            bundled: Some(false),
            available_tools: vec![],
        }),
        McpServer::Sse(_) => Err("SSE is unsupported, migrate to streamable_http".to_string()),
        _ => Err("Unknown MCP server type".to_string()),
    }
}

fn create_tool_location(path: &str, line: Option<u32>) -> ToolCallLocation {
    let mut loc = ToolCallLocation::new(path);
    if let Some(l) = line {
        loc = loc.line(l);
    }
    loc
}

fn extract_tool_locations(
    tool_request: &goose::conversation::message::ToolRequest,
    tool_response: &goose::conversation::message::ToolResponse,
) -> Vec<ToolCallLocation> {
    let mut locations = Vec::new();

    if let Ok(tool_call) = &tool_request.tool_call {
        if tool_call.name != "developer__text_editor" {
            return locations;
        }

        let path_str = tool_call
            .arguments
            .as_ref()
            .and_then(|args| args.get("path"))
            .and_then(|p| p.as_str());

        if let Some(path_str) = path_str {
            let command = tool_call
                .arguments
                .as_ref()
                .and_then(|args| args.get("command"))
                .and_then(|c| c.as_str());

            if let Ok(result) = &tool_response.tool_result {
                for content in &result.content {
                    if let RawContent::Text(text_content) = &content.raw {
                        let text = &text_content.text;

                        match command {
                            Some("view") => {
                                let line = extract_view_line_range(text)
                                    .map(|range| range.0 as u32)
                                    .or(Some(1));
                                locations.push(create_tool_location(path_str, line));
                            }
                            Some("str_replace") | Some("insert") => {
                                let line = extract_first_line_number(text)
                                    .map(|l| l as u32)
                                    .or(Some(1));
                                locations.push(create_tool_location(path_str, line));
                            }
                            Some("write") => {
                                locations.push(create_tool_location(path_str, Some(1)));
                            }
                            _ => {
                                locations.push(create_tool_location(path_str, Some(1)));
                            }
                        }
                        break;
                    }
                }
            }

            if locations.is_empty() {
                locations.push(create_tool_location(path_str, Some(1)));
            }
        }
    }

    locations
}

fn extract_view_line_range(text: &str) -> Option<(usize, usize)> {
    let re = regex::Regex::new(r"\(lines (\d+)-(\d+|end)\)").ok()?;
    if let Some(caps) = re.captures(text) {
        let start = caps.get(1)?.as_str().parse::<usize>().ok()?;
        let end = if caps.get(2)?.as_str() == "end" {
            start
        } else {
            caps.get(2)?.as_str().parse::<usize>().ok()?
        };
        return Some((start, end));
    }
    None
}

fn extract_first_line_number(text: &str) -> Option<usize> {
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

fn format_tool_name(tool_name: &str) -> String {
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

    if let Some((extension, tool)) = tool_name.split_once("__") {
        let formatted_extension = extension.replace('_', " ");
        let formatted_tool = tool.replace('_', " ");
        format!(
            "{}: {}",
            capitalize(&formatted_extension),
            capitalize(&formatted_tool)
        )
    } else {
        let formatted = tool_name.replace('_', " ");
        capitalize(&formatted)
    }
}

async fn add_builtins(agent: &Agent, builtins: Vec<String>) {
    for builtin in builtins {
        let config = if PLATFORM_EXTENSIONS.contains_key(builtin.as_str()) {
            ExtensionConfig::Platform {
                name: builtin.clone(),
                description: builtin.clone(),
                display_name: None,
                bundled: None,
                available_tools: Vec::new(),
            }
        } else {
            ExtensionConfig::Builtin {
                name: builtin.clone(),
                display_name: None,
                timeout: None,
                bundled: None,
                description: builtin.clone(),
                available_tools: Vec::new(),
            }
        };

        match agent
            .extension_manager
            .add_extension(config, None, None, None)
            .await
        {
            Ok(_) => info!(extension = %builtin, "extension loaded"),
            Err(e) => warn!(extension = %builtin, error = %e, "extension load failed"),
        }
    }
}
async fn add_extensions(agent: &Agent, extensions: Vec<ExtensionConfig>) {
    for extension in extensions {
        let name = extension.name().to_string();
        match agent
            .extension_manager
            .add_extension(extension, None, None, None)
            .await
        {
            Ok(_) => info!(extension = %name, "extension loaded"),
            Err(e) => warn!(extension = %name, error = %e, "extension load failed"),
        }
    }
}

async fn build_model_state(
    provider: &dyn Provider,
    current_model: &str,
) -> Result<SessionModelState, sacp::Error> {
    let models = provider.fetch_recommended_models().await.map_err(|e| {
        sacp::Error::internal_error().data(format!("Failed to fetch models: {}", e))
    })?;
    Ok(SessionModelState::new(
        ModelId::new(current_model),
        models
            .iter()
            .map(|name| ModelInfo::new(ModelId::new(&**name), &**name))
            .collect(),
    ))
}

impl GooseAcpAgent {
    pub fn permission_manager(&self) -> Arc<PermissionManager> {
        Arc::clone(&self.permission_manager)
    }

    pub async fn new(
        provider_factory: ProviderConstructor,
        builtins: Vec<String>,
        data_dir: std::path::PathBuf,
        config_dir: std::path::PathBuf,
        goose_mode: goose::config::GooseMode,
        disable_session_naming: bool,
    ) -> Result<Self> {
        let session_manager = Arc::new(SessionManager::new(data_dir));
        let permission_manager = Arc::new(PermissionManager::new(config_dir.clone()));

        Ok(Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            provider_factory,
            config_dir,
            session_manager,
            permission_manager,
            goose_mode,
            disable_session_naming,
            builtins,
        })
    }

    async fn create_agent_for_session(&self) -> Arc<Agent> {
        let agent = Agent::with_config(AgentConfig::new(
            Arc::clone(&self.session_manager),
            Arc::clone(&self.permission_manager),
            None,
            self.goose_mode,
            self.disable_session_naming,
            GoosePlatform::GooseCli,
        ));
        let agent = Arc::new(agent);

        let config_path = self.config_dir.join(CONFIG_YAML_NAME);
        if let Ok(config_file) = Config::new(&config_path, "goose") {
            let extensions = get_enabled_extensions_with_config(&config_file);
            add_extensions(&agent, extensions).await;
        }
        add_builtins(&agent, self.builtins.clone()).await;

        agent
    }

    pub async fn has_session(&self, session_id: &str) -> bool {
        self.sessions.lock().await.contains_key(session_id)
    }

    fn convert_acp_prompt_to_message(&self, prompt: Vec<ContentBlock>) -> Message {
        let mut user_message = Message::user();

        for block in prompt {
            match block {
                ContentBlock::Text(text) => {
                    user_message = user_message.with_text(&text.text);
                }
                ContentBlock::Image(image) => {
                    user_message = user_message.with_image(&image.data, &image.mime_type);
                }
                ContentBlock::Resource(resource) => {
                    if let EmbeddedResourceResource::TextResourceContents(text_resource) =
                        &resource.resource
                    {
                        let header = format!("--- Resource: {} ---\n", text_resource.uri);
                        let content = format!("{}{}\n---\n", header, text_resource.text);
                        user_message = user_message.with_text(&content);
                    }
                }
                ContentBlock::ResourceLink(link) => {
                    if let Some(text) = read_resource_link(link) {
                        user_message = user_message.with_text(text)
                    }
                }
                ContentBlock::Audio(..) | _ => (),
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
                cx.send_notification(SessionNotification::new(
                    session_id.clone(),
                    SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                        TextContent::new(text.text.clone()),
                    ))),
                ))?;
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
                cx.send_notification(SessionNotification::new(
                    session_id.clone(),
                    SessionUpdate::AgentThoughtChunk(ContentChunk::new(ContentBlock::Text(
                        TextContent::new(thinking.thinking.clone()),
                    ))),
                ))?;
            }
            MessageContent::ActionRequired(action_required) => {
                if let ActionRequiredData::ToolConfirmation {
                    id,
                    tool_name,
                    arguments,
                    prompt,
                } = &action_required.data
                {
                    self.handle_tool_permission_request(
                        cx,
                        &session.agent,
                        session_id,
                        id.clone(),
                        tool_name.clone(),
                        arguments.clone(),
                        prompt.clone(),
                    )?;
                }
            }
            _ => {}
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
        session
            .tool_requests
            .insert(tool_request.id.clone(), tool_request.clone());

        let tool_name = match &tool_request.tool_call {
            Ok(tool_call) => tool_call.name.to_string(),
            Err(_) => "error".to_string(),
        };

        cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ToolCall(
                ToolCall::new(
                    ToolCallId::new(tool_request.id.clone()),
                    format_tool_name(&tool_name),
                )
                .status(ToolCallStatus::Pending),
            ),
        ))?;

        Ok(())
    }

    async fn handle_tool_response(
        &self,
        tool_response: &goose::conversation::message::ToolResponse,
        session_id: &SessionId,
        session: &mut GooseAcpSession,
        cx: &JrConnectionCx<AgentToClient>,
    ) -> Result<(), sacp::Error> {
        let status = match &tool_response.tool_result {
            Ok(result) if result.is_error == Some(true) => ToolCallStatus::Failed,
            Ok(_) => ToolCallStatus::Completed,
            Err(_) => ToolCallStatus::Failed,
        };

        let content = build_tool_call_content(&tool_response.tool_result);

        let locations = if let Some(tool_request) = session.tool_requests.get(&tool_response.id) {
            extract_tool_locations(tool_request, tool_response)
        } else {
            Vec::new()
        };

        let mut fields = ToolCallUpdateFields::new().status(status).content(content);
        if !locations.is_empty() {
            fields = fields.locations(locations);
        }
        cx.send_notification(SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
                ToolCallId::new(tool_response.id.clone()),
                fields,
            )),
        ))?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_tool_permission_request(
        &self,
        cx: &JrConnectionCx<AgentToClient>,
        agent: &Arc<Agent>,
        session_id: &SessionId,
        request_id: String,
        tool_name: String,
        arguments: serde_json::Map<String, serde_json::Value>,
        prompt: Option<String>,
    ) -> Result<(), sacp::Error> {
        let cx = cx.clone();
        let agent = agent.clone();
        let session_id = session_id.clone();

        let formatted_name = format_tool_name(&tool_name);

        let mut fields = ToolCallUpdateFields::new()
            .title(formatted_name)
            .kind(ToolKind::default())
            .status(ToolCallStatus::Pending)
            .raw_input(serde_json::Value::Object(arguments));
        if let Some(p) = prompt {
            fields = fields.content(vec![ToolCallContent::Content(Content::new(
                ContentBlock::Text(TextContent::new(p)),
            ))]);
        }
        let tool_call_update = ToolCallUpdate::new(ToolCallId::new(request_id.clone()), fields);

        fn option(kind: PermissionOptionKind) -> PermissionOption {
            let id = serde_json::to_value(kind)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            PermissionOption::new(id.clone(), id, kind)
        }
        let options = vec![
            option(PermissionOptionKind::AllowAlways),
            option(PermissionOptionKind::AllowOnce),
            option(PermissionOptionKind::RejectOnce),
            option(PermissionOptionKind::RejectAlways),
        ];

        let permission_request =
            RequestPermissionRequest::new(session_id, tool_call_update, options);

        cx.send_request(permission_request)
            .on_receiving_result(move |result| async move {
                match result {
                    Ok(response) => {
                        agent
                            .handle_confirmation(
                                request_id,
                                outcome_to_confirmation(&response.outcome),
                            )
                            .await;
                        Ok(())
                    }
                    Err(e) => {
                        error!(error = ?e, "permission request failed");
                        agent
                            .handle_confirmation(
                                request_id,
                                PermissionConfirmation {
                                    principal_type: PrincipalType::Tool,
                                    permission: Permission::Cancel,
                                },
                            )
                            .await;
                        Ok(())
                    }
                }
            })?;

        Ok(())
    }
}

fn outcome_to_confirmation(outcome: &RequestPermissionOutcome) -> PermissionConfirmation {
    let permission = match outcome {
        RequestPermissionOutcome::Cancelled => Permission::Cancel,
        RequestPermissionOutcome::Selected(selected) => {
            match serde_json::from_value::<PermissionOptionKind>(serde_json::Value::String(
                selected.option_id.0.to_string(),
            )) {
                Ok(PermissionOptionKind::AllowAlways) => Permission::AlwaysAllow,
                Ok(PermissionOptionKind::AllowOnce) => Permission::AllowOnce,
                Ok(PermissionOptionKind::RejectOnce) => Permission::DenyOnce,
                Ok(PermissionOptionKind::RejectAlways) => Permission::AlwaysDeny,
                _ => Permission::Cancel,
            }
        }
        _ => Permission::Cancel,
    };
    PermissionConfirmation {
        principal_type: PrincipalType::Tool,
        permission,
    }
}

fn build_tool_call_content(tool_result: &ToolResult<CallToolResult>) -> Vec<ToolCallContent> {
    match tool_result {
        Ok(result) => result
            .content
            .iter()
            .filter_map(|content| match &content.raw {
                RawContent::Text(val) => Some(ToolCallContent::Content(Content::new(
                    ContentBlock::Text(TextContent::new(val.text.clone())),
                ))),
                RawContent::Image(val) => Some(ToolCallContent::Content(Content::new(
                    ContentBlock::Image(ImageContent::new(val.data.clone(), val.mime_type.clone())),
                ))),
                RawContent::Resource(val) => {
                    let resource = match &val.resource {
                        ResourceContents::TextResourceContents {
                            mime_type,
                            text,
                            uri,
                            ..
                        } => EmbeddedResourceResource::TextResourceContents(
                            TextResourceContents::new(text.clone(), uri.clone())
                                .mime_type(mime_type.clone()),
                        ),
                        ResourceContents::BlobResourceContents {
                            mime_type,
                            blob,
                            uri,
                            ..
                        } => EmbeddedResourceResource::BlobResourceContents(
                            BlobResourceContents::new(blob.clone(), uri.clone())
                                .mime_type(mime_type.clone()),
                        ),
                    };
                    Some(ToolCallContent::Content(Content::new(
                        ContentBlock::Resource(EmbeddedResource::new(resource)),
                    )))
                }
                RawContent::Audio(_) | RawContent::ResourceLink(_) => None,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

impl GooseAcpAgent {
    async fn on_initialize(
        &self,
        args: InitializeRequest,
    ) -> Result<InitializeResponse, sacp::Error> {
        debug!(?args, "initialize request");

        let capabilities = AgentCapabilities::new()
            .load_session(true)
            .prompt_capabilities(
                PromptCapabilities::new()
                    .image(true)
                    .audio(false)
                    .embedded_context(true),
            )
            .mcp_capabilities(McpCapabilities::new().http(true));
        Ok(InitializeResponse::new(args.protocol_version)
            .agent_capabilities(capabilities)
            .auth_methods(vec![AuthMethod::new(
                "goose-provider",
                "Configure Provider",
            )
            .description(
                "Run `goose configure` to set up your AI provider and API key",
            )]))
    }

    async fn on_new_session(
        &self,
        args: NewSessionRequest,
    ) -> Result<NewSessionResponse, sacp::Error> {
        debug!(?args, "new session request");

        let goose_session = self
            .session_manager
            .create_session(
                args.cwd.clone(),
                "ACP Session".to_string(),
                SessionType::User,
            )
            .await
            .map_err(|e| {
                sacp::Error::internal_error().data(format!("Failed to create session: {}", e))
            })?;

        let agent = self.create_agent_for_session().await;
        let provider = self
            .init_provider(&agent, &goose_session)
            .await
            .map_err(|e| {
                sacp::Error::internal_error().data(format!("Failed to set provider: {}", e))
            })?;

        for mcp_server in args.mcp_servers {
            let config = match mcp_server_to_extension_config(mcp_server) {
                Ok(c) => c,
                Err(msg) => {
                    return Err(sacp::Error::invalid_params().data(msg));
                }
            };
            let name = config.name().to_string();
            if let Err(e) = agent.add_extension(config, &goose_session.id).await {
                return Err(sacp::Error::internal_error()
                    .data(format!("Failed to add MCP server '{}': {}", name, e)));
            }
        }

        let session = GooseAcpSession {
            agent,
            messages: Conversation::new_unvalidated(Vec::new()),
            tool_requests: HashMap::new(),
            cancel_token: None,
        };

        let mut sessions = self.sessions.lock().await;
        sessions.insert(goose_session.id.clone(), session);

        info!(
            session_id = %goose_session.id,
            session_type = "acp",
            "Session started"
        );

        let model_state =
            build_model_state(&*provider, &provider.get_model_config().model_name).await?;

        Ok(NewSessionResponse::new(SessionId::new(goose_session.id)).models(model_state))
    }

    async fn init_provider(&self, agent: &Agent, session: &Session) -> Result<Arc<dyn Provider>> {
        let model_config = match &session.model_config {
            Some(config) => config.clone(),
            None => {
                let config_path = self.config_dir.join(CONFIG_YAML_NAME);
                let config = Config::new(&config_path, "goose")?;
                let model_id = config.get_goose_model()?;
                let provider_name = config.get_goose_provider()?;
                goose::model::ModelConfig::new(&model_id)?.with_canonical_limits(&provider_name)
            }
        };
        let provider = (self.provider_factory)(model_config, Vec::new()).await?;
        agent.update_provider(provider.clone(), &session.id).await?;
        Ok(provider)
    }

    async fn on_load_session(
        &self,
        args: LoadSessionRequest,
        cx: &JrConnectionCx<AgentToClient>,
    ) -> Result<LoadSessionResponse, sacp::Error> {
        debug!(?args, "load session request");

        let session_id = args.session_id.0.to_string();

        let goose_session = self
            .session_manager
            .get_session(&session_id, true)
            .await
            .map_err(|e| {
                sacp::Error::invalid_params()
                    .data(format!("Failed to load session {}: {}", session_id, e))
            })?;

        let agent = self.create_agent_for_session().await;
        let provider = self
            .init_provider(&agent, &goose_session)
            .await
            .map_err(|e| {
                sacp::Error::internal_error().data(format!("Failed to set provider: {}", e))
            })?;

        let conversation = goose_session.conversation.ok_or_else(|| {
            sacp::Error::internal_error()
                .data(format!("Session {} has no conversation data", session_id))
        })?;

        self.session_manager
            .update(&session_id)
            .working_dir(args.cwd.clone())
            .apply()
            .await
            .map_err(|e| {
                sacp::Error::internal_error()
                    .data(format!("Failed to update session working directory: {}", e))
            })?;

        let mut session = GooseAcpSession {
            agent,
            messages: conversation.clone(),
            tool_requests: HashMap::new(),
            cancel_token: None,
        };

        for message in conversation.messages() {
            if !message.metadata.user_visible {
                continue;
            }

            for content_item in &message.content {
                match content_item {
                    MessageContent::Text(text) => {
                        let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new(
                            text.text.clone(),
                        )));
                        let update = match message.role {
                            Role::User => SessionUpdate::UserMessageChunk(chunk),
                            Role::Assistant => SessionUpdate::AgentMessageChunk(chunk),
                        };
                        cx.send_notification(SessionNotification::new(
                            args.session_id.clone(),
                            update,
                        ))?;
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
                        cx.send_notification(SessionNotification::new(
                            args.session_id.clone(),
                            SessionUpdate::AgentThoughtChunk(ContentChunk::new(
                                ContentBlock::Text(TextContent::new(thinking.thinking.clone())),
                            )),
                        ))?;
                    }
                    _ => {}
                }
            }
        }

        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id.clone(), session);

        info!(
            session_id = %session_id,
            session_type = "acp",
            "Session loaded"
        );

        let model_state =
            build_model_state(&*provider, &provider.get_model_config().model_name).await?;

        Ok(LoadSessionResponse::new().models(model_state))
    }

    async fn on_prompt(
        &self,
        args: PromptRequest,
        cx: &JrConnectionCx<AgentToClient>,
    ) -> Result<PromptResponse, sacp::Error> {
        let session_id = args.session_id.0.to_string();
        let cancel_token = CancellationToken::new();

        let agent = {
            let mut sessions = self.sessions.lock().await;
            let session = sessions.get_mut(&session_id).ok_or_else(|| {
                sacp::Error::invalid_params().data(format!("Session not found: {}", session_id))
            })?;
            session.cancel_token = Some(cancel_token.clone());
            session.agent.clone()
        };

        let user_message = self.convert_acp_prompt_to_message(args.prompt);

        let session_config = SessionConfig {
            id: session_id.clone(),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        let mut stream = agent
            .reply(user_message, session_config, Some(cancel_token.clone()))
            .await
            .map_err(|e| {
                sacp::Error::internal_error().data(format!("Error getting agent reply: {}", e))
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
                    let session = sessions.get_mut(&session_id).ok_or_else(|| {
                        sacp::Error::invalid_params()
                            .data(format!("Session not found: {}", session_id))
                    })?;

                    session.messages.push(message.clone());

                    for content_item in &message.content {
                        self.handle_message_content(content_item, &args.session_id, session, cx)
                            .await?;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    return Err(sacp::Error::internal_error()
                        .data(format!("Error in agent response stream: {}", e)));
                }
            }
        }

        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.cancel_token = None;
        }

        Ok(PromptResponse::new(if was_cancelled {
            StopReason::Cancelled
        } else {
            StopReason::EndTurn
        }))
    }

    async fn on_cancel(&self, args: CancelNotification) -> Result<(), sacp::Error> {
        debug!(?args, "cancel request");

        let session_id = args.session_id.0.to_string();
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.get_mut(&session_id) {
            if let Some(ref token) = session.cancel_token {
                info!(session_id = %session_id, "prompt cancelled");
                token.cancel();
            }
        } else {
            warn!(session_id = %session_id, "cancel request for unknown session");
        }

        Ok(())
    }

    async fn on_set_model(
        &self,
        session_id: &str,
        model_id: &str,
    ) -> Result<SetSessionModelResponse, sacp::Error> {
        let config_path = self.config_dir.join(CONFIG_YAML_NAME);
        let config = Config::new(&config_path, "goose").map_err(|e| {
            sacp::Error::internal_error().data(format!("Failed to read config: {}", e))
        })?;
        let provider_name = config.get_goose_provider().map_err(|_| {
            sacp::Error::internal_error().data("No provider configured".to_string())
        })?;
        let model_config = goose::model::ModelConfig::new(model_id)
            .map_err(|e| {
                sacp::Error::invalid_params().data(format!("Invalid model config: {}", e))
            })?
            .with_canonical_limits(&provider_name);
        let provider = (self.provider_factory)(model_config, Vec::new())
            .await
            .map_err(|e| {
                sacp::Error::internal_error().data(format!("Failed to create provider: {}", e))
            })?;

        let agent = {
            let sessions = self.sessions.lock().await;
            let session = sessions.get(session_id).ok_or_else(|| {
                sacp::Error::invalid_params().data(format!("Session not found: {}", session_id))
            })?;
            session.agent.clone()
        };
        agent
            .update_provider(provider, session_id)
            .await
            .map_err(|e| {
                sacp::Error::internal_error().data(format!("Failed to update provider: {}", e))
            })?;

        info!(session_id = %session_id, model_id = %model_id, "Model switched");
        Ok(SetSessionModelResponse::new())
    }
}

#[custom_methods]
impl GooseAcpAgent {
    #[custom_method("extensions/add")]
    async fn on_add_extension(
        &self,
        req: AddExtensionRequest,
    ) -> Result<EmptyResponse, sacp::Error> {
        let config: ExtensionConfig = serde_json::from_value(req.config)
            .map_err(|e| sacp::Error::invalid_params().data(format!("bad config: {e}")))?;
        let agent = self.get_agent_for_session(&req.session_id).await?;
        agent
            .add_extension(config, &req.session_id)
            .await
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(EmptyResponse {})
    }

    #[custom_method("extensions/remove")]
    async fn on_remove_extension(
        &self,
        req: RemoveExtensionRequest,
    ) -> Result<EmptyResponse, sacp::Error> {
        let agent = self.get_agent_for_session(&req.session_id).await?;
        agent
            .remove_extension(&req.name, &req.session_id)
            .await
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(EmptyResponse {})
    }

    #[custom_method("tools")]
    async fn on_get_tools(&self, req: GetToolsRequest) -> Result<GetToolsResponse, sacp::Error> {
        let agent = self.get_agent_for_session(&req.session_id).await?;
        let tools = agent.list_tools(&req.session_id, None).await;
        let tools_json = tools
            .into_iter()
            .map(|t| serde_json::to_value(&t))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(GetToolsResponse { tools: tools_json })
    }

    #[custom_method("resource/read")]
    async fn on_read_resource(
        &self,
        req: ReadResourceRequest,
    ) -> Result<ReadResourceResponse, sacp::Error> {
        let agent = self.get_agent_for_session(&req.session_id).await?;
        let cancel_token = CancellationToken::new();
        let result = agent
            .extension_manager
            .read_resource(&req.session_id, &req.uri, &req.extension_name, cancel_token)
            .await
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        let result_json = serde_json::to_value(&result)
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(ReadResourceResponse {
            result: result_json,
        })
    }

    #[custom_method("working_dir/update")]
    async fn on_update_working_dir(
        &self,
        req: UpdateWorkingDirRequest,
    ) -> Result<EmptyResponse, sacp::Error> {
        let working_dir = req.working_dir.trim().to_string();
        if working_dir.is_empty() {
            return Err(sacp::Error::invalid_params().data("working directory cannot be empty"));
        }
        let path = std::path::PathBuf::from(&working_dir);
        if !path.exists() || !path.is_dir() {
            return Err(sacp::Error::invalid_params().data("invalid directory path"));
        }
        self.session_manager
            .update(&req.session_id)
            .working_dir(path)
            .apply()
            .await
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(EmptyResponse {})
    }

    #[custom_method("session/list")]
    async fn on_list_sessions(&self) -> Result<ListSessionsResponse, sacp::Error> {
        let sessions = self
            .session_manager
            .list_sessions()
            .await
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        let sessions_json = sessions
            .into_iter()
            .map(|s| serde_json::to_value(&s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(ListSessionsResponse {
            sessions: sessions_json,
        })
    }

    #[custom_method("session/get")]
    async fn on_get_session(
        &self,
        req: GetSessionRequest,
    ) -> Result<GetSessionResponse, sacp::Error> {
        let session = self
            .session_manager
            .get_session(&req.session_id, req.include_messages)
            .await
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        let session_json = serde_json::to_value(&session)
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(GetSessionResponse {
            session: session_json,
        })
    }

    #[custom_method("session/delete")]
    async fn on_delete_session(
        &self,
        req: DeleteSessionRequest,
    ) -> Result<EmptyResponse, sacp::Error> {
        self.session_manager
            .delete_session(&req.session_id)
            .await
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(EmptyResponse {})
    }

    #[custom_method("session/export")]
    async fn on_export_session(
        &self,
        req: ExportSessionRequest,
    ) -> Result<ExportSessionResponse, sacp::Error> {
        let data = self
            .session_manager
            .export_session(&req.session_id)
            .await
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(ExportSessionResponse { data })
    }

    #[custom_method("session/import")]
    async fn on_import_session(
        &self,
        req: ImportSessionRequest,
    ) -> Result<ImportSessionResponse, sacp::Error> {
        let session = self
            .session_manager
            .import_session(&req.data)
            .await
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        let session_json = serde_json::to_value(&session)
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(ImportSessionResponse {
            session: session_json,
        })
    }

    #[custom_method("config/extensions")]
    async fn on_get_extensions(&self) -> Result<GetExtensionsResponse, sacp::Error> {
        let extensions = goose::config::extensions::get_all_extensions();
        let warnings = goose::config::extensions::get_warnings();
        let extensions_json = extensions
            .into_iter()
            .map(|e| serde_json::to_value(&e))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
        Ok(GetExtensionsResponse {
            extensions: extensions_json,
            warnings,
        })
    }

    #[custom_method("tool/call")]
    async fn on_tool_call(
        &self,
        _req: serde_json::Value,
    ) -> Result<serde_json::Value, sacp::Error> {
        Err(sacp::Error::new(-32001, "tool/call not yet implemented"))
    }

    #[custom_method("provider/update")]
    async fn on_provider_update(
        &self,
        _req: serde_json::Value,
    ) -> Result<serde_json::Value, sacp::Error> {
        Err(sacp::Error::new(
            -32001,
            "provider/update not yet implemented",
        ))
    }

    #[custom_method("container/set")]
    async fn on_container_set(
        &self,
        _req: serde_json::Value,
    ) -> Result<serde_json::Value, sacp::Error> {
        Err(sacp::Error::new(
            -32001,
            "container/set not yet implemented",
        ))
    }

    #[custom_method("apps/list")]
    async fn on_apps_list(&self) -> Result<serde_json::Value, sacp::Error> {
        Err(sacp::Error::new(-32001, "apps/list not yet implemented"))
    }

    #[custom_method("apps/export")]
    async fn on_apps_export(
        &self,
        _req: serde_json::Value,
    ) -> Result<serde_json::Value, sacp::Error> {
        Err(sacp::Error::new(-32001, "apps/export not yet implemented"))
    }

    #[custom_method("apps/import")]
    async fn on_apps_import(
        &self,
        _req: serde_json::Value,
    ) -> Result<serde_json::Value, sacp::Error> {
        Err(sacp::Error::new(-32001, "apps/import not yet implemented"))
    }

    #[custom_method("config/providers")]
    async fn on_config_providers(&self) -> Result<serde_json::Value, sacp::Error> {
        Err(sacp::Error::new(
            -32001,
            "config/providers not yet implemented",
        ))
    }

    async fn get_agent_for_session(&self, session_id: &str) -> Result<Arc<Agent>, sacp::Error> {
        self.sessions
            .lock()
            .await
            .get(session_id)
            .map(|s| Arc::clone(&s.agent))
            .ok_or_else(|| {
                sacp::Error::invalid_params().data(format!("no active session: {session_id}"))
            })
    }
}

pub struct GooseAcpHandler {
    pub agent: Arc<GooseAcpAgent>,
}

impl JrMessageHandler for GooseAcpHandler {
    type Link = AgentToClient;

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
                    req_cx.respond(AuthenticateResponse::new())
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
                    let agent = self.agent.clone();
                    let cx_clone = cx.clone();
                    cx.spawn(async move {
                        match agent.on_prompt(req, &cx_clone).await {
                            Ok(response) => {
                                req_cx.respond(response)?;
                            }
                            Err(e) => {
                                req_cx.respond_with_error(e)?;
                            }
                        }
                        Ok(())
                    })?;
                    Ok(())
                },
            )
            .await
            .if_notification(|notif: CancelNotification| async {
                self.agent.on_cancel(notif).await
            })
            .await
            // Handle methods not yet in the sacp typed API.
            // - session/set_model: typed support pending in sacp
            // - _<method>: custom requests that will eventually route to goose-server
            .otherwise({
                let agent = self.agent.clone();
                |message: MessageCx| async move {
                    match message {
                        MessageCx::Request(req, request_cx)
                            if req.method == "session/set_model" =>
                        {
                            let params: SetSessionModelRequest = serde_json::from_value(req.params)
                                .map_err(|e| sacp::Error::invalid_params().data(e.to_string()))?;
                            let resp = agent
                                .on_set_model(&params.session_id.0, &params.model_id.0)
                                .await?;
                            let json = serde_json::to_value(resp)
                                .map_err(|e| sacp::Error::internal_error().data(e.to_string()))?;
                            request_cx.respond(json)?;
                            Ok(())
                        }
                        MessageCx::Request(req, request_cx) if req.method.starts_with('_') => {
                            match agent.handle_custom_request(&req.method, req.params).await {
                                Ok(json) => request_cx.respond(json)?,
                                Err(e) => request_cx.respond_with_error(e)?,
                            }
                            Ok(())
                        }
                        _ => Err(sacp::Error::method_not_found()),
                    }
                }
            })
            .await
            .map(|()| Handled::Yes)
    }
}

pub async fn serve<R, W>(agent: Arc<GooseAcpAgent>, read: R, write: W) -> Result<()>
where
    R: futures::AsyncRead + Unpin + Send + 'static,
    W: futures::AsyncWrite + Unpin + Send + 'static,
{
    let handler = GooseAcpHandler { agent };

    AgentToClient::builder()
        .name("goose-acp")
        .with_handler(handler)
        .serve(ByteStreams::new(write, read))
        .await?;

    Ok(())
}

pub async fn run(builtins: Vec<String>) -> Result<()> {
    register_builtin_extensions(goose_mcp::BUILTIN_EXTENSIONS.clone());
    info!("listening on stdio");

    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    let server =
        crate::server_factory::AcpServer::new(crate::server_factory::AcpServerFactoryConfig {
            builtins,
            data_dir: Paths::data_dir(),
            config_dir: Paths::config_dir(),
        });
    let agent = server.create_agent().await?;
    serve(agent, incoming, outgoing).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sacp::schema::{
        EnvVariable, HttpHeader, McpServer, McpServerHttp, McpServerSse, McpServerStdio,
        PermissionOptionId, ResourceLink, SelectedPermissionOutcome,
    };
    use std::io::Write;
    use tempfile::NamedTempFile;
    use test_case::test_case;

    #[test_case(
        McpServer::Stdio(
            McpServerStdio::new("github", "/path/to/github-mcp-server")
                .args(vec!["stdio".into()])
                .env(vec![EnvVariable::new("GITHUB_PERSONAL_ACCESS_TOKEN", "ghp_xxxxxxxxxxxx")])
        ),
        Ok(ExtensionConfig::Stdio {
            name: "github".into(),
            description: String::new(),
            cmd: "/path/to/github-mcp-server".into(),
            args: vec!["stdio".into()],
            envs: Envs::new(
                [(
                    "GITHUB_PERSONAL_ACCESS_TOKEN".into(),
                    "ghp_xxxxxxxxxxxx".into()
                )]
                .into()
            ),
            env_keys: vec![],
            timeout: None,
            bundled: Some(false),
            available_tools: vec![],
        })
    )]
    #[test_case(
        McpServer::Http(
            McpServerHttp::new("github", "https://api.githubcopilot.com/mcp/")
                .headers(vec![HttpHeader::new("Authorization", "Bearer ghp_xxxxxxxxxxxx")])
        ),
        Ok(ExtensionConfig::StreamableHttp {
            name: "github".into(),
            description: String::new(),
            uri: "https://api.githubcopilot.com/mcp/".into(),
            envs: Envs::default(),
            env_keys: vec![],
            headers: HashMap::from([(
                "Authorization".into(),
                "Bearer ghp_xxxxxxxxxxxx".into()
            )]),
            timeout: None,
            bundled: Some(false),
            available_tools: vec![],
        })
    )]
    #[test_case(
        McpServer::Sse(McpServerSse::new("test-sse", "https://agent-fin.biodnd.com/sse")),
        Err("SSE is unsupported, migrate to streamable_http".to_string())
    )]
    fn test_mcp_server_to_extension_config(
        input: McpServer,
        expected: Result<ExtensionConfig, String>,
    ) {
        assert_eq!(mcp_server_to_extension_config(input), expected);
    }

    fn new_resource_link(content: &str) -> anyhow::Result<(ResourceLink, NamedTempFile)> {
        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;

        let name = file
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let uri = format!("file://{}", file.path().to_str().unwrap());
        let link = ResourceLink::new(name, uri);
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

    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("allow_once".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::AllowOnce };
        "allow_once_maps_to_allow_once"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("allow_always".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::AlwaysAllow };
        "allow_always_maps_to_always_allow"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("reject_once".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::DenyOnce };
        "reject_once_maps_to_deny_once"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("reject_always".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::AlwaysDeny };
        "reject_always_maps_to_always_deny"
    )]
    #[test_case(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(PermissionOptionId::from("unknown".to_string()))),
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::Cancel };
        "unknown_option_maps_to_cancel"
    )]
    #[test_case(
        RequestPermissionOutcome::Cancelled,
        PermissionConfirmation { principal_type: PrincipalType::Tool, permission: Permission::Cancel };
        "cancelled_maps_to_cancel"
    )]
    fn test_outcome_to_confirmation(
        input: RequestPermissionOutcome,
        expected: PermissionConfirmation,
    ) {
        assert_eq!(outcome_to_confirmation(&input), expected);
    }

    use goose::providers::errors::ProviderError;

    struct MockModelProvider {
        models: Result<Vec<String>, ProviderError>,
    }

    #[async_trait::async_trait]
    impl goose::providers::base::Provider for MockModelProvider {
        fn get_name(&self) -> &str {
            "mock"
        }

        async fn stream(
            &self,
            _model_config: &goose::model::ModelConfig,
            _session_id: &str,
            _system: &str,
            _messages: &[goose::conversation::message::Message],
            _tools: &[rmcp::model::Tool],
        ) -> Result<goose::providers::base::MessageStream, ProviderError> {
            unimplemented!()
        }

        fn get_model_config(&self) -> goose::model::ModelConfig {
            goose::model::ModelConfig::new_or_fail("unused")
        }

        async fn fetch_recommended_models(&self) -> Result<Vec<String>, ProviderError> {
            self.models.clone()
        }
    }

    #[test_case(
        "model-a", Ok(vec!["model-a".into(), "model-b".into()])
        => Ok(SessionModelState::new(
            ModelId::new("model-a"),
            vec![ModelInfo::new(ModelId::new("model-a"), "model-a"),
                 ModelInfo::new(ModelId::new("model-b"), "model-b")],
        ))
        ; "returns current and available models"
    )]
    #[test_case(
        "model-a", Ok(vec![])
        => Ok(SessionModelState::new(ModelId::new("model-a"), vec![]))
        ; "empty model list"
    )]
    #[test_case(
        "model-a", Err(ProviderError::ExecutionError("fail".into()))
        => matches Err(_)
        ; "fetch error propagates"
    )]
    #[test_case(
        "switched-model", Ok(vec!["model-a".into(), "switched-model".into()])
        => Ok(SessionModelState::new(
            ModelId::new("switched-model"),
            vec![ModelInfo::new(ModelId::new("model-a"), "model-a"),
                 ModelInfo::new(ModelId::new("switched-model"), "switched-model")],
        ))
        ; "current model reflects switched model"
    )]
    #[tokio::test]
    async fn test_build_model_state(
        current_model: &str,
        models: Result<Vec<String>, ProviderError>,
    ) -> Result<SessionModelState, sacp::Error> {
        let provider = MockModelProvider { models };
        build_model_state(&provider, current_model).await
    }
}
