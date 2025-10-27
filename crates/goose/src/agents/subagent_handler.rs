use crate::{
    agents::{
        extension::PlatformExtensionContext, subagent_task_config::TaskConfig, Agent, AgentEvent,
        SessionConfig,
    },
    conversation::{message::Message, Conversation},
    execution::manager::AgentManager,
    session::SessionManager,
};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use rmcp::model::{ErrorCode, ErrorData};
use std::pin::Pin;
use std::{future::Future, sync::Arc};
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
) -> Pin<Box<dyn Future<Output = Result<Conversation>> + Send>> {
    Box::pin(async move {
        let agent_manager = AgentManager::instance()
            .await
            .map_err(|e| anyhow!("Failed to create AgentManager: {}", e))?;
        let parent_session_id = task_config.parent_session_id;
        let working_dir = task_config.parent_working_dir;
        let (agent, session_id) = match parent_session_id {
            Some(parent_session_id) => {
                let session = SessionManager::create_session(
                    working_dir.clone(),
                    format!("Subagent task for: {}", parent_session_id),
                )
                .await
                .map_err(|e| anyhow!("Failed to create a session for sub agent: {}", e))?;

                let agent = agent_manager
                    .get_or_create_agent(session.id.clone())
                    .await
                    .map_err(|e| anyhow!("Failed to get sub agent session file path: {}", e))?;
                (agent, Some(session.id))
            }
            None => {
                let agent = Arc::new(Agent::new());
                agent
                    .extension_manager
                    .set_context(PlatformExtensionContext {
                        session_id: None,
                        extension_manager: Some(Arc::downgrade(&agent.extension_manager)),
                        tool_route_manager: Some(Arc::downgrade(&agent.tool_route_manager)),
                    })
                    .await;
                (agent, None)
            }
        };

        agent
            .update_provider(task_config.provider)
            .await
            .map_err(|e| anyhow!("Failed to set provider on sub agent: {}", e))?;

        for extension in task_config.extensions {
            if let Err(e) = agent.add_extension(extension.clone()).await {
                debug!(
                    "Failed to add extension '{}' to subagent: {}",
                    extension.name(),
                    e
                );
            }
        }

        let mut conversation =
            Conversation::new_unvalidated(
                vec![Message::user().with_text(text_instruction.clone())],
            );
        let session_config = if let Some(session_id) = session_id {
            Some(SessionConfig {
                id: session_id,
                working_dir,
                schedule_id: None,
                execution_mode: None,
                max_turns: task_config.max_turns.map(|v| v as u32),
                retry_config: None,
            })
        } else {
            None
        };
        let mut stream = agent
            .reply(conversation.clone(), session_config, None)
            .await
            .map_err(|e| anyhow!("Failed to get reply from agent: {}", e))?;
        while let Some(message_result) = stream.next().await {
            match message_result {
                Ok(AgentEvent::Message(msg)) => conversation.push(msg),
                Ok(AgentEvent::McpNotification(_)) | Ok(AgentEvent::ModelChange { .. }) => {}
                Ok(AgentEvent::HistoryReplaced(updated_conversation)) => {
                    conversation = updated_conversation;
                }
                Err(e) => {
                    tracing::error!("Error receiving message from subagent: {}", e);
                    break;
                }
            }
        }

        Ok(conversation)
    })
}
