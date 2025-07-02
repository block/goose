use crate::{
    agents::{extension_manager::ExtensionManager, Agent},
    message::{Message, MessageContent, ToolRequest},
    prompt_template::render_global_file,
    providers::base::Provider,
    providers::errors::ProviderError,
    recipe::Recipe,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use mcp_core::protocol::{JsonRpcMessage, JsonRpcNotification};
use mcp_core::{handler::ToolError, role::Role, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, instrument};
use uuid::Uuid;

use crate::agents::platform_tools::{
    self, PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_READ_RESOURCE_TOOL_NAME,
    PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use crate::agents::subagent_tools::SUBAGENT_RUN_TASK_TOOL_NAME;

/// Status of a subagent
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubAgentStatus {
    Ready,             // Ready to process messages
    Processing,        // Currently working on a task
    Completed(String), // Task completed (with optional message for success/error)
    Terminated,        // Manually terminated
}

/// Configuration for a subagent
#[derive(Debug)]
pub struct SubAgentConfig {
    pub id: String,
    pub recipe: Option<Recipe>,
    pub instructions: Option<String>,
    pub max_turns: Option<usize>,
    pub timeout_seconds: Option<u64>,
}

impl SubAgentConfig {
    pub fn new_with_recipe(recipe: Recipe) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            recipe: Some(recipe),
            instructions: None,
            max_turns: None,
            timeout_seconds: None,
        }
    }

    pub fn new_with_instructions(instructions: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            recipe: None,
            instructions: Some(instructions),
            max_turns: None,
            timeout_seconds: None,
        }
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = Some(max_turns);
        self
    }

    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self
    }
}

/// Progress information for a subagent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentProgress {
    pub subagent_id: String,
    pub status: SubAgentStatus,
    pub message: String,
    pub turn: usize,
    pub max_turns: Option<usize>,
    pub timestamp: DateTime<Utc>,
}

/// A specialized agent that can handle specific tasks independently
pub struct SubAgent {
    pub id: String,
    pub conversation: Arc<Mutex<Vec<Message>>>,
    pub status: Arc<RwLock<SubAgentStatus>>,
    pub config: SubAgentConfig,
    pub turn_count: Arc<Mutex<usize>>,
    pub created_at: DateTime<Utc>,
    pub recipe_extensions: Arc<Mutex<Vec<String>>>,
    pub missing_extensions: Arc<Mutex<Vec<String>>>, // Track extensions that weren't enabled
    pub mcp_notification_tx: mpsc::Sender<JsonRpcMessage>, // For MCP notifications
}

impl SubAgent {
    /// Create a new subagent with the given configuration and provider
    #[instrument(skip(config, _provider, extension_manager, mcp_notification_tx))]
    pub async fn new(
        config: SubAgentConfig,
        _provider: Arc<dyn Provider>,
        extension_manager: Arc<tokio::sync::RwLockReadGuard<'_, ExtensionManager>>,
        mcp_notification_tx: mpsc::Sender<JsonRpcMessage>,
    ) -> Result<(Arc<Self>, tokio::task::JoinHandle<()>), anyhow::Error> {
        debug!("Creating new subagent with id: {}", config.id);

        let mut missing_extensions = Vec::new();
        let mut recipe_extensions = Vec::new();

        // Check if extensions from recipe exist in the extension manager
        if let Some(recipe) = &config.recipe {
            if let Some(extensions) = &recipe.extensions {
                for extension in extensions {
                    let extension_name = extension.name();
                    let existing_extensions = extension_manager.list_extensions().await?;

                    if !existing_extensions.contains(&extension_name) {
                        missing_extensions.push(extension_name);
                    } else {
                        recipe_extensions.push(extension_name);
                    }
                }
            }
        } else {
            // If no recipe, inherit all extensions from the parent agent
            let existing_extensions = extension_manager.list_extensions().await?;
            recipe_extensions = existing_extensions;
        }

        let subagent = Arc::new(SubAgent {
            id: config.id.clone(),
            conversation: Arc::new(Mutex::new(Vec::new())),
            status: Arc::new(RwLock::new(SubAgentStatus::Ready)),
            config,
            turn_count: Arc::new(Mutex::new(0)),
            created_at: Utc::now(),
            recipe_extensions: Arc::new(Mutex::new(recipe_extensions)),
            missing_extensions: Arc::new(Mutex::new(missing_extensions)),
            mcp_notification_tx,
        });

        // Send initial MCP notification
        let subagent_clone = Arc::clone(&subagent);
        subagent_clone
            .send_mcp_notification("subagent_created", "Subagent created and ready")
            .await;

        // Create a background task handle (for future use with streaming/monitoring)
        let subagent_clone = Arc::clone(&subagent);
        let handle = tokio::spawn(async move {
            // This could be used for background monitoring, cleanup, etc.
            debug!("Subagent {} background task started", subagent_clone.id);
        });

        debug!("Subagent {} created successfully", subagent.id);
        Ok((subagent, handle))
    }

