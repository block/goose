use crate::agents::subagent_task_config::DEFAULT_SUBAGENT_MAX_TURNS;
use crate::{
    agents::extension::ExtensionConfig,
    agents::{extension_manager::ExtensionManager, Agent, TaskConfig},
    config::ExtensionConfigManager,
    message::{Message, MessageContent, ToolRequest},
    prompt_template::render_global_file,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use mcp_core::handler::ToolError;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
// use serde_json::{self};
use crate::agents::agent::AgentEvent;
use async_stream::try_stream;
use futures::stream::BoxStream;
use futures::stream::StreamExt;
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
            conversation: Arc::new(Mutex::new(Vec::new())),
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

    #[instrument(skip(self, message))]
    pub async fn reply_subagent(
        &self,
        message: String,
        task_config: TaskConfig,
    ) -> Result<BoxStream<'_, Result<AgentEvent, anyhow::Error>>, anyhow::Error> {
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
        let messages = self.get_conversation().await;

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

        // Return streaming response exactly like main agent
        let max_turns = self.config.max_turns.unwrap_or(DEFAULT_SUBAGENT_MAX_TURNS);

        Ok(Box::pin(try_stream! {
            let mut turns_taken = 0;
            let mut messages = messages; // Make messages mutable in stream scope

            loop {
                turns_taken += 1;
                if turns_taken > max_turns {
                    self.set_status(SubAgentStatus::Completed("Max turns exceeded".to_string())).await;
                    yield AgentEvent::Message(Message::assistant().with_text(
                        "I've reached the maximum number of actions I can do without user input."
                    ));
                    break;
                }

                let mut stream = Agent::stream_response_from_provider(
                    Arc::clone(provider),
                    &system_prompt,
                    &messages,
                    &tools,
                    &toolshim_tools,
                ).await?;

                let mut added_message = false;
                let mut messages_to_add = Vec::new();

                while let Some(next) = stream.next().await {
                    match next {
                        Ok((response, _usage)) => {
                            if let Some(response) = response {

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

                                yield AgentEvent::Message(response.clone());

                                let num_tool_requests = tool_requests.len();

                                if num_tool_requests == 0 {
                                    continue;
                                }

                                let mut tool_response_message = Message::user();

                                for request in &tool_requests {
                                    if let Ok(tool_call) = &request.tool_call {
                                        let tool_result = match self
                                            .extension_manager
                                            .read()
                                            .await
                                            .dispatch_tool_call(tool_call.clone())
                                            .await
                                        {
                                            Ok(result) => result.result.await,
                                            Err(e) => Err(ToolError::ExecutionError(e.to_string())),
                                        };

                                        tool_response_message = tool_response_message.with_tool_response(
                                            request.id.clone(),
                                            tool_result,
                                        );
                                    }
                                }

                                yield AgentEvent::Message(tool_response_message.clone());
                                added_message = true;
                                messages_to_add.push(response);
                                messages_to_add.push(tool_response_message);
                                break;
                            }
                        }
                        Err(e) => {
                            self.set_status(SubAgentStatus::Completed(format!("Error: {}", e))).await;
                            error!("Error in stream: {}", e);
                            break;
                        }
                    }
                }

                if !added_message {
                    self.set_status(SubAgentStatus::Completed("No response generated".to_string())).await;
                    break;
                }

                // Add messages to conversation and internal state
                for msg in &messages_to_add {
                    self.add_message(msg.clone()).await;
                }
                messages.extend(messages_to_add);
            }
        }))
    }

    async fn add_message(&self, message: Message) {
        let mut conversation = self.conversation.lock().await;
        conversation.push(message);
    }

    async fn get_conversation(&self) -> Vec<Message> {
        self.conversation.lock().await.clone()
    }

    async fn build_system_prompt(&self, available_tools: &[Tool]) -> Result<String, anyhow::Error> {
        let mut context = HashMap::new();

        // Add basic context
        context.insert(
            "current_date_time",
            serde_json::Value::String(Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()),
        );
        context.insert("subagent_id", serde_json::Value::String(self.id.clone()));

        if let Some(max_turns) = self.config.max_turns {
            context.insert(
                "max_turns",
                serde_json::Value::Number(serde_json::Number::from(max_turns)),
            );
        }

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

        context.insert(
            "tool_count",
            serde_json::Value::Number(serde_json::Number::from(available_tools.len())),
        );

        let system_prompt = render_global_file("subagent_system.md", &context)
            .map_err(|e| anyhow!("Failed to render subagent system prompt: {}", e))?;

        Ok(system_prompt)
    }
}
