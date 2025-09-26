use std::sync::Arc;

use crate::{
    agents::{subagent_task_config::TaskConfig, AgentEvent, SessionConfig},
    conversation::{message::Message, Conversation},
    execution::{manager::AgentManager, SessionExecutionMode},
    providers::base::Provider,
    session,
};
use anyhow::{anyhow, Result};
use futures::future::BoxFuture;
use futures::StreamExt;
use rmcp::model::{ErrorCode, ErrorData};
use tracing::debug;

/// Standalone function to run a complete subagent task with output options
pub async fn run_complete_subagent_task(
    text_instruction: String,
    task_config: TaskConfig,
    return_last_only: bool,
) -> Result<String, anyhow::Error> {
    let messages = get_agent_messages(text_instruction, task_config)
        .await
        .map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to execute task: {}", e),
                None,
            )
        })?;

    // Extract text content based on return_last_only flag
    let response_text = if return_last_only {
        // Get only the last message's text content
        messages
            .messages()
            .last()
            .and_then(|message| {
                message.content.iter().find_map(|content| match content {
                    crate::conversation::message::MessageContent::Text(text_content) => {
                        Some(text_content.text.clone())
                    }
                    _ => None,
                })
            })
            .unwrap_or_else(|| String::from("No text content in last message"))
    } else {
        // Extract all text content from all messages (original behavior)
        let all_text_content: Vec<String> = messages
            .iter()
            .flat_map(|message| {
                message.content.iter().filter_map(|content| {
                    match content {
                        crate::conversation::message::MessageContent::Text(text_content) => {
                            Some(text_content.text.clone())
                        }
                        crate::conversation::message::MessageContent::ToolResponse(
                            tool_response,
                        ) => {
                            // Extract text from tool response
                            if let Ok(contents) = &tool_response.tool_result {
                                let texts: Vec<String> = contents
                                    .iter()
                                    .filter_map(|content| {
                                        if let rmcp::model::RawContent::Text(raw_text_content) =
                                            &content.raw
                                        {
                                            Some(raw_text_content.text.clone())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                if !texts.is_empty() {
                                    Some(format!("Tool result: {}", texts.join("\n")))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                })
            })
            .collect();

        all_text_content.join("\n")
    };

    // Return the result
    Ok(response_text)
}

fn get_agent_messages(
    text_instruction: String,
    task_config: TaskConfig,
) -> BoxFuture<'static, Result<Conversation>> {
    Box::pin(async move {
        let agent_manager = AgentManager::instance(None)
            .await
            .map_err(|e| anyhow!("Failed to create AgentManager: {}", e))?;
        let parent_session_id = task_config
            .parent_session_id
            .ok_or_else(|| anyhow!("Parent session ID is missing"))?;
        let session_id = session::generate_session_id();
        let agent = agent_manager
            .get_or_create_agent(
                session_id.clone(),
                SessionExecutionMode::SubTask {
                    parent_session: parent_session_id,
                },
            )
            .await
            .map_err(|e| anyhow!("Failed to get sub agent session file path: {}", e))?;
        let agent_provider: Arc<dyn Provider> = task_config
            .provider
            .ok_or_else(|| anyhow!("No provider configured for subagent"))?;
        agent
            .update_provider(agent_provider)
            .await
            .map_err(|e| anyhow!("Failed to set provider on sub agent: {}", e))?;

        if let Some(recipe_extensions) = task_config.extensions {
            for extension in recipe_extensions {
                if let Err(e) = agent.add_extension(extension.clone()).await {
                    debug!(
                        "Failed to add extension '{}' to subagent: {}",
                        extension.name(),
                        e
                    );
                }
            }
        }

        let session_file_path = crate::session::storage::get_path(
            crate::session::storage::Identifier::Name(session_id.clone()),
        )
        .map_err(|e| anyhow!("Failed to get sub agent session file path: {}", e))?;
        let mut session_messages =
            Conversation::new_unvalidated(
                vec![Message::user().with_text(text_instruction.clone())],
            );
        let current_dir = std::env::current_dir()
            .map_err(|e| anyhow!("Failed to get current directory for sub agent: {}", e))?;
        let session_config = SessionConfig {
            id: crate::session::storage::Identifier::Name(session_id.clone()),
            working_dir: current_dir,
            schedule_id: None,
            execution_mode: None,
            max_turns: task_config.max_turns.map(|v| v as u32),
            retry_config: None,
        };

        let mut stream = agent
            .reply(session_messages.clone(), Some(session_config), None)
            .await
            .map_err(|e| anyhow!("Failed to get reply from agent: {}", e))?;
        while let Some(message_result) = stream.next().await {
            match message_result {
                Ok(AgentEvent::Message(msg)) => session_messages.push(msg),
                Ok(AgentEvent::McpNotification(_))
                | Ok(AgentEvent::ModelChange { .. })
                | Ok(AgentEvent::HistoryReplaced(_)) => {} // Handle informational events
                Err(e) => {
                    tracing::error!("Error receiving message from subagent: {}", e);
                    break;
                }
            }
        }

        if let Ok(mut updated_metadata) = crate::session::storage::read_metadata(&session_file_path)
        {
            updated_metadata.message_count = session_messages.len();
            if let Err(e) = crate::session::storage::save_messages_with_metadata(
                &session_file_path,
                &updated_metadata,
                &session_messages,
            ) {
                tracing::error!("Failed to persist final messages: {}", e);
                return Err(anyhow!("Failed to save messages: {}", e));
            }
        } else {
            tracing::error!("Failed to read updated metadata before final save");
        }

        Ok(session_messages)
    })
}
