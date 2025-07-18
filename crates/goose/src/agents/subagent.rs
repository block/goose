use crate::{
    agents::{Agent, TaskConfig},
    message::{Message, MessageContent, ToolRequest},
    prompt_template::render_global_file,
    providers::errors::ProviderError,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use mcp_core::protocol::{JsonRpcMessage, JsonRpcNotification};
use mcp_core::{handler::ToolError, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use std::fs::OpenOptions;
use std::io::Write;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, instrument};

/// Status of a subagent
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubAgentStatus {
    Ready,             // Ready to process messages
    Processing,        // Currently working on a task
    Completed(String), // Task completed (with optional message for success/error)
    Terminated,        // Manually terminated
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
    pub config: TaskConfig,
    pub turn_count: Arc<Mutex<usize>>,
    pub created_at: DateTime<Utc>,
}

impl SubAgent {
    /// Log conversation details to /tmp for debugging
    async fn log_conversation(&self, message: &str, details: &str) {
        let log_file = format!("/tmp/subagent_{}_conversation.log", self.id);
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S.%3f UTC");
        let log_entry = format!("[{}] {}: {}\n", timestamp, message, details);

        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
            let _ = file.write_all(log_entry.as_bytes());
        }
    }

    /// Create a new subagent with the given configuration and provider
    #[instrument(skip(task_config))]
    pub async fn new(
        task_config: TaskConfig,
    ) -> Result<(Arc<Self>, tokio::task::JoinHandle<()>), anyhow::Error> {
        debug!("Creating new subagent with id: {}", task_config.id);

        let subagent = Arc::new(SubAgent {
            id: task_config.id.clone(),
            conversation: Arc::new(Mutex::new(Vec::new())),
            status: Arc::new(RwLock::new(SubAgentStatus::Ready)),
            config: task_config,
            turn_count: Arc::new(Mutex::new(0)),
            created_at: Utc::now(),
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

        if let Err(e) = self.config.mcp_tx.send(notification).await {
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
    #[instrument(skip(self, message))]
    pub async fn reply_subagent(
        &self,
        message: String,
        _task_config: TaskConfig, // Unused parameter, kept for API compatibility
    ) -> Result<Message, anyhow::Error> {
        debug!("Processing message for subagent {}", self.id);
        self.log_conversation("START", &format!("Processing message: {}", message))
            .await;

        self.send_mcp_notification("message_processing", &format!("Processing: {}", message))
            .await;

        // Get provider and extension manager from self.config
        let provider = self
            .config
            .provider
            .as_ref()
            .ok_or_else(|| anyhow!("No provider configured for subagent"))?;

        let extension_manager = self
            .config
            .extension_manager
            .as_ref()
            .ok_or_else(|| anyhow!("No extension manager configured for subagent"))?;

        // Check if we've exceeded max turns
        {
            let turn_count = *self.turn_count.lock().await;
            if let Some(max_turns) = self.config.max_turns {
                if turn_count >= max_turns {
                    self.log_conversation(
                        "ERROR",
                        &format!("Maximum turns ({}) exceeded", max_turns),
                    )
                    .await;
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
            self.log_conversation(
                "TURN",
                &format!("Turn {}/{}", turn_count, self.config.max_turns.unwrap_or(0)),
            )
            .await;
            self.send_mcp_notification(
                "turn_progress",
                &format!("Turn {}/{}", turn_count, self.config.max_turns.unwrap_or(0)),
            )
            .await;
        }

        // Get the current conversation for context
        let mut messages = self.get_conversation().await;

        // Get tools based on whether we're using a recipe or inheriting from parent
        let tools: Vec<Tool> = extension_manager
            .read()
            .await
            .get_prefixed_tools(None)
            .await
            .unwrap_or_default();

        let toolshim_tools: Vec<Tool> = vec![];

        // Build system prompt using the template
        let system_prompt = self.build_system_prompt(&tools).await?;
        self.log_conversation(
            "SYSTEM_PROMPT",
            &format!("Available tools: {}", tools.len()),
        )
        .await;

        // Generate response from provider with proper loop management
        let max_tool_iterations = 20; // Prevent infinite loops
        let mut iteration_count = 0;

        loop {
            iteration_count += 1;
            if iteration_count > max_tool_iterations {
                self.log_conversation(
                    "ERROR",
                    &format!("Maximum tool iterations ({}) exceeded", max_tool_iterations),
                )
                .await;
                self.set_status(SubAgentStatus::Completed(
                    "Maximum tool iterations exceeded".to_string(),
                ))
                .await;
                return Ok(Message::assistant().with_text("I've reached the maximum number of tool iterations. The task may be too complex or there might be an issue with the tool chain."));
            }

            self.log_conversation(
                "LOOP_START",
                &format!(
                    "Starting iteration {}/{}",
                    iteration_count, max_tool_iterations
                ),
            )
            .await;

            match Agent::generate_response_from_provider(
                Arc::clone(provider),
                &system_prompt,
                &messages,
                &tools,
                &toolshim_tools,
            )
            .await
            {
                Ok((response, _usage)) => {
                    self.log_conversation(
                        "PROVIDER_RESPONSE",
                        &format!("Response length: {} chars", response.as_concat_text().len()),
                    )
                    .await;

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

                    self.log_conversation(
                        "TOOL_REQUESTS",
                        &format!("Found {} tool requests", tool_requests.len()),
                    )
                    .await;

                    // If there are no tool requests, we're done
                    if tool_requests.is_empty() {
                        self.log_conversation("COMPLETE", "No tool requests, completing")
                            .await;

                        // Add the final response to both local messages and persistent conversation
                        messages.push(response.clone());
                        self.add_message(response.clone()).await;

                        // Send notification about response
                        self.send_mcp_notification(
                            "response_generated",
                            &format!("Responded: {}", response.as_concat_text()),
                        )
                        .await;

                        // Set status to completed and return the final response
                        self.set_status(SubAgentStatus::Completed(
                            "Task completed successfully".to_string(),
                        ))
                        .await;
                        return Ok(response);
                    }

                    // Add the assistant message with tool calls to the local conversation
                    messages.push(response.clone());

                    // Process tool calls with limits
                    let max_tool_calls_per_iteration = 5; // Reduced from 10 to be more conservative
                    let mut successful_tool_calls = 0;
                    let mut failed_tool_calls = 0;

                    for (tool_index, request) in tool_requests.iter().enumerate() {
                        if tool_index >= max_tool_calls_per_iteration {
                            self.log_conversation(
                                "TOOL_LIMIT",
                                &format!(
                                    "Reached limit of {} tool calls per iteration",
                                    max_tool_calls_per_iteration
                                ),
                            )
                            .await;
                            break;
                        }

                        if let Ok(tool_call) = &request.tool_call {
                            self.log_conversation(
                                "TOOL_CALL",
                                &format!("Tool {}: {}", tool_index + 1, tool_call.name),
                            )
                            .await;

                            // Send notification about tool usage
                            self.send_mcp_notification(
                                "tool_usage",
                                &format!("Using tool: {}", tool_call.name),
                            )
                            .await;

                            // Handle platform tools or dispatch to extension manager
                            let tool_result = match extension_manager
                                .read()
                                .await
                                .dispatch_tool_call(tool_call.clone())
                                .await
                            {
                                Ok(result) => result.result.await,
                                Err(e) => Err(ToolError::ExecutionError(e.to_string())),
                            };

                            match tool_result {
                                Ok(result) => {
                                    successful_tool_calls += 1;
                                    self.log_conversation(
                                        "TOOL_SUCCESS",
                                        &format!("Tool {} completed", tool_call.name),
                                    )
                                    .await;

                                    // Create a user message with tool response content
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
                                    failed_tool_calls += 1;
                                    self.log_conversation(
                                        "TOOL_ERROR",
                                        &format!("Tool {} failed: {}", tool_call.name, e),
                                    )
                                    .await;

                                    // Create a user message with tool error content
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

                    self.log_conversation(
                        "TOOL_SUMMARY",
                        &format!(
                            "Iteration {}: {} successful, {} failed tool calls",
                            iteration_count, successful_tool_calls, failed_tool_calls
                        ),
                    )
                    .await;

                    // If all tool calls failed, we should probably stop to avoid infinite loops
                    if successful_tool_calls == 0 && failed_tool_calls > 0 {
                        self.log_conversation(
                            "WARNING",
                            "All tool calls failed, considering completion",
                        )
                        .await;
                        // Add a simple completion message and break
                        let completion_message = Message::assistant().with_text("I encountered issues with the tools and cannot complete the task as requested.");
                        self.add_message(completion_message.clone()).await;
                        self.set_status(SubAgentStatus::Completed(
                            "Task failed due to tool errors".to_string(),
                        ))
                        .await;
                        return Ok(completion_message);
                    }

                    // Continue the loop to get the next response from the provider
                    self.log_conversation(
                        "LOOP_CONTINUE",
                        &format!("Continuing to iteration {}", iteration_count + 1),
                    )
                    .await;
                }
                Err(ProviderError::ContextLengthExceeded(_)) => {
                    self.log_conversation("ERROR", "Context length exceeded")
                        .await;
                    self.set_status(SubAgentStatus::Completed(
                        "Context length exceeded".to_string(),
                    ))
                    .await;
                    return Ok(Message::assistant().with_context_length_exceeded(
                        "The context length of the model has been exceeded. Please start a new session and try again.",
                    ));
                }
                Err(ProviderError::RateLimitExceeded(_)) => {
                    self.log_conversation("ERROR", "Rate limit exceeded").await;
                    self.set_status(SubAgentStatus::Completed("Rate limit exceeded".to_string()))
                        .await;
                    return Ok(Message::assistant()
                        .with_text("Rate limit exceeded. Please try again later."));
                }
                Err(e) => {
                    self.log_conversation("ERROR", &format!("Provider error: {}", e))
                        .await;
                    self.set_status(SubAgentStatus::Completed(format!("Error: {}", e)))
                        .await;
                    error!("Error: {}", e);
                    return Ok(Message::assistant().with_text(format!("Ran into this error: {e}.\n\nPlease retry if you think this is a transient or recoverable error.")));
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

    /// Filter out subagent spawning tools to prevent infinite recursion
    fn _filter_subagent_tools(tools: Vec<Tool>) -> Vec<Tool> {
        // TODO: add this in subagent loop
        tools
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
