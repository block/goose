use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::stream::BoxStream;

use crate::config::permission::PermissionLevel;
use crate::config::{Config, ExtensionConfigManager, PermissionManager};
use crate::message::{Message, MessageContent, ToolRequest};
use crate::permission::permission_judge::check_tool_permissions;
use crate::permission::{Permission, PermissionConfirmation};
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::recipe::{Author, Recipe};
use crate::token_counter::TokenCounter;
use crate::truncate::{truncate_messages, OldestFirstTruncation};
use regex::Regex;
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, instrument, warn};

use crate::agents::extension::{ExtensionConfig, ExtensionResult, ToolInfo};
use crate::agents::extension_manager::{get_parameter_names, ExtensionManager};
use crate::agents::platform_tools::{
    PLATFORM_ENABLE_EXTENSION_TOOL_NAME, PLATFORM_LIST_RESOURCES_TOOL_NAME,
    PLATFORM_READ_RESOURCE_TOOL_NAME, PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use crate::agents::prompt_manager::PromptManager;
use crate::agents::types::SessionConfig;
use crate::agents::types::{FrontendTool, ToolResultReceiver};
use mcp_core::{
    prompt::Prompt, protocol::GetPromptResult, tool::Tool, Content, ToolError, ToolResult,
};

const MAX_TRUNCATION_ATTEMPTS: usize = 3;
const ESTIMATE_FACTOR_DECAY: f32 = 0.9;

/// The main goose Agent
pub struct Agent {
    pub(super) provider: Arc<dyn Provider>,
    pub(super) extension_manager: Mutex<ExtensionManager>,
    pub(super) frontend_tools: HashMap<String, FrontendTool>,
    pub(super) frontend_instructions: Option<String>,
    pub(super) prompt_manager: PromptManager,
    pub(super) token_counter: TokenCounter,
    pub(super) confirmation_tx: mpsc::Sender<(String, PermissionConfirmation)>,
    pub(super) confirmation_rx: Mutex<mpsc::Receiver<(String, PermissionConfirmation)>>,
    pub(super) tool_result_tx: mpsc::Sender<(String, ToolResult<Vec<Content>>)>,
    pub(super) tool_result_rx: ToolResultReceiver,
}

impl Agent {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        let token_counter = TokenCounter::new(provider.get_model_config().tokenizer_name());
        // Create channels with buffer size 32 (adjust if needed)
        let (confirm_tx, confirm_rx) = mpsc::channel(32);
        let (tool_tx, tool_rx) = mpsc::channel(32);

        Self {
            provider,
            extension_manager: Mutex::new(ExtensionManager::new()),
            frontend_tools: HashMap::new(),
            frontend_instructions: None,
            prompt_manager: PromptManager::new(),
            token_counter,
            confirmation_tx: confirm_tx,
            confirmation_rx: Mutex::new(confirm_rx),
            tool_result_tx: tool_tx,
            tool_result_rx: Arc::new(Mutex::new(tool_rx)),
        }
    }

    /// Get a reference count clone to the provider
    pub fn provider(&self) -> Arc<dyn Provider> {
        Arc::clone(&self.provider)
    }

    /// Check if a tool is a frontend tool
    pub fn is_frontend_tool(&self, name: &str) -> bool {
        self.frontend_tools.contains_key(name)
    }

    /// Get a reference to a frontend tool
    pub fn get_frontend_tool(&self, name: &str) -> Option<&FrontendTool> {
        self.frontend_tools.get(name)
    }

    /// Get all tools from all clients with proper prefixing
    pub async fn get_prefixed_tools(&mut self) -> ExtensionResult<Vec<Tool>> {
        let mut tools = self
            .extension_manager
            .lock()
            .await
            .get_prefixed_tools()
            .await?;

        // Add frontend tools directly - they don't need prefixing since they're already uniquely named
        for frontend_tool in self.frontend_tools.values() {
            tools.push(frontend_tool.tool.clone());
        }

        Ok(tools)
    }

    /// Dispatch a single tool call to the appropriate client
    #[instrument(skip(self, tool_call, request_id), fields(input, output))]
    async fn dispatch_tool_call(
        &self,
        tool_call: mcp_core::tool::ToolCall,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let extension_manager = self.extension_manager.lock().await;
        let result = if tool_call.name == PLATFORM_READ_RESOURCE_TOOL_NAME {
            // Check if the tool is read_resource and handle it separately
            extension_manager
                .read_resource(tool_call.arguments.clone())
                .await
        } else if tool_call.name == PLATFORM_LIST_RESOURCES_TOOL_NAME {
            extension_manager
                .list_resources(tool_call.arguments.clone())
                .await
        } else if tool_call.name == PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME {
            extension_manager.search_available_extensions().await
        } else if self.is_frontend_tool(&tool_call.name) {
            // For frontend tools, return an error indicating we need frontend execution
            Err(ToolError::ExecutionError(
                "Frontend tool execution required".to_string(),
            ))
        } else {
            extension_manager
                .dispatch_tool_call(tool_call.clone())
                .await
        };

        debug!(
            "input" = serde_json::to_string(&tool_call).unwrap(),
            "output" = serde_json::to_string(&result).unwrap(),
        );

        (request_id, result)
    }

    /// Truncates the messages to fit within the model's context window
    /// Ensures the last message is a user message and removes tool call-response pairs
    async fn truncate_messages(
        &self,
        messages: &mut Vec<Message>,
        estimate_factor: f32,
        system_prompt: &str,
        tools: &mut Vec<Tool>,
    ) -> anyhow::Result<()> {
        // Model's actual context limit
        let context_limit = self.provider.get_model_config().context_limit();

        // Our conservative estimate of the **target** context limit
        // Our token count is an estimate since model providers often don't provide the tokenizer (eg. Claude)
        let context_limit = (context_limit as f32 * estimate_factor) as usize;

        // Take into account the system prompt, and our tools input and subtract that from the
        // remaining context limit
        let system_prompt_token_count = self.token_counter.count_tokens(system_prompt);
        let tools_token_count = self.token_counter.count_tokens_for_tools(tools.as_slice());

        // Check if system prompt + tools exceed our context limit
        let remaining_tokens = context_limit
            .checked_sub(system_prompt_token_count)
            .and_then(|remaining| remaining.checked_sub(tools_token_count))
            .ok_or_else(|| {
                anyhow::anyhow!("System prompt and tools exceed estimated context limit")
            })?;

        let context_limit = remaining_tokens;

        // Calculate current token count of each message, use count_chat_tokens to ensure we
        // capture the full content of the message, include ToolRequests and ToolResponses
        let mut token_counts: Vec<usize> = messages
            .iter()
            .map(|msg| {
                self.token_counter
                    .count_chat_tokens("", std::slice::from_ref(msg), &[])
            })
            .collect();

        truncate_messages(
            messages,
            &mut token_counts,
            context_limit,
            &OldestFirstTruncation,
        )
    }

    async fn enable_extension(
        &self,
        extension_name: String,
        request_id: String,
    ) -> (String, Result<Vec<Content>, ToolError>) {
        let mut extension_manager = self.extension_manager.lock().await;
        let config = match ExtensionConfigManager::get_config_by_name(&extension_name) {
            Ok(Some(config)) => config,
            Ok(None) => {
                return (
                    request_id,
                    Err(ToolError::ExecutionError(format!(
                        "Extension '{}' not found. Please check the extension name and try again.",
                        extension_name
                    ))),
                )
            }
            Err(e) => {
                return (
                    request_id,
                    Err(ToolError::ExecutionError(format!(
                        "Failed to get extension config: {}",
                        e
                    ))),
                )
            }
        };

        let result = extension_manager
            .add_extension(config)
            .await
            .map(|_| {
                vec![Content::text(format!(
                    "The extension '{}' has been installed successfully",
                    extension_name
                ))]
            })
            .map_err(|e| ToolError::ExecutionError(e.to_string()));

        (request_id, result)
    }

    pub async fn add_extension(&mut self, extension: ExtensionConfig) -> ExtensionResult<()> {
        match &extension {
            ExtensionConfig::Frontend {
                name: _,
                tools,
                instructions,
                bundled: _,
            } => {
                // For frontend tools, just store them in the frontend_tools map
                for tool in tools {
                    let frontend_tool = FrontendTool {
                        name: tool.name.clone(),
                        tool: tool.clone(),
                    };
                    self.frontend_tools.insert(tool.name.clone(), frontend_tool);
                }
                // Store instructions if provided, using "frontend" as the key
                if let Some(instructions) = instructions {
                    self.frontend_instructions = Some(instructions.clone());
                } else {
                    // Default frontend instructions if none provided
                    self.frontend_instructions = Some(
                        "The following tools are provided directly by the frontend and will be executed by the frontend when called.".to_string(),
                    );
                }
            }
            _ => {
                let mut extension_manager = self.extension_manager.lock().await;
                let _ = extension_manager.add_extension(extension).await;
            }
        };

        Ok(())
    }

    pub async fn list_tools(&self) -> Vec<Tool> {
        let extension_manager = self.extension_manager.lock().await;
        extension_manager
            .get_prefixed_tools()
            .await
            .unwrap_or_default()
    }

    pub async fn remove_extension(&mut self, name: &str) {
        let mut extension_manager = self.extension_manager.lock().await;
        extension_manager
            .remove_extension(name)
            .await
            .expect("Failed to remove extension");
    }

    pub async fn list_extensions(&self) -> Vec<String> {
        let extension_manager = self.extension_manager.lock().await;
        extension_manager
            .list_extensions()
            .await
            .expect("Failed to list extensions")
    }

    /// Handle a confirmation response for a tool request
    pub async fn handle_confirmation(
        &self,
        request_id: String,
        confirmation: PermissionConfirmation,
    ) {
        if let Err(e) = self.confirmation_tx.send((request_id, confirmation)).await {
            error!("Failed to send confirmation: {}", e);
        }
    }

    #[instrument(skip(self, messages, session), fields(user_message))]
    pub async fn reply(
        &self,
        messages: &[Message],
        session: Option<SessionConfig>,
    ) -> anyhow::Result<BoxStream<'_, anyhow::Result<Message>>> {
        let mut messages = messages.to_vec();
        let reply_span = tracing::Span::current();
        let mut truncation_attempt: usize = 0;

        // Load settings from config
        let config = Config::global();

        // Setup tools and prompt
        let (mut tools, mut toolshim_tools, mut system_prompt) =
            self.prepare_tools_and_prompt().await?;

        let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());

        let (tools_with_readonly_annotation, tools_without_annotation) =
            Self::categorize_tools_by_annotation(&tools);

        if let Some(content) = messages
            .last()
            .and_then(|msg| msg.content.first())
            .and_then(|c| c.as_text())
        {
            debug!("user_message" = &content);
        }

        Ok(Box::pin(async_stream::try_stream! {
            let _ = reply_span.enter();
            loop {
                match Self::generate_response_from_provider(
                    self.provider(),
                    &system_prompt,
                    &messages,
                    &tools,
                    &toolshim_tools,
                ).await {
                    Ok((response, usage)) => {
                        // record usage for the session in the session file
                        if let Some(session_config) = session.clone() {
                            Self::update_session_metrics(session_config, &usage, messages.len()).await?;
                        }

                        // Reset truncation attempt
                        truncation_attempt = 0;

                        // Yield the assistant's response, but filter out frontend tool requests that we'll process separately
                        let filtered_response = Message {
                            role: response.role.clone(),
                            created: response.created,
                            content: response.content.iter().filter(|c| {
                                if let MessageContent::ToolRequest(req) = c {
                                    // Only filter out frontend tool requests
                                    if let Ok(tool_call) = &req.tool_call {
                                        return !self.is_frontend_tool(&tool_call.name);
                                    }
                                }
                                true
                            }).cloned().collect(),
                        };
                        yield filtered_response.clone();

                        tokio::task::yield_now().await;

                        // First collect any tool requests
                        let tool_requests: Vec<&ToolRequest> = response.content
                            .iter()
                            .filter_map(|content| content.as_tool_request())
                            .collect();

                        if tool_requests.is_empty() {
                            break;
                        }

                        // Process tool requests depending on goose_mode
                        let mut message_tool_response = Message::user();

                        // First handle any frontend tool requests
                        let mut remaining_requests = Vec::new();
                        for request in &tool_requests {
                            if let Ok(tool_call) = request.tool_call.clone() {
                                if self.is_frontend_tool(&tool_call.name) {
                                    // Send frontend tool request and wait for response
                                    yield Message::assistant().with_frontend_tool_request(
                                        request.id.clone(),
                                        Ok(tool_call.clone())
                                    );

                                    if let Some((id, result)) = self.tool_result_rx.lock().await.recv().await {
                                        message_tool_response = message_tool_response.with_tool_response(id, result);
                                    }
                                } else {
                                    remaining_requests.push(request);
                                }
                            } else {
                                remaining_requests.push(request);
                            }
                        }

                        // Clone goose_mode once before the match to avoid move issues
                        let mode = goose_mode.clone();
                        if mode.as_str() == "chat" {
                            // Skip all tool calls in chat mode
                            for request in remaining_requests {
                                message_tool_response = message_tool_response.with_tool_response(
                                    request.id.clone(),
                                    Ok(vec![Content::text(
                                        "Let the user know the tool call was skipped in Goose chat mode. \
                                        DO NOT apologize for skipping the tool call. DO NOT say sorry. \
                                        Provide an explanation of what the tool call would do, structured as a \
                                        plan for the user. Again, DO NOT apologize. \
                                        **Example Plan:**\n \
                                        1. **Identify Task Scope** - Determine the purpose and expected outcome.\n \
                                        2. **Outline Steps** - Break down the steps.\n \
                                        If needed, adjust the explanation based on user preferences or questions."
                                    )]),
                                );
                            }
                        } else {
                            // Split tool requests into enable_extension and others
                            let (enable_extension_requests, non_enable_extension_requests): (Vec<&ToolRequest>, Vec<&ToolRequest>) = remaining_requests.clone()
                                .into_iter()
                                .partition(|req| {
                                    req.tool_call.as_ref()
                                        .map(|call| call.name == PLATFORM_ENABLE_EXTENSION_TOOL_NAME)
                                        .unwrap_or(false)
                                });
                            let mut permission_manager = PermissionManager::default();
                            let permission_check_result = check_tool_permissions(non_enable_extension_requests,
                                                            &mode,
                                                            tools_with_readonly_annotation.clone(),
                                                            tools_without_annotation.clone(),
                                                            &mut permission_manager,
                                                            self.provider()).await;

                            // Handle pre-approved and read-only tools in parallel
                            let mut tool_futures = Vec::new();
                            let mut install_results = Vec::new();

                            let denied_content_text = Content::text(
                                "The user has declined to run this tool. \
                                DO NOT attempt to call this tool again. \
                                If there are no alternative methods to proceed, clearly explain the situation and STOP.");
                            // Handle install extension requests
                            for request in &enable_extension_requests {
                                if let Ok(tool_call) = request.tool_call.clone() {
                                    let confirmation = Message::user().with_enable_extension_request(
                                        request.id.clone(),
                                        tool_call.arguments.get("extension_name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string()
                                    );
                                    yield confirmation;

                                    let mut rx = self.confirmation_rx.lock().await;
                                    while let Some((req_id, extension_confirmation)) = rx.recv().await {
                                        let extension_name = tool_call.arguments.get("extension_name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        if req_id == request.id {
                                            if extension_confirmation.permission == Permission::AllowOnce || extension_confirmation.permission == Permission::AlwaysAllow {
                                                let install_result = self.enable_extension(extension_name, request.id.clone()).await;
                                                install_results.push(install_result);
                                            } else {
                                                // User declined - add declined response
                                                message_tool_response = message_tool_response.with_tool_response(
                                                    request.id.clone(),
                                                    Ok(vec![denied_content_text.clone()]),
                                                );
                                            }
                                            break;
                                        }
                                    }
                                }
                            }

                            // Skip the confirmation for approved tools
                            for request in &permission_check_result.approved {
                                if let Ok(tool_call) = request.tool_call.clone() {
                                    let tool_future = self.dispatch_tool_call(tool_call, request.id.clone());
                                     tool_futures.push(tool_future);
                                }
                            }

                            for request in &permission_check_result.denied {
                                message_tool_response = message_tool_response.with_tool_response(
                                    request.id.clone(),
                                    Ok(vec![denied_content_text.clone()]),
                                );
                            }

                            // Process read-only tools
                            for request in &permission_check_result.needs_approval {
                                if let Ok(tool_call) = request.tool_call.clone() {
                                    let confirmation = Message::user().with_tool_confirmation_request(
                                        request.id.clone(),
                                        tool_call.name.clone(),
                                        tool_call.arguments.clone(),
                                        Some("Goose would like to call the above tool. Allow? (y/n):".to_string()),
                                    );
                                    yield confirmation;

                                    // Wait for confirmation response through the channel
                                    let mut rx = self.confirmation_rx.lock().await;
                                    while let Some((req_id, tool_confirmation)) = rx.recv().await {
                                        if req_id == request.id {
                                            let confirmed = tool_confirmation.permission == Permission::AllowOnce || tool_confirmation.permission == Permission::AlwaysAllow;
                                            if confirmed {
                                                // Add this tool call to the futures collection
                                                let tool_future = self.dispatch_tool_call(tool_call.clone(), request.id.clone());
                                                tool_futures.push(tool_future);
                                                if tool_confirmation.permission == Permission::AlwaysAllow {
                                                    permission_manager.update_user_permission(&tool_call.name, PermissionLevel::AlwaysAllow);
                                                }
                                            } else {
                                                // User declined - add declined response
                                                message_tool_response = message_tool_response.with_tool_response(
                                                    request.id.clone(),
                                                    Ok(vec![denied_content_text.clone()]),
                                                );
                                            }
                                            break; // Exit the loop once the matching `req_id` is found
                                        }
                                    }
                                }
                            }

                            // Wait for all tool calls to complete
                            let results = futures::future::join_all(tool_futures).await;
                            for (request_id, output) in results {
                                message_tool_response = message_tool_response.with_tool_response(
                                    request_id,
                                    output,
                                );
                            }

                            // Check if any install results had errors before processing them
                            let all_install_successful = !install_results.iter().any(|(_, result)| result.is_err());
                            for (request_id, output) in install_results {
                                message_tool_response = message_tool_response.with_tool_response(
                                    request_id,
                                    output
                                );
                            }

                            // Update system prompt and tools if installations were successful
                            if all_install_successful {
                                (tools, toolshim_tools, system_prompt) = self.prepare_tools_and_prompt().await?;
                            }
                    }

                        yield message_tool_response.clone();

                        messages.push(response);
                        messages.push(message_tool_response);
                    },
                    Err(ProviderError::ContextLengthExceeded(_)) => {
                        if truncation_attempt >= MAX_TRUNCATION_ATTEMPTS {
                            // Create an error message & terminate the stream
                            // the previous message would have been a user message (e.g. before any tool calls, this is just after the input message.
                            // at the start of a loop after a tool call, it would be after a tool_use assistant followed by a tool_result user)
                            yield Message::assistant().with_text("Error: Context length exceeds limits even after multiple attempts to truncate. Please start a new session with fresh context and try again.");
                            break;
                        }
                        truncation_attempt += 1;
                        warn!("Context length exceeded. Truncation Attempt: {}/{}.", truncation_attempt, MAX_TRUNCATION_ATTEMPTS);
                        // Decay the estimate factor as we make more truncation attempts
                        // Estimate factor decays like this over time: 0.9, 0.81, 0.729, ...
                        let estimate_factor: f32 = ESTIMATE_FACTOR_DECAY.powi(truncation_attempt as i32);
                        if let Err(err) = self.truncate_messages(&mut messages, estimate_factor, &system_prompt, &mut tools).await {
                            yield Message::assistant().with_text(format!("Error: Unable to truncate messages to stay within context limit. \n\nRan into this error: {}.\n\nPlease start a new session with fresh context and try again.", err));
                            break;
                        }
                        // Retry the loop after truncation
                        continue;
                    },
                    Err(e) => {
                        // Create an error message & terminate the stream
                        error!("Error: {}", e);
                        yield Message::assistant().with_text(format!("Ran into this error: {e}.\n\nPlease retry if you think this is a transient or recoverable error."));
                        break;
                    }
                }

                // Yield control back to the scheduler to prevent blocking
                tokio::task::yield_now().await;
            }
        }))
    }

    /// Extend the system prompt with one line of additional instruction
    pub async fn extend_system_prompt(&mut self, instruction: String) {
        self.prompt_manager.add_system_prompt_extra(instruction);
    }

    /// Override the system prompt with a custom template
    pub async fn override_system_prompt(&mut self, template: String) {
        self.prompt_manager.set_system_prompt_override(template);
    }

    pub async fn list_extension_prompts(&self) -> HashMap<String, Vec<Prompt>> {
        let extension_manager = self.extension_manager.lock().await;
        extension_manager
            .list_prompts()
            .await
            .expect("Failed to list prompts")
    }

    pub async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult> {
        let extension_manager = self.extension_manager.lock().await;

        // First find which extension has this prompt
        let prompts = extension_manager
            .list_prompts()
            .await
            .map_err(|e| anyhow!("Failed to list prompts: {}", e))?;

        if let Some(extension) = prompts
            .iter()
            .find(|(_, prompt_list)| prompt_list.iter().any(|p| p.name == name))
            .map(|(extension, _)| extension)
        {
            return extension_manager
                .get_prompt(extension, name, arguments)
                .await
                .map_err(|e| anyhow!("Failed to get prompt: {}", e));
        }

        Err(anyhow!("Prompt '{}' not found", name))
    }

    pub async fn get_plan_prompt(&self) -> anyhow::Result<String> {
        let extension_manager = self.extension_manager.lock().await;
        let tools = extension_manager.get_prefixed_tools().await?;
        let tools_info = tools
            .into_iter()
            .map(|tool| {
                ToolInfo::new(
                    &tool.name,
                    &tool.description,
                    get_parameter_names(&tool),
                    None,
                )
            })
            .collect();

        let plan_prompt = extension_manager.get_planning_prompt(tools_info).await;

        Ok(plan_prompt)
    }

    pub async fn handle_tool_result(&self, id: String, result: ToolResult<Vec<Content>>) {
        if let Err(e) = self.tool_result_tx.send((id, result)).await {
            tracing::error!("Failed to send tool result: {}", e);
        }
    }

    pub async fn create_recipe(&self, mut messages: Vec<Message>) -> Result<Recipe> {
        let extension_manager = self.extension_manager.lock().await;
        let extensions_info = extension_manager.get_extensions_info().await;
        let system_prompt = self
            .prompt_manager
            .build_system_prompt(extensions_info, self.frontend_instructions.clone());

        let recipe_prompt = self.prompt_manager.get_recipe_prompt().await;
        let tools = extension_manager.get_prefixed_tools().await?;

        messages.push(Message::user().with_text(recipe_prompt));

        let (result, _usage) = self
            .provider
            .complete(&system_prompt, &messages, &tools)
            .await?;

        let content = result.as_concat_text();

        // the response may be contained in ```json ```, strip that before parsing json
        let re = Regex::new(r"(?s)```[^\n]*\n(.*?)\n```").unwrap();
        let clean_content = re
            .captures(&content)
            .and_then(|caps| caps.get(1).map(|m| m.as_str()))
            .unwrap_or(&content)
            .trim()
            .to_string();

        // try to parse json response from the LLM
        let (instructions, activities) =
            if let Ok(json_content) = serde_json::from_str::<Value>(&clean_content) {
                let instructions = json_content
                    .get("instructions")
                    .ok_or_else(|| anyhow!("Missing 'instructions' in json response"))?
                    .as_str()
                    .ok_or_else(|| anyhow!("instructions' is not a string"))?
                    .to_string();

                let activities = json_content
                    .get("activities")
                    .ok_or_else(|| anyhow!("Missing 'activities' in json response"))?
                    .as_array()
                    .ok_or_else(|| anyhow!("'activities' is not an array'"))?
                    .iter()
                    .map(|act| {
                        act.as_str()
                            .map(|s| s.to_string())
                            .ok_or(anyhow!("'activities' array element is not a string"))
                    })
                    .collect::<Result<_, _>>()?;

                (instructions, activities)
            } else {
                // If we can't get valid JSON, try string parsing
                // Use split_once to get the content after "Instructions:".
                let after_instructions = content
                    .split_once("instructions:")
                    .map(|(_, rest)| rest)
                    .unwrap_or(&content);

                // Split once more to separate instructions from activities.
                let (instructions_part, activities_text) = after_instructions
                    .split_once("activities:")
                    .unwrap_or((after_instructions, ""));

                let instructions = instructions_part
                    .trim_end_matches(|c: char| c.is_whitespace() || c == '#')
                    .trim()
                    .to_string();
                let activities_text = activities_text.trim();

                // Regex to remove bullet markers or numbers with an optional dot.
                let bullet_re = Regex::new(r"^[•\-\*\d]+\.?\s*").expect("Invalid regex");

                // Process each line in the activities section.
                let activities: Vec<String> = activities_text
                    .lines()
                    .map(|line| bullet_re.replace(line, "").to_string())
                    .map(|s| s.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();

                (instructions, activities)
            };

        let extensions = ExtensionConfigManager::get_all().unwrap_or_default();
        let extension_configs: Vec<_> = extensions
            .iter()
            .filter(|e| e.enabled)
            .map(|e| e.config.clone())
            .collect();

        let author = Author {
            contact: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .ok(),
            metadata: None,
        };

        let recipe = Recipe::builder()
            .title("Custom recipe from chat")
            .description("a custom recipe instance from this chat session")
            .instructions(instructions)
            .activities(activities)
            .extensions(extension_configs)
            .author(author)
            .build()
            .expect("valid recipe");

        Ok(recipe)
    }
}
