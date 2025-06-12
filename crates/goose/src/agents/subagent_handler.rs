use anyhow::Result;
use mcp_core::{Content, ToolError};
use serde_json::Value;
use std::sync::Arc;

use crate::agents::Agent;
use crate::agents::subagent_types::SpawnSubAgentArgs;


impl Agent {
    /// Handle spawning a new interactive subagent
    pub async fn handle_spawn_subagent(
        &self,
        arguments: Value,
    ) -> Result<Vec<Content>, ToolError> {
        let subagent_manager = self.subagent_manager.lock().await;
        let manager = subagent_manager.as_ref().ok_or_else(|| {
            ToolError::ExecutionError("Subagent manager not initialized".to_string())
        })?;

        // Parse arguments
        let recipe_name = arguments
            .get("recipe_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing recipe_name parameter".to_string()))?
            .to_string();

        let message = arguments
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing message parameter".to_string()))?
            .to_string();

        let mut args = SpawnSubAgentArgs::new(recipe_name, message);

        if let Some(max_turns) = arguments.get("max_turns").and_then(|v| v.as_u64()) {
            args = args.with_max_turns(max_turns as usize);
        }

        if let Some(timeout) = arguments.get("timeout_seconds").and_then(|v| v.as_u64()) {
            args = args.with_timeout(timeout);
        }

        // Get the provider from the parent agent
        let provider = self.provider().await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to get provider: {}", e))
        })?;

        // Get the extension manager from the parent agent
        let extension_manager = Arc::new(self.extension_manager.read().await);

        // Spawn the subagent
        match manager.spawn_interactive_subagent(args, provider, extension_manager).await {
            Ok(subagent_id) => {
                Ok(vec![Content::text(format!(
                    "Subagent spawned successfully with ID: {}\nUse platform__get_subagent_status to check progress.",
                    subagent_id
                ))])
            }
            Err(e) => Err(ToolError::ExecutionError(format!("Failed to spawn subagent: {}", e))),
        }
    }

    /// Handle listing all subagents
    pub async fn handle_list_subagents(&self) -> Result<Vec<Content>, ToolError> {
        let subagent_manager = self.subagent_manager.lock().await;
        let manager = subagent_manager.as_ref().ok_or_else(|| {
            ToolError::ExecutionError("Subagent manager not initialized".to_string())
        })?;

        let subagent_ids = manager.list_subagents().await;
        let status_map = manager.get_subagent_status().await;

        if subagent_ids.is_empty() {
            Ok(vec![Content::text("No active subagents.".to_string())])
        } else {
            let mut response = String::from("Active subagents:\n");
            for id in subagent_ids {
                let status = status_map
                    .get(&id)
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "Unknown".to_string());
                response.push_str(&format!("- {}: {}\n", id, status));
            }
            Ok(vec![Content::text(response)])
        }
    }

    /// Handle getting subagent status
    pub async fn handle_get_subagent_status(
        &self,
        arguments: Value,
    ) -> Result<Vec<Content>, ToolError> {
        let subagent_manager = self.subagent_manager.lock().await;
        let manager = subagent_manager.as_ref().ok_or_else(|| {
            ToolError::ExecutionError("Subagent manager not initialized".to_string())
        })?;

        let include_conversation = arguments
            .get("include_conversation")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if let Some(subagent_id) = arguments.get("subagent_id").and_then(|v| v.as_str()) {
            // Get status for specific subagent
            if let Some(subagent) = manager.get_subagent(subagent_id).await {
                let progress = subagent.get_progress().await;
                let mut response = format!(
                    "Subagent ID: {}\nStatus: {:?}\nMessage: {}\nTurn: {}",
                    progress.subagent_id, progress.status, progress.message, progress.turn
                );

                if let Some(max_turns) = progress.max_turns {
                    response.push_str(&format!("/{}", max_turns));
                }

                response.push_str(&format!("\nTimestamp: {}", progress.timestamp));

                if include_conversation {
                    response.push_str("\n\n");
                    response.push_str(&subagent.get_formatted_conversation().await);
                }

                Ok(vec![Content::text(response)])
            } else {
                Err(ToolError::ExecutionError(format!(
                    "Subagent {} not found",
                    subagent_id
                )))
            }
        } else {
            // Get status for all subagents
            let progress_map = manager.get_subagent_progress().await;

            if progress_map.is_empty() {
                Ok(vec![Content::text("No active subagents.".to_string())])
            } else {
                let mut response = String::from("All subagent status:\n\n");
                for (id, progress) in progress_map {
                    response.push_str(&format!(
                        "Subagent ID: {}\nStatus: {:?}\nMessage: {}\nTurn: {}",
                        id, progress.status, progress.message, progress.turn
                    ));

                    if let Some(max_turns) = progress.max_turns {
                        response.push_str(&format!("/{}", max_turns));
                    }

                    response.push_str(&format!("\nTimestamp: {}\n\n", progress.timestamp));
                }
                Ok(vec![Content::text(response)])
            }
        }
    }

    /// Handle sending a message to an existing subagent
    pub async fn handle_send_message_to_subagent(
        &self,
        arguments: Value,
    ) -> Result<Vec<Content>, ToolError> {
        let subagent_manager = self.subagent_manager.lock().await;
        let manager = subagent_manager.as_ref().ok_or_else(|| {
            ToolError::ExecutionError("Subagent manager not initialized".to_string())
        })?;

        // Parse arguments
        let subagent_id = arguments
            .get("subagent_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing subagent_id parameter".to_string()))?
            .to_string();

        let message = arguments
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing message parameter".to_string()))?
            .to_string();

        // Get the provider from the parent agent
        let provider = self.provider().await.map_err(|e| {
            ToolError::ExecutionError(format!("Failed to get provider: {}", e))
        })?;

        // Get the extension manager from the parent agent
        let extension_manager = Arc::new(self.extension_manager.read().await);

        // Send message to subagent and get response
        match manager.send_message_to_subagent(&subagent_id, message, provider, extension_manager).await {
            Ok(response) => {
                Ok(vec![Content::text(response)])
            }
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to send message to subagent: {}",
                e
            ))),
        }
    }
} 