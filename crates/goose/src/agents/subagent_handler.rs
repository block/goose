use crate::agents::subagent::SubAgent;
use crate::agents::subagent_task_config::TaskConfig;
use anyhow::Result;
use mcp_core::ToolError;
use rmcp::model::Content;
use serde_json::Value;

/// Standalone function to run a complete subagent task
pub async fn run_complete_subagent_task(
    task_arguments: Value,
    task_config: TaskConfig,
) -> Result<Vec<Content>, ToolError> {
    // Parse arguments - using "task" as the main message parameter
    let text_instruction = task_arguments
        .get("text_instruction")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::ExecutionError("Missing text_instruction parameter".to_string()))?
        .to_string();

    // Create the subagent with the parent agent's provider
    let (subagent, handle) = SubAgent::new(task_config.clone())
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Failed to create subagent: {}", e)))?;

    // Execute the subagent task
    let result = match subagent.reply_subagent(text_instruction, task_config).await {
        Ok(response) => {
            let response_text = response.as_concat_text();
            Ok(vec![Content::text(response_text)])
        }
        Err(e) => Err(ToolError::ExecutionError(format!(
            "Subagent execution failed: {}",
            e
        ))),
    };

    // Clean up the subagent handle
    if let Err(e) = handle.await {
        tracing::debug!("Subagent handle cleanup error: {}", e);
    }

    // Return the result
    result
}
