use async_trait::async_trait;
use chrono::{DateTime, Utc};
use goose::conversation::Conversation;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BenchAgentError {
    pub message: String,
    pub level: String, // ERROR, WARN, etc.
    pub timestamp: DateTime<Utc>,
}

// avoid tying benchmarking to current session-impl.
#[async_trait]
pub trait BenchBaseSession: Send + Sync {
    async fn headless(&mut self, message: String) -> anyhow::Result<()>;
    fn session_file(&self) -> Option<PathBuf>;
    fn message_history(&self) -> Conversation;
    fn get_total_token_usage(&self) -> anyhow::Result<Option<i32>>;
}
// struct for managing agent-session-access. to be passed to evals for benchmarking
pub struct BenchAgent {
    session: Box<dyn BenchBaseSession>,
    errors: Arc<Mutex<Vec<BenchAgentError>>>,
}

impl BenchAgent {
    pub fn new(session: Box<dyn BenchBaseSession>) -> Self {
        let errors = Arc::new(Mutex::new(Vec::new()));
        Self { session, errors }
    }

    pub(crate) async fn prompt(&mut self, p: String) -> anyhow::Result<Conversation> {
        // Clear previous errors
        {
            let mut errors = self.errors.lock().await;
            errors.clear();
        }
        self.session.headless(p).await?;
        Ok(self.session.message_history())
    }

    pub async fn get_errors(&self) -> Vec<BenchAgentError> {
        let errors = self.errors.lock().await;
        errors.clone()
    }

    pub(crate) async fn get_token_usage(&self) -> Option<i32> {
        self.session.get_total_token_usage().ok().flatten()
    }
    pub(crate) fn session_file(&self) -> Option<PathBuf> {
        self.session.session_file()
    }
}
