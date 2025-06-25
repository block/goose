use crate::{
    agents::{extension_manager::ExtensionManager, Agent},
    message::{Message, MessageContent, ToolRequest},
    providers::base::Provider,
    providers::errors::ProviderError,
    recipe::Recipe,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use mcp_core::{handler::ToolError, role::Role, tool::Tool};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, instrument};
use uuid::Uuid;

use crate::agents::subagent_types::{SubAgentNotification, SubAgentUpdate, SubAgentUpdateType};

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
    pub notification_tx: mpsc::Sender<SubAgentNotification>, // For user-visible notifications
    pub update_tx: mpsc::Sender<SubAgentUpdate>,     // For main agent internal updates
}

impl SubAgent {
    /// Create a new subagent with the given configuration and provider
    #[instrument(skip(config, _provider, extension_manager, notification_tx, update_tx))]
    pub async fn new(
        config: SubAgentConfig,
        _provider: Arc<dyn Provider>,
        extension_manager: Arc<tokio::sync::RwLockReadGuard<'_, ExtensionManager>>,
        notification_tx: mpsc::Sender<SubAgentNotification>,
        update_tx: mpsc::Sender<SubAgentUpdate>,
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
            notification_tx,
            update_tx,
        });

        // Send initial notification
        let subagent_clone = Arc::clone(&subagent);
        subagent_clone
            .send_notification("Subagent created and ready".to_string(), false)
            .await;

        // Send initial update to main agent
        subagent_clone
            .send_update(
                SubAgentUpdateType::Progress,
                "Subagent initialized and ready".to_string(),
            )
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

        // Now send notifications without holding the lock
        match &status {
            SubAgentStatus::Completed(msg) => {
                self.send_notification(format!("Completed: {}", msg), true)
                    .await;

                self.send_update(
                    SubAgentUpdateType::Completion,
                    format!("Completed: {}", msg),
                )
                .await;
            }
            SubAgentStatus::Terminated => {
                self.send_notification("Terminated".to_string(), true).await;

                self.send_update(SubAgentUpdateType::Completion, "Terminated".to_string())
                    .await;
            }
            _ => {}
        }
    }

    /// Send a notification about the subagent's activity
    pub async fn send_notification(&self, message: String, is_complete: bool) {
        let notification = SubAgentNotification {
            subagent_id: self.id.clone(),
            message,
            timestamp: Utc::now(),
            is_complete,
        };

        if let Err(e) = self.notification_tx.send(notification).await {
            error!(
                "Failed to send notification from subagent {}: {}",
                self.id, e
            );
        }
    }

    /// Send an update to the main agent (not visible to user)
    pub async fn send_update(&self, update_type: SubAgentUpdateType, content: String) {
        let conversation = if update_type == SubAgentUpdateType::Completion
            || update_type == SubAgentUpdateType::Result
        {
            // Include full conversation for completion and results
            let conv = self.get_formatted_conversation().await;
            Some(conv)
        } else {
            None
        };

        let update = SubAgentUpdate {
            subagent_id: self.id.clone(),
            update_type,
            content,
            conversation,
            timestamp: Utc::now(),
        };

        if let Err(e) = self.update_tx.send(update).await {
            error!("Failed to send update from subagent {}: {}", self.id, e);
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
        self.send_notification(format!("Processing message: {}", message), false)
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
            self.send_notification(
                format!("Turn {}/{}", turn_count, self.config.max_turns.unwrap_or(0)),
                false,
            )
            .await;

            // Send update to main agent
            self.send_update(
                SubAgentUpdateType::Progress,
                format!(
                    "Processing turn {}/{}",
                    turn_count,
                    self.config.max_turns.unwrap_or(0)
                ),
            )
            .await;
        }

        // Get the current conversation for context
        let mut messages = self.get_conversation().await;

        // Get tools and system prompt from the extension manager
        let tools: Vec<Tool> = extension_manager.get_prefixed_tools(None).await?;
        let toolshim_tools: Vec<Tool> = vec![];

        // Build system prompt based on whether we have a recipe or direct instructions
        let instructions = if let Some(recipe) = &self.config.recipe {
            recipe.instructions.as_deref().unwrap_or("")
        } else {
            self.config.instructions.as_deref().unwrap_or("")
        };

        let system_prompt = format!(
            "You are a helpful subagent that was spawned by goose. You converse with goose and can use the following tools: {}\n\n{}",
            self.recipe_extensions.lock().await.join(", "),
            instructions
        );

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
                        self.send_notification(
                            format!("Responded: {}", response.as_concat_text()),
                            false,
                        )
                        .await;

                        // Send update to main agent with the result
                        self.send_update(
                            SubAgentUpdateType::Result,
                            format!("Result: {}", response.as_concat_text()),
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
                            self.send_notification(
                                format!("Using tool: {}", tool_call.name),
                                false,
                            )
                            .await;

                            match extension_manager
                                .dispatch_tool_call(tool_call.clone())
                                .await
                            {
                                Ok(result) => {
                                    let tool_response = result.result.await;

                                    // Create a user message with the tool response
                                    let tool_response_message = Message::user().with_tool_response(
                                        request.id.clone(),
                                        tool_response.clone(),
                                    );
                                    messages.push(tool_response_message);

                                    // Send notification about tool completion
                                    self.send_notification(
                                        format!("Tool {} completed", tool_call.name),
                                        false,
                                    )
                                    .await;

                                    // Send update to main agent
                                    self.send_update(
                                        SubAgentUpdateType::Progress,
                                        format!("Tool {} completed", tool_call.name),
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
                                    self.send_notification(
                                        format!("Tool {} error: {}", tool_call.name, e),
                                        false,
                                    )
                                    .await;

                                    // Send update to main agent
                                    self.send_update(
                                        SubAgentUpdateType::Error,
                                        format!("Tool {} error: {}", tool_call.name, e),
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
}
