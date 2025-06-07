use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, instrument, warn};

use crate::agents::extension_manager::ExtensionManager;
use crate::agents::subagent::{SubAgent, SubAgentConfig, SubAgentProgress, SubAgentStatus};
use crate::agents::subagent_types::SpawnSubAgentArgs;
use crate::providers::base::Provider;
use crate::recipe::Recipe;

/// Manages the lifecycle of subagents
pub struct SubAgentManager {
    subagents: Arc<RwLock<HashMap<String, Arc<SubAgent>>>>,
    handles: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl SubAgentManager {
    /// Create a new subagent manager
    pub fn new() -> Self {
        Self {
            subagents: Arc::new(RwLock::new(HashMap::new())),
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn a new interactive subagent
    #[instrument(skip(self, args, provider, extension_manager))]
    pub async fn spawn_interactive_subagent(
        &self,
        args: SpawnSubAgentArgs,
        provider: Arc<dyn Provider>,
        extension_manager: Arc<tokio::sync::RwLockReadGuard<'_, ExtensionManager>>,
    ) -> Result<String> {
        debug!(
            "Spawning interactive subagent with recipe: {}",
            args.recipe_name
        );

        // Load the recipe
        let recipe = self.load_recipe(&args.recipe_name).await?;

        // Create subagent config
        let mut config = SubAgentConfig::new(recipe);
        if let Some(max_turns) = args.max_turns {
            config = config.with_max_turns(max_turns);
        }
        if let Some(timeout) = args.timeout_seconds {
            config = config.with_timeout(timeout);
        }

        // Create the subagent with the parent agent's provider
        let (subagent, handle) = SubAgent::new(config, Arc::clone(&provider), Arc::clone(&extension_manager)).await?;
        let subagent_id = subagent.id.clone();

        // Store the subagent and its handle
        {
            let mut subagents = self.subagents.write().await;
            subagents.insert(subagent_id.clone(), Arc::clone(&subagent));
        }
        {
            let mut handles = self.handles.lock().await;
            handles.insert(subagent_id.clone(), handle);
        }

        // Process the initial message
        match subagent.reply_subagent(args.message, provider, extension_manager).await {
            Ok(_stream) => {
                // For now, we don't handle the stream here - that would be done by the caller
                debug!(
                    "Subagent {} spawned and initial message processed",
                    subagent_id
                );
            }
            Err(e) => {
                error!(
                    "Failed to process initial message for subagent {}: {}",
                    subagent_id, e
                );
                // Clean up the failed subagent
                self.terminate_subagent(&subagent_id).await?;
                return Err(e);
            }
        }

        Ok(subagent_id)
    }

    /// Get a subagent by ID
    pub async fn get_subagent(&self, id: &str) -> Option<Arc<SubAgent>> {
        let subagents = self.subagents.read().await;
        subagents.get(id).cloned()
    }

    /// List all active subagent IDs
    pub async fn list_subagents(&self) -> Vec<String> {
        let subagents = self.subagents.read().await;
        subagents.keys().cloned().collect()
    }

    /// Get status of all subagents
    pub async fn get_subagent_status(&self) -> HashMap<String, SubAgentStatus> {
        let subagents = self.subagents.read().await;
        let mut status_map = HashMap::new();

        for (id, subagent) in subagents.iter() {
            status_map.insert(id.clone(), subagent.get_status().await);
        }

        status_map
    }

    /// Get progress of all subagents
    pub async fn get_subagent_progress(&self) -> HashMap<String, SubAgentProgress> {
        let subagents = self.subagents.read().await;
        let mut progress_map = HashMap::new();

        for (id, subagent) in subagents.iter() {
            progress_map.insert(id.clone(), subagent.get_progress().await);
        }

        progress_map
    }

    /// Send a message to a specific subagent
    #[instrument(skip(self, message, provider, extension_manager))]
    pub async fn send_message_to_subagent(
        &self,
        subagent_id: &str,
        message: String,
        provider: Arc<dyn Provider>,
        extension_manager: Arc<tokio::sync::RwLockReadGuard<'_, ExtensionManager>>,
    ) -> Result<String> {
        let subagent = self
            .get_subagent(subagent_id)
            .await
            .ok_or_else(|| anyhow!("Subagent {} not found", subagent_id))?;

        // Process the message and get a reply
        match subagent.reply_subagent(message, provider, extension_manager).await {
            Ok(response) => {
                Ok(format!("Message sent to subagent {}. Response:\n{}", subagent_id, response.as_concat_text()))
            }
            Err(e) => Err(anyhow!("Failed to process message in subagent: {}", e))
        }
    }

    /// Terminate a specific subagent
    #[instrument(skip(self))]
    pub async fn terminate_subagent(&self, id: &str) -> Result<()> {
        debug!("Terminating subagent {}", id);

        // Get and terminate the subagent
        let subagent = {
            let mut subagents = self.subagents.write().await;
            subagents.remove(id)
        };

        if let Some(subagent) = subagent {
            subagent.terminate().await?;
        } else {
            warn!("Attempted to terminate non-existent subagent {}", id);
            return Err(anyhow!("Subagent {} not found", id));
        }

        // Clean up the background handle
        let handle = {
            let mut handles = self.handles.lock().await;
            handles.remove(id)
        };

        if let Some(handle) = handle {
            handle.abort();
        }

        debug!("Subagent {} terminated successfully", id);
        Ok(())
    }

    /// Terminate all subagents
    #[instrument(skip(self))]
    pub async fn terminate_all_subagents(&self) -> Result<()> {
        debug!("Terminating all subagents");

        let subagent_ids: Vec<String> = {
            let subagents = self.subagents.read().await;
            subagents.keys().cloned().collect()
        };

        for id in subagent_ids {
            if let Err(e) = self.terminate_subagent(&id).await {
                error!("Failed to terminate subagent {}: {}", id, e);
            }
        }

        debug!("All subagents terminated");
        Ok(())
    }

    /// Get formatted conversation from a subagent
    pub async fn get_subagent_conversation(&self, id: &str) -> Result<String> {
        let subagent = self
            .get_subagent(id)
            .await
            .ok_or_else(|| anyhow!("Subagent {} not found", id))?;

        Ok(subagent.get_formatted_conversation().await)
    }

    /// Clean up completed or failed subagents
    pub async fn cleanup_completed_subagents(&self) -> Result<usize> {
        let mut completed_ids = Vec::new();

        // Find completed subagents
        {
            let subagents = self.subagents.read().await;
            for (id, subagent) in subagents.iter() {
                if subagent.is_completed().await {
                    completed_ids.push(id.clone());
                }
            }
        }

        // Remove completed subagents
        let count = completed_ids.len();
        for id in completed_ids {
            if let Err(e) = self.terminate_subagent(&id).await {
                error!("Failed to cleanup completed subagent {}: {}", id, e);
            }
        }

        debug!("Cleaned up {} completed subagents", count);
        Ok(count)
    }

    /// Load a recipe from file
    async fn load_recipe(&self, recipe_name: &str) -> Result<Recipe> {
        // Try to load from current directory first
        let recipe_path = if recipe_name.ends_with(".yaml") || recipe_name.ends_with(".yml") {
            recipe_name.to_string()
        } else {
            format!("{}.yaml", recipe_name)
        };

        if Path::new(&recipe_path).exists() {
            let content = tokio::fs::read_to_string(&recipe_path).await?;
            let recipe: Recipe = serde_yaml::from_str(&content)?;
            return Ok(recipe);
        }

        // Try some common recipe locations
        let common_paths = [
            format!("recipes/{}", recipe_path),
            format!("./recipes/{}", recipe_path),
            format!("../recipes/{}", recipe_path),
        ];

        for path in &common_paths {
            if Path::new(path).exists() {
                let content = tokio::fs::read_to_string(path).await?;
                let recipe: Recipe = serde_yaml::from_str(&content)?;
                return Ok(recipe);
            }
        }

        Err(anyhow!(
            "Recipe file '{}' not found in current directory or common recipe locations",
            recipe_name
        ))
    }

    /// Get count of active subagents
    pub async fn get_active_count(&self) -> usize {
        let subagents = self.subagents.read().await;
        subagents.len()
    }

    /// Check if a subagent exists
    pub async fn has_subagent(&self, id: &str) -> bool {
        let subagents = self.subagents.read().await;
        subagents.contains_key(id)
    }
}

impl Drop for SubAgentManager {
    fn drop(&mut self) {
        // Note: In a real implementation, you might want to spawn a task to clean up
        // subagents gracefully, but for now we'll rely on the Drop implementations
        // of the individual components
        debug!("SubAgentManager dropped");
    }
}
