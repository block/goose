use crate::agents::subagent::SubAgent;
use crate::agents::subagent_task_config::TaskConfig;
use anyhow::Result;

/// Standalone function to run a complete subagent task
pub async fn run_complete_subagent_task(
    text_instruction: String,
    task_config: TaskConfig,
) -> Result<String, anyhow::Error> {
    // Create the subagent with the parent agent's provider
    let (subagent, handle) = SubAgent::new(task_config.clone()).await?;

    // Execute the subagent task
    let result = subagent
        .reply_subagent_simple(text_instruction, task_config)
        .await?;
    let response_text = result.as_concat_text();

    // Clean up the subagent handle
    if let Err(e) = handle.await {
        tracing::debug!("Subagent handle cleanup error: {}", e);
    }

    // Return the result
    Ok(response_text)
}
