use crate::agents::ExtensionConfig;
use crate::providers::base::Provider;
use std::env;
use std::fmt;
use std::sync::Arc;

pub const DEFAULT_SUBAGENT_MAX_TURNS: usize = 25;
pub const GOOSE_SUBAGENT_MAX_TURNS_ENV_VAR: &str = "GOOSE_SUBAGENT_MAX_TURNS";

#[derive(Clone)]
pub struct TaskConfig {
    pub provider: Arc<dyn Provider>,
    pub extensions: Vec<ExtensionConfig>,
    pub max_turns: Option<usize>,
}

impl fmt::Debug for TaskConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskConfig")
            .field("provider", &"<dyn Provider>")
            .field("max_turns", &self.max_turns)
            .field("extensions", &self.extensions)
            .finish()
    }
}

impl TaskConfig {
    pub fn new(provider: Arc<dyn Provider>, extensions: Vec<ExtensionConfig>) -> Self {
        Self {
            provider,
            extensions,
            max_turns: Some(
                env::var(GOOSE_SUBAGENT_MAX_TURNS_ENV_VAR)
                    .ok()
                    .and_then(|val| val.parse::<usize>().ok())
                    .unwrap_or(DEFAULT_SUBAGENT_MAX_TURNS),
            ),
        }
    }
}
