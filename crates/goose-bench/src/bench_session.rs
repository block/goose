use async_trait::async_trait;
use chrono::{DateTime, Utc};
use goose::message::Message;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BenchAgentError {
    pub message: String,
    pub level: String,
    pub timestamp: DateTime<Utc>,
}

#[async_trait]
pub trait BenchBaseSession: Send + Sync {
    async fn headless(&mut self, message: String) -> anyhow::Result<()>;
    fn session_file(&self) -> PathBuf;
    fn message_history(&self) -> Vec<Message>;
    async fn override_system_prompt(&self, override_prompt: String);
    fn get_total_token_usage(&self) -> anyhow::Result<Option<i32>>;
    async fn cleanup_extensions(&self) -> anyhow::Result<()>;
    // New method to access the underlying agent for interaction control
    fn get_agent(&self) -> &goose::agents::Agent;
    fn get_messages_mut(&mut self) -> &mut Vec<Message>;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
pub struct BenchAgent {
    pub session: Box<dyn BenchBaseSession>,
    errors: Arc<Mutex<Vec<BenchAgentError>>>,
}

impl BenchAgent {
    pub fn new(session: Box<dyn BenchBaseSession>) -> Self {
        let errors = Arc::new(Mutex::new(Vec::new()));
        Self { session, errors }
    }

    pub(crate) async fn prompt(&mut self, p: String) -> anyhow::Result<Vec<Message>> {
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

    pub async fn clear_errors(&self) {
        let mut errors = self.errors.lock().await;
        errors.clear();
    }

    pub async fn prompt_with_limit(
        &mut self,
        prompt: String,
        max_interactions: usize,
    ) -> anyhow::Result<Vec<Message>> {
        if let Some(interaction_limited) = self.try_downcast_interaction_limited() {
            interaction_limited
                .prompt_with_limit(prompt, max_interactions)
                .await
        } else {
            self.prompt(prompt).await
        }
    }

    pub async fn prompt_multi_turn(
        &mut self,
        prompts: Vec<String>,
    ) -> anyhow::Result<Vec<Message>> {
        if let Some(interaction_limited) = self.try_downcast_interaction_limited() {
            interaction_limited.prompt_multi_turn(prompts).await
        } else {
            if prompts.is_empty() {
                return Err(anyhow::anyhow!("At least one prompt is required"));
            }
            self.prompt(prompts.into_iter().next().unwrap()).await
        }
    }

    fn try_downcast_interaction_limited(
        &mut self,
    ) -> Option<&mut crate::interaction_limited_agent::InteractionLimitedAgent> {
        self.session
            .as_any_mut()
            .downcast_mut::<crate::interaction_limited_agent::InteractionLimitedAgent>()
    }

    pub(crate) async fn get_token_usage(&self) -> Option<i32> {
        self.session.get_total_token_usage().ok().flatten()
    }
    pub(crate) fn session_file(&self) -> PathBuf {
        self.session.session_file()
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let cleanup_timeout = tokio::time::Duration::from_secs(5);
        match tokio::time::timeout(cleanup_timeout, self.session.cleanup_extensions()).await {
            Ok(result) => result,
            Err(_timeout) => Ok(()),
        }
    }

    pub async fn reset_for_reuse(&mut self) -> anyhow::Result<()> {
        self.clear_errors().await;
        self.session.get_messages_mut().clear();
        
        if let Some(interaction_limited) = self.try_downcast_interaction_limited() {
            interaction_limited.reset().await?;
        }
        Ok(())
    }
}
