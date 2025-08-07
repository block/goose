use crate::agents::subagent_task_config::DEFAULT_SUBAGENT_MAX_TURNS;
use crate::{
    agents::extension::ExtensionConfig,
    agents::{extension_manager::ExtensionManager, Agent, TaskConfig},
    config::ExtensionConfigManager,
    prompt_template::render_global_file,
    providers::errors::ProviderError,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use mcp_core::handler::ToolError;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
// use serde_json::{self};
use crate::conversation::message::{Message, MessageContent, ToolRequest};
use crate::conversation::Conversation;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
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
    pub conversation: Arc<Mutex<Conversation>>,
    pub status: Arc<RwLock<SubAgentStatus>>,
    pub config: TaskConfig,
    pub turn_count: Arc<Mutex<usize>>,
    pub created_at: DateTime<Utc>,
    pub extension_manager: Arc<RwLock<ExtensionManager>>,
}

impl SubAgent {
    /// Create a new subagent with the given configuration and provider
    #[instrument(skip(task_config))]
    pub async fn new(task_config: TaskConfig) -> Result<Arc<Self>, anyhow::Error> {
        debug!("Creating new subagent with id: {}", task_config.id);

        // Create a new extension manager for this subagent
        let mut extension_manager = ExtensionManager::new();

        // Add extensions based on task_type:
        // 1. If executing dynamic task (task_type = 'text_instruction'), default to using all enabled extensions
        // 2. (TODO) If executing a sub-recipe task, only use recipe extensions

        // Get all enabled extensions from config
        let enabled_extensions = ExtensionConfigManager::get_all()
            .unwrap_or_default()
            .into_iter()
            .filter(|ext| ext.enabled)
            .map(|ext| ext.config)
            .collect::<Vec<ExtensionConfig>>();

        // Add enabled extensions to the subagent's extension manager
        for extension in enabled_extensions {
            if let Err(e) = extension_manager.add_extension(extension).await {
                debug!("Failed to add extension to subagent: {}", e);
                // Continue with other extensions even if one fails
            }
        }

        let subagent = Arc::new(SubAgent {
            id: task_config.id.clone(),
            conversation: Arc::new(Mutex::new(Conversation::new_unvalidated(Vec::new()))),
            status: Arc::new(RwLock::new(SubAgentStatus::Ready)),
            config: task_config,
            turn_count: Arc::new(Mutex::new(0)),
            created_at: Utc::now(),
            extension_manager: Arc::new(RwLock::new(extension_manager)),
        });

        debug!("Subagent {} created successfully", subagent.id);
        Ok(subagent)
    }

    /// Update the status of the subagent
    async fn set_status(&self, status: SubAgentStatus) {
        // Update the status first, then release the lock
        {
            let mut current_status = self.status.write().await;
            *current_status = status.clone();
        } // Write lock is released here!
    }

    /// Process a message and generate a response using the subagent's provider
    #[instrument(skip(self, message))]
    pub async fn reply_subagent(
        &self,
        message: String,
        task_config: TaskConfig,
    ) -> Result<Conversation, anyhow::Error> {
        debug!("Processing message for subagent {}", self.id);

        // Get provider from task config
        let provider = self
            .config
            .provider
            .as_ref()
            .ok_or_else(|| anyhow!("No provider configured for subagent"))?;

        // Set status to processing
        self.set_status(SubAgentStatus::Processing).await;

        // Add user message to conversation
        let user_message = Message::user().with_text(message.clone());
        {
            let mut conversation = self.conversation.lock().await;
            conversation.push(user_message.clone());
        }

        // Get the current conversation for context
        let mut messages = {
            let conversation = self.conversation.lock().await;
            conversation.clone()
        };

        // Get tools from the subagent's own extension manager
        let tools: Vec<Tool> = self
            .extension_manager
            .read()
            .await
            .get_prefixed_tools(None)
            .await
            .unwrap_or_default();

        let toolshim_tools: Vec<Tool> = vec![];

        // Build system prompt using the template
        let system_prompt = self.build_system_prompt(&tools).await?;

        // Generate response from provider with loop for tool processing (max_turns iterations)
        let mut loop_count = 0;
        let max_turns = self.config.max_turns.unwrap_or(DEFAULT_SUBAGENT_MAX_TURNS);
        let mut last_error: Option<anyhow::Error> = None;

        // Generate response from provider
        loop {
            loop_count += 1;

            match Agent::generate_response_from_provider(
                Arc::clone(provider),
                &system_prompt,
                messages.messages(),
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
                    if tool_requests.is_empty() || loop_count >= max_turns {
                        self.add_message(response.clone()).await;
                        messages.push(response.clone());

                        // Set status back to ready
                        self.set_status(SubAgentStatus::Completed("Completed!".to_string()))
                            .await;
                        break;
                    }

                    // Add the assistant message with tool calls to the conversation
                    messages.push(response.clone());

                    // Process each tool request and create user response messages
                    for request in &tool_requests {
                        if let Ok(tool_call) = &request.tool_call {
                            // Handle platform tools or dispatch to extension manager
                            let tool_result = match self
                                .extension_manager
                                .read()
                                .await
                                .dispatch_tool_call(tool_call.clone(), CancellationToken::default())
                                .await
                            {
                                Ok(result) => result.result.await,
                                Err(e) => Err(ToolError::ExecutionError(e.to_string())),
                            };

                            match tool_result {
                                Ok(result) => {
                                    // Create a user message with the tool response
                                    let tool_response_message = Message::user()
                                        .with_tool_response(request.id.clone(), Ok(result.clone()));
                                    messages.push(tool_response_message);
                                }
                                Err(e) => {
                                    // Create a user message with the tool error
                                    let tool_error_message = Message::user().with_tool_response(
                                        request.id.clone(),
                                        Err(ToolError::ExecutionError(e.to_string())),
                                    );
                                    messages.push(tool_error_message);
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
                    last_error = Some(anyhow::anyhow!("Context length exceeded"));
                    break;
                }
                Err(ProviderError::RateLimitExceeded(_)) => {
                    self.set_status(SubAgentStatus::Completed("Rate limit exceeded".to_string()))
                        .await;
                    last_error = Some(anyhow::anyhow!("Rate limit exceeded"));
                    break;
                }
                Err(e) => {
                    self.set_status(SubAgentStatus::Completed(format!("Error: {}", e)))
                        .await;
                    error!("Error: {}", e);
                    last_error = Some(anyhow::anyhow!("Provider error: {}", e));
                    break;
                }
            }
        }

        // Handle error cases or return the last message
        if let Some(error) = last_error {
            Err(error)
        } else {
            Ok(messages)
        }
    }

    /// Add a message to the conversation (for tracking agent responses)
    async fn add_message(&self, message: Message) {
        let mut conversation = self.conversation.lock().await;
        conversation.push(message);
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

        // Add max turns if configured
        if let Some(max_turns) = self.config.max_turns {
            context.insert(
                "max_turns",
                serde_json::Value::Number(serde_json::Number::from(max_turns)),
            );
        }

        // Add available tools with descriptions for better context
        let tools_with_descriptions: Vec<String> = available_tools
            .iter()
            .map(|t| {
                if let Some(description) = &t.description {
                    format!("{}: {}", t.name, description)
                } else {
                    t.name.to_string()
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
