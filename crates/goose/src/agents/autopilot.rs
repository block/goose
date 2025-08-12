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

    /// Check if the conversation indicates we should switch to oracle model
    /// Has access to full conversation context and self state
    fn should_switch_to_oracle(&self, conversation: &Conversation) -> bool {
        // Can access self state (e.g., self.switch_active, self.oracle_config, etc.)
        if self.oracle_config.is_none() {
            return false;
        }

        // Get the last user message from the conversation
        let last_user_message = conversation
            .messages()
            .iter()
            .rev()
            .find(|msg| msg.role == rmcp::model::Role::User)
            .and_then(|msg| msg.content.first())
            .and_then(|content| content.as_text());

        // Check for "think" trigger word
        if let Some(text) = last_user_message {
            return text.to_lowercase().contains("think");
        }

        false
    }

    /// Check if the conversation indicates we should switch to second-opinion model
    /// Has access to full conversation context and self state
    fn should_switch_to_second_opinion(&self, conversation: &Conversation) -> bool {
        // Can access self state (e.g., self.switch_active, self.second_opinion_config, etc.)
        if self.second_opinion_config.is_none() {
            return false;
        }

        // Get the last user message from the conversation
        let last_user_message = conversation
            .messages()
            .iter()
            .rev()
            .find(|msg| msg.role == rmcp::model::Role::User)
            .and_then(|msg| msg.content.first())
            .and_then(|content| content.as_text());

        // Check for "help" trigger word
        if let Some(text) = last_user_message {
            return text.to_lowercase().contains("help");
        }

        false
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

        // Check for "think" -> oracle
        if self.should_switch_to_oracle(conversation) {
            let oracle = self.oracle_config.as_ref().unwrap().clone();
            info!("AutoPilot: Detected 'think' - switching to oracle model");
            return self
                .create_and_switch_provider(&oracle, current_provider)
                .await;
        }

        // Check for "help" -> second-opinion
        if self.should_switch_to_second_opinion(conversation) {
            let second_opinion = self.second_opinion_config.as_ref().unwrap().clone();
            info!("AutoPilot: Detected 'help' - switching to second-opinion model");
            return self
                .create_and_switch_provider(&second_opinion, current_provider)
                .await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use crate::conversation::Conversation;

    fn create_test_autopilot() -> AutoPilot {
        AutoPilot {
            original_provider: None,
            oracle_config: Some(ModelConfig {
                provider: "openai".to_string(),
                model: "o1-preview".to_string(),
                role: "oracle".to_string(),
            }),
            second_opinion_config: Some(ModelConfig {
                provider: "anthropic".to_string(),
                model: "claude-3-5-sonnet-20241022".to_string(),
                role: "second-opinion".to_string(),
            }),
            switch_active: false,
        }
    }

    fn create_conversation_with_text(text: &str) -> Conversation {
        let message = Message::user().with_text(text);
        Conversation::new(vec![message]).expect("Failed to create conversation")
    }

    #[test]
    fn test_should_switch_to_oracle_with_trigger() {
        let autopilot = create_test_autopilot();
        let conversation = create_conversation_with_text("I need to think about this problem");
        
        assert!(autopilot.should_switch_to_oracle(&conversation));
    }

    #[test]
    fn test_should_switch_to_oracle_without_trigger() {
        let autopilot = create_test_autopilot();
        let conversation = create_conversation_with_text("Just a normal message");
        
        assert!(!autopilot.should_switch_to_oracle(&conversation));
    }

    #[test]
    fn test_should_switch_to_second_opinion_with_trigger() {
        let autopilot = create_test_autopilot();
        let conversation = create_conversation_with_text("I need help with this");
        
        assert!(autopilot.should_switch_to_second_opinion(&conversation));
    }

    #[test]
    fn test_should_switch_to_second_opinion_without_trigger() {
        let autopilot = create_test_autopilot();
        let conversation = create_conversation_with_text("Just a normal message");
        
        assert!(!autopilot.should_switch_to_second_opinion(&conversation));
    }

    #[test]
    fn test_no_switch_when_config_missing() {
        let mut autopilot = create_test_autopilot();
        autopilot.oracle_config = None;
        autopilot.second_opinion_config = None;
        
        let conversation = create_conversation_with_text("I need to think and need help");
        
        assert!(!autopilot.should_switch_to_oracle(&conversation));
        assert!(!autopilot.should_switch_to_second_opinion(&conversation));
    }

    #[test]
    fn test_is_switched_state() {
        let mut autopilot = create_test_autopilot();
        assert!(!autopilot.is_switched());
        
        autopilot.switch_active = true;
        assert!(autopilot.is_switched());
    }

    #[test]
    fn test_empty_conversation() {
        let autopilot = create_test_autopilot();
        let conversation = Conversation::empty();
        
        // Should not switch on empty conversation
        assert!(!autopilot.should_switch_to_oracle(&conversation));
        assert!(!autopilot.should_switch_to_second_opinion(&conversation));
    }

    #[test]
    fn test_case_insensitive_triggers() {
        let autopilot = create_test_autopilot();
        
        // Test uppercase trigger words
        let conversation = create_conversation_with_text("I need to THINK about this");
        assert!(autopilot.should_switch_to_oracle(&conversation));
        
        let conversation = create_conversation_with_text("I need HELP with this");
        assert!(autopilot.should_switch_to_second_opinion(&conversation));
    }
}