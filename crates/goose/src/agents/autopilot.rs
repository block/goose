use anyhow::Result;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, info};

use crate::config::Config;
use crate::conversation::Conversation;
use crate::providers;

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub role: String,
}

/// AutoPilot manages automatic model switching based on conversation context
pub struct AutoPilot {
    oracle_config: Option<ModelConfig>,
    second_opinion_config: Option<ModelConfig>,
    original_provider: Option<Arc<dyn crate::providers::base::Provider>>,
    switch_active: bool,
}

impl AutoPilot {
    /// Create a new AutoPilot instance, loading model configurations from config
    pub fn new() -> Self {
        let config = Config::global();

        // Try to load models configuration from config.yaml
        let models: Vec<ModelConfig> = config.get_param("models").unwrap_or_else(|_| Vec::new());

        let oracle_config = models.iter().find(|m| m.role == "oracle").cloned();

        let second_opinion_config = models.iter().find(|m| m.role == "second-opinion").cloned();

        if oracle_config.is_some() || second_opinion_config.is_some() {
            info!("AutoPilot initialized with model configurations");
            if let Some(ref oracle) = oracle_config {
                debug!("Oracle model: {}/{}", oracle.provider, oracle.model);
            }
            if let Some(ref second) = second_opinion_config {
                debug!("Second-opinion model: {}/{}", second.provider, second.model);
            }
        } else {
            debug!("AutoPilot: No model configurations found in config");
        }

        Self {
            oracle_config,
            second_opinion_config,
            original_provider: None,
            switch_active: false,
        }
    }

    /// Check if a model switch should occur based on the conversation
    /// Returns Some((provider, model)) if a switch should happen, None otherwise
    pub async fn check_for_switch(
        &mut self,
        conversation: &Conversation,
        current_provider: Arc<dyn crate::providers::base::Provider>,
    ) -> Result<Option<Arc<dyn crate::providers::base::Provider>>> {
        // If we already switched, check if we should switch back
        if self.switch_active {
            debug!("AutoPilot: Switching back to original provider");
            self.switch_active = false;
            if let Some(original) = self.original_provider.take() {
                return Ok(Some(original));
            }
            return Ok(None);
        }

        // Get the last user message
        let last_user_message = conversation
            .messages()
            .iter()
            .rev()
            .find(|msg| msg.role == rmcp::model::Role::User)
            .and_then(|msg| msg.content.first())
            .and_then(|content| content.as_text());

        if let Some(text) = last_user_message {
            let text_lower = text.to_lowercase();

            // Check for "think" -> oracle
            if text_lower.contains("think") && self.oracle_config.is_some() {
                let oracle = self.oracle_config.as_ref().unwrap().clone();
                info!("AutoPilot: Detected 'think' - switching to oracle model");
                return self
                    .create_and_switch_provider(&oracle, current_provider)
                    .await;
            }
            
            // Check for "help" -> second-opinion
            if text_lower.contains("help") && self.second_opinion_config.is_some() {
                let second_opinion = self.second_opinion_config.as_ref().unwrap().clone();
                info!("AutoPilot: Detected 'help' - switching to second-opinion model");
                return self
                    .create_and_switch_provider(&second_opinion, current_provider)
                    .await;
            }
        }

        Ok(None)
    }

    /// Create a new provider and prepare for switching
    async fn create_and_switch_provider(
        &mut self,
        model_config: &ModelConfig,
        current_provider: Arc<dyn crate::providers::base::Provider>,
    ) -> Result<Option<Arc<dyn crate::providers::base::Provider>>> {
        // Store the original provider so we can switch back
        self.original_provider = Some(current_provider);
        self.switch_active = true;

        // Create the new provider
        let model = crate::model::ModelConfig::new_or_fail(&model_config.model);
        let new_provider = providers::create(&model_config.provider, model)?;

        debug!(
            "AutoPilot: Created provider {} with model {}",
            model_config.provider, model_config.model
        );

        Ok(Some(new_provider))
    }

    /// Check if autopilot is currently in a switched state
    #[allow(dead_code)]
    pub fn is_switched(&self) -> bool {
        self.switch_active
    }
}