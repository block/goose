use anyhow::Result;
use mcp_core::{Content, ToolError};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agents::subagent_types::{
    SpawnSubAgentArgs, SubAgentUpdate, SubAgentUpdateType,
};
use crate::agents::Agent;

impl Agent {
    /// Handle running a complete subagent task (replaces the individual spawn/send/check tools)
    pub async fn handle_run_subagent_task(
        &self,
        arguments: Value,
    ) -> Result<Vec<Content>, ToolError> {
        let subagent_manager = self.subagent_manager.lock().await;
        let manager = subagent_manager.as_ref().ok_or_else(|| {
            ToolError::ExecutionError("Subagent manager not initialized".to_string())
        })?;

        // Parse arguments - using "task" as the main message parameter
        let message = arguments
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing task parameter".to_string()))?
            .to_string();

        // Either recipe_name or instructions must be provided
        let recipe_name = arguments
            .get("recipe_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let instructions = arguments
            .get("instructions")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut args = if let Some(recipe_name) = recipe_name {
            SpawnSubAgentArgs::new_with_recipe(recipe_name, message.clone())
        } else if let Some(instructions) = instructions {
            SpawnSubAgentArgs::new_with_instructions(instructions, message.clone())
        } else {
            return Err(ToolError::ExecutionError(
                "Either recipe_name or instructions parameter must be provided".to_string(),
            ));
        };

        // Set max_turns with default of 10
        let max_turns = arguments
            .get("max_turns")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        args = args.with_max_turns(max_turns);

        if let Some(timeout) = arguments.get("timeout_seconds").and_then(|v| v.as_u64()) {
            args = args.with_timeout(timeout);
        }

        // Get the provider from the parent agent
        let provider = self
            .provider()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to get provider: {}", e)))?;

        // Get the extension manager from the parent agent
        let extension_manager = Arc::new(self.extension_manager.read().await);

        // Run the complete subagent task
        match manager
            .run_complete_subagent_task(args, provider, extension_manager)
            .await
        {
            Ok(result) => Ok(vec![Content::text(result)]),
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to run subagent task: {}",
                e
            ))),
        }
    }

    /// Get updates from subagents (not visible to user)
    pub async fn get_subagent_updates(&self) -> Vec<SubAgentUpdate> {
        let mut subagent_manager = self.subagent_manager.lock().await;
        if let Some(manager) = subagent_manager.as_mut() {
            manager.process_updates().await
        } else {
            Vec::new()
        }
    }

    /// Process subagent updates and build a context map
    pub async fn process_subagent_updates(&self) -> HashMap<String, String> {
        let mut result_map = HashMap::new();
        let updates = self.get_subagent_updates().await;

        for update in updates {
            // Only store results and completions in the context
            if update.update_type == SubAgentUpdateType::Result
                || update.update_type == SubAgentUpdateType::Completion
            {
                let key = format!("subagent_{}", update.subagent_id);
                let value = if let Some(conversation) = update.conversation {
                    format!("{}\n\nFull conversation:\n{}", update.content, conversation)
                } else {
                    update.content
                };
                result_map.insert(key, value);
            }
        }

        result_map
    }
}
