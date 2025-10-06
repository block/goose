use crate::agents::ExtensionConfig;
use crate::providers::base::Provider;
use std::env;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

/// Default maximum number of turns for task execution
pub const DEFAULT_SUBAGENT_MAX_TURNS: usize = 25;

/// Environment variable name for configuring max turns
pub const GOOSE_SUBAGENT_MAX_TURNS_ENV_VAR: &str = "GOOSE_SUBAGENT_MAX_TURNS";

/// Configuration for task execution with all necessary dependencies
#[derive(Clone)]
pub struct TaskConfig {
    pub provider: Arc<dyn Provider>,
    pub parent_session_id: String,
    pub parent_working_dir: PathBuf,
    pub extensions: Vec<ExtensionConfig>,
    pub max_turns: Option<usize>,
}

impl fmt::Debug for TaskConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskConfig")
            .field("provider", &"<dyn Provider>")
            .field("parent_session_id", &self.parent_session_id)
            .field("parent_working_dir", &self.parent_working_dir)
            .field("max_turns", &self.max_turns)
            .field("extensions", &self.extensions)
            .finish()
    }
}

impl TaskConfig {
    /// Create a new TaskConfig with all required dependencies
    pub fn new(
        provider: Arc<dyn Provider>,
        parent_session_id: String,
        parent_working_dir: PathBuf,
        extensions: Vec<ExtensionConfig>,
    ) -> Self {
        Self {
            provider,
            parent_session_id,
            parent_working_dir,
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
