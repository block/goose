//! `SessionContextProvider` — the trait that abstracts session storage so the
//! agent loop doesn't have to depend on a concrete `SessionManager`.
//!
//! The runtime impl in this crate ([`crate::session::SessionManager`]) keeps
//! the SQLite-backed storage that `goose-cli` and `goose-server` use. The
//! `goose-core` carve-out reads the trait, not the impl.

use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;

use crate::config::GooseMode;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::model::ModelConfig;
use crate::session::extension_data::ExtensionData;
use crate::session::session_manager::SessionType;
use crate::session::Session;

/// Pending mutations to apply to a session in one transactional call.
///
/// Use the chainable setters and pass the result to
/// [`SessionContextProvider::apply_update`]. Fields default to `None`,
/// meaning "leave the existing value untouched". The token / cost fields are
/// double-`Option` so a caller can distinguish "don't touch" (outer `None`)
/// from "set to `None`" (`Some(None)`).
#[derive(Default, Debug, Clone)]
pub struct SessionUpdate {
    pub extension_data: Option<ExtensionData>,
    pub provider_name: Option<String>,
    pub model_config: Option<ModelConfig>,
    pub clear_model_config: bool,
    pub goose_mode: Option<GooseMode>,
    pub total_tokens: Option<Option<i32>>,
    pub input_tokens: Option<Option<i32>>,
    pub output_tokens: Option<Option<i32>>,
    pub accumulated_total_tokens: Option<Option<i32>>,
    pub accumulated_input_tokens: Option<Option<i32>>,
    pub accumulated_output_tokens: Option<Option<i32>>,
    pub accumulated_cost: Option<Option<f64>>,
    pub schedule_id: Option<Option<String>>,
}

impl SessionUpdate {
    pub fn extension_data(mut self, data: ExtensionData) -> Self {
        self.extension_data = Some(data);
        self
    }

    pub fn provider_name(mut self, name: impl Into<String>) -> Self {
        self.provider_name = Some(name.into());
        self
    }

    pub fn model_config(mut self, model_config: ModelConfig) -> Self {
        self.model_config = Some(model_config);
        self.clear_model_config = false;
        self
    }

    pub fn clear_model_config(mut self) -> Self {
        self.clear_model_config = true;
        self.model_config = None;
        self
    }

    pub fn goose_mode(mut self, mode: GooseMode) -> Self {
        self.goose_mode = Some(mode);
        self
    }

    pub fn total_tokens(mut self, tokens: Option<i32>) -> Self {
        self.total_tokens = Some(tokens);
        self
    }

    pub fn input_tokens(mut self, tokens: Option<i32>) -> Self {
        self.input_tokens = Some(tokens);
        self
    }

    pub fn output_tokens(mut self, tokens: Option<i32>) -> Self {
        self.output_tokens = Some(tokens);
        self
    }

    pub fn accumulated_total_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_total_tokens = Some(tokens);
        self
    }

    pub fn accumulated_input_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_input_tokens = Some(tokens);
        self
    }

    pub fn accumulated_output_tokens(mut self, tokens: Option<i32>) -> Self {
        self.accumulated_output_tokens = Some(tokens);
        self
    }

    pub fn accumulated_cost(mut self, cost: Option<f64>) -> Self {
        self.accumulated_cost = Some(cost);
        self
    }

    pub fn schedule_id(mut self, id: Option<String>) -> Self {
        self.schedule_id = Some(id);
        self
    }
}

/// Read and mutate session state without depending on a concrete storage backend.
///
/// The runtime impl is [`crate::session::SessionManager`]. `goose-core` consumers
/// will eventually depend only on this trait, not on `SessionManager`'s
/// SQLite-backed storage.
#[async_trait]
pub trait SessionContextProvider: Send + Sync {
    async fn get_session(&self, id: &str, include_messages: bool) -> Result<Session>;

    async fn create_session(
        &self,
        working_dir: PathBuf,
        name: String,
        session_type: SessionType,
        goose_mode: GooseMode,
    ) -> Result<Session>;

    async fn add_message(&self, id: &str, message: &Message) -> Result<()>;

    async fn replace_conversation(&self, id: &str, conversation: &Conversation) -> Result<()>;

    async fn apply_update(&self, id: &str, update: SessionUpdate) -> Result<()>;
}
