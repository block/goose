use anyhow::Result;
use mcp_core::{Content, ToolError};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agents::subagent_types::{
    SpawnSubAgentArgs, SubAgentNotification, SubAgentUpdate, SubAgentUpdateType,
};
use crate::agents::Agent;

impl Agent {
    /// Handle spawning a new interactive subagent
    /// Handle spawning a new interactive subagent
    pub async fn handle_spawn_subagent(&self, arguments: Value) -> Result<Vec<Content>, ToolError> {
        let subagent_manager = self.subagent_manager.lock().await;
        let manager = subagent_manager.as_ref().ok_or_else(|| {
            ToolError::ExecutionError("Subagent manager not initialized".to_string())
        })?;

        // Parse arguments
        let message = arguments
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("Missing message parameter".to_string()))?
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

        if let Some(max_turns) = arguments.get("max_turns").and_then(|v| v.as_u64()) {
            args = args.with_max_turns(max_turns as usize);
        }

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

        // Spawn the subagent (without processing initial message)
        match manager
            .spawn_interactive_subagent(args, provider.clone(), extension_manager.clone())
            .await
        {
            Ok(subagent_id) => {
                // Send the initial message separately
                match manager
                    .send_message_to_subagent(&subagent_id, message, provider, extension_manager)
                    .await
                {
                    Ok(_response) => {
                        Ok(vec![Content::text(format!(
                            "Subagent spawned successfully with ID: {}\nInitial message sent. Use subagent__check_progress to check status.",
                            subagent_id
                        ))])
                    }
                    Err(e) => {
                        // Clean up the subagent if initial message fails
                        let _ = manager.terminate_subagent(&subagent_id).await;
                        Err(ToolError::ExecutionError(format!(
                            "Failed to send initial message to subagent: {}",
                            e
                        )))
                    }
                }
            }
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to spawn subagent: {}",
                e
            ))),
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
        let provider = self
            .provider()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Failed to get provider: {}", e)))?;

        // Get the extension manager from the parent agent
        let extension_manager = Arc::new(self.extension_manager.read().await);

        // Send message to subagent and get response
        match manager
            .send_message_to_subagent(&subagent_id, message, provider, extension_manager)
            .await
        {
            Ok(response) => Ok(vec![Content::text(response)]),
            Err(e) => Err(ToolError::ExecutionError(format!(
                "Failed to send message to subagent: {}",
                e
            ))),
        }
    }

    /// Get notifications from subagents
    pub async fn get_subagent_notifications(&self) -> Vec<SubAgentNotification> {
        let mut subagent_manager = self.subagent_manager.lock().await;
        if let Some(manager) = subagent_manager.as_mut() {
            manager.process_notifications().await
        } else {
            Vec::new()
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
