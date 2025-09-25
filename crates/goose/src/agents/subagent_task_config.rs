use crate::providers::base::Provider;
use std::env;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

/// Default maximum number of turns for task execution
pub const DEFAULT_SUBAGENT_MAX_TURNS: usize = 25;

/// Environment variable name for configuring max turns
pub const GOOSE_SUBAGENT_MAX_TURNS_ENV_VAR: &str = "GOOSE_SUBAGENT_MAX_TURNS";

/// Configuration for task execution with all necessary dependencies
#[derive(Clone)]
pub struct TaskConfig {
    pub id: String,
    pub provider: Option<Arc<dyn Provider>>,
    pub parent_session_id: Option<String>,
    pub max_turns: Option<usize>,
    pub extensions: Option<Vec<crate::agents::extension::ExtensionConfig>>,
}

impl fmt::Debug for TaskConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskConfig")
            .field("id", &self.id)
            .field("provider", &"<dyn Provider>")
            .field("max_turns", &self.max_turns)
            .field("extensions", &self.extensions)
            .finish()
    }
}

impl TaskConfig {
    /// Create a new TaskConfig with all required dependencies
    pub fn new(provider: Option<Arc<dyn Provider>>, parent_session_id: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            provider,
            parent_session_id,
            max_turns: Some(
                env::var(GOOSE_SUBAGENT_MAX_TURNS_ENV_VAR)
                    .ok()
                    .and_then(|val| val.parse::<usize>().ok())
                    .unwrap_or(DEFAULT_SUBAGENT_MAX_TURNS),
            ),
            extensions: None,
        }
    }

    /// Get a reference to the provider
    pub fn provider(&self) -> Option<&Arc<dyn Provider>> {
        self.provider.as_ref()
    }
}
