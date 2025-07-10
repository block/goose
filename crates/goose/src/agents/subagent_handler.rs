use anyhow::Result;
use mcp_core::{Content, ToolError};
use serde_json::Value;
use std::sync::Arc;

use crate::agents::subagent_types::SpawnSubAgentArgs;
use crate::agents::Agent;
use crate::model::ModelConfig;
use crate::providers::create;

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

        // Determine which provider to use
        let provider = if let Some(recipe_name) = &args.recipe_name {
            // Load the recipe to check if it specifies a provider
            match manager.load_recipe(recipe_name).await {
                Ok(recipe) => {
                    if let Some(settings) = recipe.settings {
                        if let Some(recipe_provider_name) = settings.goose_provider {
                            // Recipe specifies a provider, create it
                            let model_name = if let Some(recipe_model) = settings.goose_model {
                                recipe_model
                            } else {
                                // Use the agent's provider's model if not specified in recipe
                                self.provider()
                                    .await
                                    .map_err(|e| {
                                        ToolError::ExecutionError(format!(
                                            "Failed to get provider: {}",
                                            e
                                        ))
                                    })?
                                    .get_model_config()
                                    .model_name
                            };
                            let model_config =
                                ModelConfig::new(model_name).with_temperature(settings.temperature);

                            match create(&recipe_provider_name, model_config) {
                                Ok(recipe_provider) => {
                                    tracing::info!(
                                        "Using recipe-specified provider: {}",
                                        recipe_provider_name
                                    );
                                    recipe_provider
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to create recipe provider '{}': {}. Falling back to agent provider.", recipe_provider_name, e);
                                    // Fall back to agent's provider
                                    self.provider().await.map_err(|e| {
                                        ToolError::ExecutionError(format!(
                                            "Failed to get provider: {}",
                                            e
                                        ))
                                    })?
                                }
                            }
                        } else {
                            // Recipe doesn't specify a provider, use agent's provider
                            self.provider().await.map_err(|e| {
                                ToolError::ExecutionError(format!("Failed to get provider: {}", e))
                            })?
                        }
                    } else {
                        // Recipe doesn't have settings, use agent's provider
                        self.provider().await.map_err(|e| {
                            ToolError::ExecutionError(format!("Failed to get provider: {}", e))
                        })?
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load recipe '{}': {}. Using agent provider.",
                        recipe_name,
                        e
                    );
                    // Fall back to agent's provider if recipe loading fails
                    self.provider().await.map_err(|e| {
                        ToolError::ExecutionError(format!("Failed to get provider: {}", e))
                    })?
                }
            }
        } else {
            // No recipe, use agent's provider
            self.provider()
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Failed to get provider: {}", e)))?
        };

        // Get the extension manager from the parent agent
        let extension_manager = Arc::clone(&self.extension_manager);

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
}