    /// Get the current status of the subagent
    pub async fn get_status(&self) -> SubAgentStatus {
        self.status.read().await.clone()
    }

    /// Update the status of the subagent
    async fn set_status(&self, status: SubAgentStatus) {
        // Update the status first, then release the lock
        {
            let mut current_status = self.status.write().await;
            *current_status = status.clone();
        } // Write lock is released here!

        // Send MCP notifications based on status
        match &status {
            SubAgentStatus::Processing => {
                self.send_mcp_notification("status_changed", "Processing request")
                    .await;
            }
            SubAgentStatus::Completed(msg) => {
                self.send_mcp_notification("completed", &format!("Completed: {}", msg))
                    .await;
            }
            SubAgentStatus::Terminated => {
                self.send_mcp_notification("terminated", "Subagent terminated")
                    .await;
            }
            _ => {}
        }
    }

    /// Send an MCP notification about the subagent's activity
    pub async fn send_mcp_notification(&self, notification_type: &str, message: &str) {
        let notification = JsonRpcMessage::Notification(JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "notifications/message".to_string(),
            params: Some(json!({
                "level": "info",
                "logger": format!("subagent_{}", self.id),
                "data": {
                    "subagent_id": self.id,
                    "type": notification_type,
                    "message": message,
                    "timestamp": Utc::now().to_rfc3339()
                }
            })),
        });

