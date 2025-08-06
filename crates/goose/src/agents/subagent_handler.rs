use crate::agents::subagent::SubAgent;
use crate::agents::subagent_task_config::TaskConfig;
use anyhow::Result;
use mcp_core::ToolError;

/// Standalone function to run a complete subagent task
pub async fn run_complete_subagent_task(
    text_instruction: String,
    task_config: TaskConfig,
) -> Result<String, anyhow::Error> {
    // Create the subagent with the parent agent's provider
    let subagent = SubAgent::new(task_config.clone())
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Failed to create subagent: {}", e)))?;

    // Execute the subagent task
    let messages = subagent
        .reply_subagent(text_instruction, task_config)
        .await?;

    // Extract all text content from all messages
    let all_text_content: Vec<String> = messages
        .iter()
        .flat_map(|message| {
            message.content.iter().filter_map(|content| {
                match content {
                    crate::conversation::message::MessageContent::Text(text_content) => {
                        Some(text_content.text.clone())
                    }
                    crate::conversation::message::MessageContent::ToolResponse(tool_response) => {
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

    let response_text = all_text_content.join("\n");

    // Return the result
    Ok(response_text)
}
