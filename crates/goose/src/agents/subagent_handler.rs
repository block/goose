use crate::agents::subagent::SubAgent;
use crate::agents::subagent_task_config::TaskConfig;
use anyhow::Result;
use rmcp::model::{ErrorCode, ErrorData};

/// Standalone function to run a complete subagent task
pub async fn run_complete_subagent_task(
    text_instruction: String,
    task_config: TaskConfig,
) -> Result<String, anyhow::Error> {
    run_complete_subagent_task_with_options(text_instruction, task_config, false).await
}

/// Standalone function to run a complete subagent task with output options
pub async fn run_complete_subagent_task_with_options(
    text_instruction: String,
    task_config: TaskConfig,
    return_last_only: bool,
) -> Result<String, anyhow::Error> {
    // Create the subagent with the parent agent's provider
    let subagent = SubAgent::new(task_config.clone()).await.map_err(|e| {
        ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            format!("Failed to create subagent: {}", e),
            None,
        )
    })?;

    // Execute the subagent task
    let messages = subagent
        .reply_subagent(text_instruction, task_config)
        .await?;

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