        if let Err(e) = self.mcp_notification_tx.send(notification).await {
            error!(
                "Failed to send MCP notification from subagent {}: {}",
                self.id, e
            );
        }
    }

    /// Get current progress information
    pub async fn get_progress(&self) -> SubAgentProgress {
        let status = self.get_status().await;
        let turn_count = *self.turn_count.lock().await;

        SubAgentProgress {
            subagent_id: self.id.clone(),
            status: status.clone(),
            message: match &status {
                SubAgentStatus::Ready => "Ready to process messages".to_string(),
                SubAgentStatus::Processing => "Processing request...".to_string(),
                SubAgentStatus::Completed(msg) => msg.clone(),
                SubAgentStatus::Terminated => "Subagent terminated".to_string(),
            },
            turn: turn_count,
            max_turns: self.config.max_turns,
            timestamp: Utc::now(),
        }
    }

    /// Process a message and generate a response using the subagent's provider
    #[instrument(skip(self, message, provider, extension_manager))]
    pub async fn reply_subagent(
        &self,
        message: String,
        provider: Arc<dyn Provider>,
        extension_manager: Arc<tokio::sync::RwLockReadGuard<'_, ExtensionManager>>,
    ) -> Result<Message, anyhow::Error> {
        debug!("Processing message for subagent {}", self.id);
        self.send_mcp_notification("message_processing", &format!("Processing: {}", message))
            .await;

        // Check if we've exceeded max turns
        {
            let turn_count = *self.turn_count.lock().await;
            if let Some(max_turns) = self.config.max_turns {
                if turn_count >= max_turns {
                    self.set_status(SubAgentStatus::Completed(
                        "Maximum turns exceeded".to_string(),
                    ))
                    .await;
                    return Err(anyhow!("Maximum turns ({}) exceeded", max_turns));
                }
            }
        }

        // Set status to processing
        self.set_status(SubAgentStatus::Processing).await;

        // Add user message to conversation
        let user_message = Message::user().with_text(message.clone());
        {
            let mut conversation = self.conversation.lock().await;
            conversation.push(user_message.clone());
        }

        // Increment turn count
        {
            let mut turn_count = self.turn_count.lock().await;
            *turn_count += 1;
            self.send_mcp_notification(
                "turn_progress",
                &format!("Turn {}/{}", turn_count, self.config.max_turns.unwrap_or(0)),
            )
            .await;
        }

        // Get the current conversation for context
        let mut messages = self.get_conversation().await;

        // Get tools based on whether we're using a recipe or inheriting from parent
        let tools: Vec<Tool> = if self.config.recipe.is_some() {
            // Recipe mode: only get tools from the recipe's extensions
            let recipe_extensions = self.recipe_extensions.lock().await;
            let mut recipe_tools = Vec::new();

            debug!(
                "Subagent {} operating in recipe mode with {} extensions",
                self.id,
                recipe_extensions.len()
            );

            for extension_name in recipe_extensions.iter() {
                match extension_manager
                    .get_prefixed_tools(Some(extension_name.clone()))
                    .await
                {
                    Ok(mut ext_tools) => {
                        debug!(
                            "Added {} tools from extension {}",
                            ext_tools.len(),
                            extension_name
                        );
                        recipe_tools.append(&mut ext_tools);
                    }
                    Err(e) => {
                        debug!(
                            "Failed to get tools for extension {}: {}",
                            extension_name, e
                        );
                    }
                }
            }

            debug!(
                "Subagent {} has {} total recipe tools before filtering",
                self.id,
                recipe_tools.len()
            );
            // Filter out subagent tools from recipe tools
            let mut filtered_tools = Self::filter_subagent_tools(recipe_tools);

            // Add platform tools (except subagent tools)
            Self::add_platform_tools(&mut filtered_tools, &extension_manager).await;

            debug!(
                "Subagent {} has {} tools after filtering and adding platform tools",
                self.id,
                filtered_tools.len()
            );
            filtered_tools
        } else {
            // No recipe: inherit all tools from parent (but filter out subagent tools)
            debug!(
                "Subagent {} operating in inheritance mode, using all parent tools",
                self.id
            );
            let parent_tools = extension_manager.get_prefixed_tools(None).await?;
            debug!(
                "Subagent {} has {} parent tools before filtering",
                self.id,
                parent_tools.len()
            );
            let mut filtered_tools = Self::filter_subagent_tools(parent_tools);

            // Add platform tools (except subagent tools)
            Self::add_platform_tools(&mut filtered_tools, &extension_manager).await;

            debug!(
                "Subagent {} has {} tools after filtering and adding platform tools",
                self.id,
                filtered_tools.len()
            );
            filtered_tools
        };

        let toolshim_tools: Vec<Tool> = vec![];

        // Build system prompt using the template
        let system_prompt = self.build_system_prompt(&tools).await?;

        // Generate response from provider
        loop {
            match Agent::generate_response_from_provider(
                Arc::clone(&provider),
                &system_prompt,
                &messages,
                &tools,
                &toolshim_tools,
            )
            .await
            {
                Ok((response, _usage)) => {
                    // Process any tool calls in the response
                    let tool_requests: Vec<ToolRequest> = response
                        .content
                        .iter()
                        .filter_map(|content| {
                            if let MessageContent::ToolRequest(req) = content {
                                Some(req.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    // If there are no tool requests, we're done
                    if tool_requests.is_empty() {
                        self.add_message(response.clone()).await;

                        // Send notification about response
                        self.send_mcp_notification(
                            "response_generated",
                            &format!("Responded: {}", response.as_concat_text()),
                        )
                        .await;

                        // Add delay before completion to ensure all processing finishes
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                        // Set status back to ready and return the final response
                        self.set_status(SubAgentStatus::Completed("Completed!".to_string()))
                            .await;
                        break Ok(response);
                    }

                    // Add the assistant message with tool calls to the conversation
                    messages.push(response.clone());

                    // Process each tool request and create user response messages
                    for request in &tool_requests {
                        if let Ok(tool_call) = &request.tool_call {
                            // Send notification about tool usage
                            self.send_mcp_notification(
                                "tool_usage",
                                &format!("Using tool: {}", tool_call.name),
                            )
                            .await;

                            // Handle platform tools or dispatch to extension manager
                            let tool_result = if self.is_platform_tool(&tool_call.name) {
                                self.handle_platform_tool_call(
                                    tool_call.clone(),
                                    &extension_manager,
                                )
                                .await
                            } else {
                                match extension_manager
                                    .dispatch_tool_call(tool_call.clone())
                                    .await
                                {
                                    Ok(result) => result.result.await,
                                    Err(e) => Err(ToolError::ExecutionError(e.to_string())),
                                }
                            };

                            match tool_result {
                                Ok(result) => {
                                    // Create a user message with the tool response
                                    let tool_response_message = Message::user()
                                        .with_tool_response(request.id.clone(), Ok(result.clone()));
                                    messages.push(tool_response_message);

                                    // Send notification about tool completion
                                    self.send_mcp_notification(
                                        "tool_completed",
                                        &format!("Tool {} completed successfully", tool_call.name),
                                    )
                                    .await;
                                }
                                Err(e) => {
                                    // Create a user message with the tool error
                                    let tool_error_message = Message::user().with_tool_response(
                                        request.id.clone(),
                                        Err(ToolError::ExecutionError(e.to_string())),
                                    );
                                    messages.push(tool_error_message);

                                    // Send notification about tool error
                                    self.send_mcp_notification(
                                        "tool_error",
                                        &format!("Tool {} error: {}", tool_call.name, e),
                                    )
                                    .await;
                                }
                            }
                        }
                    }

                    // Continue the loop to get the next response from the provider
                }
                Err(ProviderError::ContextLengthExceeded(_)) => {
                    self.set_status(SubAgentStatus::Completed(
                        "Context length exceeded".to_string(),
                    ))
                    .await;
                    break Ok(Message::assistant().with_context_length_exceeded(
                        "The context length of the model has been exceeded. Please start a new session and try again.",
                    ));
                }
                Err(ProviderError::RateLimitExceeded(_)) => {
                    self.set_status(SubAgentStatus::Completed("Rate limit exceeded".to_string()))
                        .await;
                    break Ok(Message::assistant()
                        .with_text("Rate limit exceeded. Please try again later."));
                }
                Err(e) => {
                    self.set_status(SubAgentStatus::Completed(format!("Error: {}", e)))
                        .await;
                    error!("Error: {}", e);
                    break Ok(Message::assistant().with_text(format!("Ran into this error: {e}.\n\nPlease retry if you think this is a transient or recoverable error.")));
                }
            }
        }
    }

    /// Add a message to the conversation (for tracking agent responses)
    pub async fn add_message(&self, message: Message) {
        let mut conversation = self.conversation.lock().await;
        conversation.push(message);
    }

    /// Get the full conversation history
    pub async fn get_conversation(&self) -> Vec<Message> {
        self.conversation.lock().await.clone()
    }

    /// Check if the subagent has completed its task
    pub async fn is_completed(&self) -> bool {
        matches!(
            self.get_status().await,
            SubAgentStatus::Completed(_) | SubAgentStatus::Terminated
        )
    }

    /// Terminate the subagent
    pub async fn terminate(&self) -> Result<(), anyhow::Error> {
        debug!("Terminating subagent {}", self.id);
        self.set_status(SubAgentStatus::Terminated).await;
        Ok(())
    }

    /// Get formatted conversation for display
    pub async fn get_formatted_conversation(&self) -> String {
        let conversation = self.conversation.lock().await;

        let mut formatted = format!("=== Subagent {} Conversation ===\n", self.id);

        if let Some(recipe) = &self.config.recipe {
            formatted.push_str(&format!("Recipe: {}\n", recipe.title));
        } else if let Some(instructions) = &self.config.instructions {
            formatted.push_str(&format!("Instructions: {}\n", instructions));
        } else {
            formatted.push_str("Mode: Ad-hoc subagent\n");
        }

        formatted.push_str(&format!(
            "Created: {}\n",
            self.created_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        let progress = self.get_progress().await;

        formatted.push_str(&format!("Status: {:?}\n", progress.status));
        formatted.push_str(&format!("Turn: {}", progress.turn));
        if let Some(max_turns) = progress.max_turns {
            formatted.push_str(&format!("/{}", max_turns));
        }
        formatted.push_str("\n\n");

        for (i, message) in conversation.iter().enumerate() {
            formatted.push_str(&format!(
                "{}. {}: {}\n",
                i + 1,
                match message.role {
                    Role::User => "User",
                    Role::Assistant => "Assistant",
                },
                message.as_concat_text()
            ));
        }

        formatted.push_str("=== End Conversation ===\n");

        formatted
    }

    /// Get the list of extensions that weren't enabled
    pub async fn get_missing_extensions(&self) -> Vec<String> {
        self.missing_extensions.lock().await.clone()
    }

    /// Filter out subagent spawning tools to prevent infinite recursion
    fn filter_subagent_tools(tools: Vec<Tool>) -> Vec<Tool> {
        let original_count = tools.len();
        let filtered_tools: Vec<Tool> = tools
            .into_iter()
            .filter(|tool| {
                let should_keep = tool.name != SUBAGENT_RUN_TASK_TOOL_NAME;
                if !should_keep {
                    debug!("Filtering out subagent tool: {}", tool.name);
                }
                should_keep
            })
            .collect();

        let filtered_count = filtered_tools.len();
        if filtered_count < original_count {
            debug!(
                "Filtered {} subagent tool(s) from {} total tools",
                original_count - filtered_count,
                original_count
            );
        }

        filtered_tools
    }

    /// Add platform tools to the subagent's tool list (excluding dangerous tools)
    async fn add_platform_tools(tools: &mut Vec<Tool>, extension_manager: &ExtensionManager) {
        debug!("Adding safe platform tools to subagent");

        // Add safe platform tools - subagents can search for extensions but can't manage them or schedules
        tools.push(platform_tools::search_available_extensions_tool());
        debug!("Added search_available_extensions tool");

        // Add resource tools if supported - these are generally safe for subagents
        if extension_manager.supports_resources() {
            tools.extend([
                platform_tools::read_resource_tool(),
                platform_tools::list_resources_tool(),
            ]);
            debug!("Added 2 resource platform tools");
        }

        // Note: We explicitly do NOT add these tools for security reasons:
        // - manage_extensions (could interfere with parent agent's extensions)
        // - manage_schedule (could interfere with parent agent's scheduling)
        // - subagent spawning tools (prevent recursion)
        debug!("Platform tools added successfully (dangerous tools excluded)");
    }

    /// Check if a tool name is a platform tool that subagents can use
    fn is_platform_tool(&self, tool_name: &str) -> bool {
        matches!(
            tool_name,
            PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME
                | PLATFORM_READ_RESOURCE_TOOL_NAME
                | PLATFORM_LIST_RESOURCES_TOOL_NAME
        )
    }

    /// Handle platform tool calls that are safe for subagents
    async fn handle_platform_tool_call(
        &self,
        tool_call: mcp_core::tool::ToolCall,
        extension_manager: &ExtensionManager,
    ) -> Result<Vec<mcp_core::Content>, ToolError> {
        debug!("Handling platform tool: {}", tool_call.name);

        match tool_call.name.as_str() {
            PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME => extension_manager
                .search_available_extensions()
                .await
                .map_err(|e| ToolError::ExecutionError(e.to_string())),
            PLATFORM_READ_RESOURCE_TOOL_NAME => extension_manager
                .read_resource(tool_call.arguments)
                .await
                .map_err(|e| ToolError::ExecutionError(e.to_string())),
            PLATFORM_LIST_RESOURCES_TOOL_NAME => extension_manager
                .list_resources(tool_call.arguments)
                .await
                .map_err(|e| ToolError::ExecutionError(e.to_string())),
            _ => Err(ToolError::ExecutionError(format!(
                "Platform tool '{}' is not available to subagents for security reasons",
                tool_call.name
            ))),
        }
    }

    /// Build the system prompt for the subagent using the template
    async fn build_system_prompt(&self, available_tools: &[Tool]) -> Result<String, anyhow::Error> {
        let mut context = HashMap::new();

        // Add basic context
        context.insert(
            "current_date_time",
            serde_json::Value::String(Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()),
        );
        context.insert("subagent_id", serde_json::Value::String(self.id.clone()));

        // Add recipe information if available
        if let Some(recipe) = &self.config.recipe {
            context.insert(
                "recipe_title",
                serde_json::Value::String(recipe.title.clone()),
            );
        }

        // Add max turns if configured
        if let Some(max_turns) = self.config.max_turns {
            context.insert(
                "max_turns",
                serde_json::Value::Number(serde_json::Number::from(max_turns)),
            );
        }

        // Add task instructions
        let instructions = if let Some(recipe) = &self.config.recipe {
            recipe.instructions.as_deref().unwrap_or("")
        } else {
            self.config.instructions.as_deref().unwrap_or("")
        };
        context.insert(
            "task_instructions",
            serde_json::Value::String(instructions.to_string()),
        );

        // Add available extensions (only if we have a recipe and extensions)
        if self.config.recipe.is_some() {
            let extensions: Vec<String> = self.recipe_extensions.lock().await.clone();
            if !extensions.is_empty() {
                context.insert(
                    "extensions",
                    serde_json::Value::Array(
                        extensions
                            .into_iter()
                            .map(serde_json::Value::String)
                            .collect(),
                    ),
                );
            }
        }

        // Add available tools with descriptions for better context
        let tools_with_descriptions: Vec<String> = available_tools
            .iter()
            .map(|t| {
                if t.description.is_empty() {
                    t.name.clone()
                } else {
                    format!("{}: {}", t.name, t.description)
                }
            })
            .collect();

        context.insert(
            "available_tools",
            serde_json::Value::String(if tools_with_descriptions.is_empty() {
                "None".to_string()
            } else {
                tools_with_descriptions.join(", ")
            }),
        );

        // Add tool count for context
        context.insert(
            "tool_count",
            serde_json::Value::Number(serde_json::Number::from(available_tools.len())),
        );

        // Render the subagent system prompt template
        let system_prompt = render_global_file("subagent_system.md", &context)
            .map_err(|e| anyhow!("Failed to render subagent system prompt: {}", e))?;

        Ok(system_prompt)
    }
}
