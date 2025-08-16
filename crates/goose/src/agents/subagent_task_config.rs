use crate::config::Config;
use crate::model::ModelConfig;
use crate::providers::base::Provider;
use crate::providers::create;
use std::env;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

/// Default maximum number of turns for task execution
pub const DEFAULT_SUBAGENT_MAX_TURNS: usize = 10;

/// config variable name for max subagent turns
pub const GOOSE_SUBAGENT_MAX_TURNS: &str = "GOOSE_SUBAGENT_MAX_TURNS";

/// config variable name for subagent provider
pub const GOOSE_SUBAGENT_PROVIDER: &str = "GOOSE_SUBAGENT_PROVIDER";

/// config variable name for subagent model
pub const GOOSE_SUBAGENT_MODEL: &str = "GOOSE_SUBAGENT_MODEL";

/// Configuration for task execution with all necessary dependencies
#[derive(Clone)]
pub struct TaskConfig {
    pub id: String,
    pub provider: Option<Arc<dyn Provider>>,
    pub provider_name: Option<String>,
    pub max_turns: Option<usize>,
}

impl fmt::Debug for TaskConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskConfig")
            .field("id", &self.id)
            .field("provider", &"<provider>")
            .field("max_turns", &self.max_turns)
            .finish()
    }
}

impl TaskConfig {
    /// Get a configuration value with environment variable precedence
    /// First checks environment variable, then falls back to config
    pub fn get_var(var_name: &str) -> Option<String> {
        // First try environment variable
        // Do not remove this, env var is used by goose-responder client
        if let Ok(value) = env::var(var_name) {
            return Some(value);
        }

        // Fall back to config
        let config = Config::global();
        config.get_param::<String>(var_name).ok()
    }

    /// Create a new task configuration
    pub fn new(fallback_provider: Option<Arc<dyn Provider>>) -> Self {
        // Get max_turns with environment variable precedence
        let max_turns = Self::get_var(GOOSE_SUBAGENT_MAX_TURNS)
            .and_then(|val| val.parse::<usize>().ok())
            .unwrap_or(DEFAULT_SUBAGENT_MAX_TURNS);

        // Determine provider with environment variable precedence
        let (provider, provider_name) = Self::determine_subagent_provider(fallback_provider);

        Self {
            id: Uuid::new_v4().to_string(),
            provider,
            provider_name,
            max_turns: Some(max_turns),
        }
    }

    /// Get a reference to the provider
    pub fn provider(&self) -> Option<&Arc<dyn Provider>> {
        self.provider.as_ref()
    }

    /// Determine the provider to use for subagent tasks
    /// First tries to use GOOSE_SUBAGENT_PROVIDER from env/config, then falls back to provided provider
    fn determine_subagent_provider(
        fallback_provider: Option<Arc<dyn Provider>>,
    ) -> (Option<Arc<dyn Provider>>, Option<String>) {
        // First try to get the subagent provider from env/config
        if let Some(provider_name) = Self::get_var(GOOSE_SUBAGENT_PROVIDER) {
            // Try to get the model config for the subagent provider
            if let Some(model_name) = Self::get_var(GOOSE_SUBAGENT_MODEL) {
                if let Ok(model_config) = ModelConfig::new(&model_name) {
                    // Create the provider using the factory
                    if let Ok(provider) = create(&provider_name, model_config) {
                        return (Some(provider), Some(provider_name));
                    }
                }
            }
        }

        // Fall back to provided provider - get name from GOOSE_PROVIDER config
        if fallback_provider.is_some() {
            let provider_name =
                Self::get_var("GOOSE_PROVIDER").unwrap_or_else(|| "unknown".to_string());
            return (fallback_provider, Some(provider_name));
        }

        (fallback_provider, None)
    }
}
