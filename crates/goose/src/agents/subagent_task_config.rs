use crate::agents::{ExtensionConfig, ExtensionLoadResult};
use crate::config::Config;
use crate::providers::base::Provider;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Default maximum number of turns for task execution
pub const DEFAULT_SUBAGENT_MAX_TURNS: usize = 25;

/// Configuration for task execution with all necessary dependencies
#[derive(Clone)]
pub struct TaskConfig {
    pub provider: Arc<dyn Provider>,
    pub model_config: goose_providers::model::ModelConfig,
    pub parent_session_id: String,
    pub parent_working_dir: PathBuf,
    pub extensions: Vec<ExtensionConfig>,
    pub max_turns: Option<usize>,
    /// Extensions the caller requested that were unavailable before attach
    /// was attempted (e.g. filtered out by `build_task_config` because they
    /// were not active in the parent session). Pre-populated by the caller;
    /// consumed alongside attach-time results in `subagent_handler` so the
    /// parent LLM sees one entry per requested extension whether the drop
    /// happened at filter time or attach time.
    pub pre_attach_load_results: Vec<ExtensionLoadResult>,
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
    pub fn new(
        provider: Arc<dyn Provider>,
        model_config: goose_providers::model::ModelConfig,
        parent_session_id: &str,
        parent_working_dir: &Path,
        extensions: Vec<ExtensionConfig>,
    ) -> Self {
        Self {
            provider,
            model_config,
            parent_session_id: parent_session_id.to_owned(),
            parent_working_dir: parent_working_dir.to_owned(),
            extensions,
            max_turns: Some(
                Config::global()
                    .get_param::<usize>("GOOSE_SUBAGENT_MAX_TURNS")
                    .unwrap_or(DEFAULT_SUBAGENT_MAX_TURNS),
            ),
            pre_attach_load_results: Vec::new(),
        }
    }

    pub fn with_max_turns(mut self, max_turns: Option<usize>) -> Self {
        if let Some(turns) = max_turns {
            self.max_turns = Some(turns);
        }
        self
    }

    /// Attach a set of pre-computed extension load results to this config.
    ///
    /// Used by callers (e.g. `build_task_config`) to record extensions that
    /// were requested but unavailable before the attach step ran, so the
    /// parent LLM can see the drop in the subagent's tool response.
    pub fn with_pre_attach_load_results(
        mut self,
        results: Vec<ExtensionLoadResult>,
    ) -> Self {
        self.pre_attach_load_results = results;
        self
    }
}
