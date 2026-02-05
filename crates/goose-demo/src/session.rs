//! Session - A multi-turn conversation with the agent
//!
//! A Session owns all the state for a conversation:
//! - Message history
//! - Enabled extensions (kept alive between prompts)
//! - Model configuration
//!
//! Sessions are cached in memory by the Server to preserve extension connections.
//! They can be reconstructed from the database if the server restarts.

use std::sync::Arc;

use agent_client_protocol_schema::{ContentBlock, PromptRequest, SessionId, StopReason};
use parking_lot::RwLock;
use rig::message::Message;
use tracing::{info, instrument, warn};

use crate::agent_loop::{self, StepResult};
use crate::db::{Database, SessionData};
use crate::extension::{EnabledExtensions, ExtensionCatalog};
use crate::notifier::Notifier;
use crate::provider::{Model, ProviderConfig};
use crate::Result;

/// A session represents a multi-turn conversation
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,

    /// Database for persistence
    db: Arc<Database>,

    /// Conversation history
    messages: Vec<Message>,

    /// Base system prompt / preamble (extension info is added dynamically)
    base_preamble: Option<String>,

    /// Enabled extensions (session-scoped, owns running instances)
    extensions: EnabledExtensions,

    /// LLM model for this session
    model: Model,
}

impl Session {
    /// Create a new session (not yet persisted)
    pub fn new(
        id: SessionId,
        db: Arc<Database>,
        catalog: Arc<RwLock<ExtensionCatalog>>,
        provider_config: ProviderConfig,
    ) -> Result<Self> {
        let model = Model::from_config(&provider_config)?;
        let extensions = EnabledExtensions::new(catalog);

        Ok(Self {
            id,
            db,
            messages: Vec::new(),
            base_preamble: None,
            extensions,
            model,
        })
    }

    /// Reconstruct a session from database data
    ///
    /// Note: Extensions are NOT re-enabled here. They will be
    /// re-enabled based on the persisted list.
    pub async fn from_db(
        id: SessionId,
        db: Arc<Database>,
        catalog: Arc<RwLock<ExtensionCatalog>>,
        provider_config: ProviderConfig,
        data: &SessionData,
    ) -> Result<Self> {
        let model = Model::from_config(&provider_config)?;
        let messages: Vec<Message> = serde_json::from_str(&data.messages_json)?;
        let mut extensions = EnabledExtensions::new(catalog);

        // Re-enable previously enabled extensions
        let enabled_names: Vec<String> = serde_json::from_str(&data.enabled_extensions_json)?;
        for name in enabled_names {
            if let Err(e) = extensions.enable(&name).await {
                // Extension might no longer exist in catalog - log and continue
                warn!(extension = %name, error = %e, "Failed to re-enable extension");
            }
        }

        Ok(Self {
            id,
            db,
            messages,
            base_preamble: data.preamble.clone(),
            extensions,
            model,
        })
    }

    /// Handle a prompt request
    ///
    /// This is the main entry point for processing user input.
    /// It runs the agent loop until completion, persisting at checkpoints.
    #[instrument(skip(self, notifier), fields(session_id = %self.id))]
    pub async fn prompt<N: Notifier>(
        &mut self,
        request: PromptRequest,
        notifier: &N,
    ) -> Result<StopReason> {
        // Add incoming prompt to history
        for block in &request.prompt {
            if let ContentBlock::Text(t) = block {
                self.messages.push(Message::user(&t.text));
            }
        }

        // Run the agent loop
        loop {
            let result = agent_loop::run_step(
                &self.id,
                &mut self.messages,
                self.base_preamble.as_deref(),
                &mut self.extensions,
                &self.model,
                notifier,
            )
            .await?;

            match result {
                StepResult::ToolsExecuted => {
                    // Checkpoint - persist for crash recovery
                    self.save_state()?;
                }
                StepResult::Done => {
                    // Final persist and return
                    self.save_state()?;
                    info!("Prompt completed");
                    return Ok(StopReason::EndTurn);
                }
            }
        }
    }

    /// Enable an extension by name
    #[instrument(skip(self), fields(session_id = %self.id, extension = %name))]
    pub async fn enable_extension(&mut self, name: &str) -> Result<()> {
        self.extensions.enable(name).await?;
        self.save_enabled_extensions()?;
        Ok(())
    }

    /// Disable an extension by name
    #[instrument(skip(self), fields(session_id = %self.id, extension = %name))]
    pub fn disable_extension(&mut self, name: &str) -> Result<()> {
        self.extensions.disable(name)?;
        self.save_enabled_extensions()?;
        Ok(())
    }

    /// Set the base system preamble
    pub fn set_preamble(&mut self, preamble: Option<String>) -> Result<()> {
        self.base_preamble = preamble.clone();
        self.db
            .update_preamble(&self.id.to_string(), preamble.as_deref())
    }

    /// Save all session state to database
    fn save_state(&self) -> Result<()> {
        self.save_messages()?;
        self.save_enabled_extensions()?;
        Ok(())
    }

    /// Save messages to database
    fn save_messages(&self) -> Result<()> {
        let messages_json = serde_json::to_string(&self.messages)?;
        self.db
            .update_messages(&self.id.to_string(), &messages_json)
    }

    /// Save enabled extensions to database
    fn save_enabled_extensions(&self) -> Result<()> {
        let enabled_names = self.extensions.enabled_names();
        let enabled_json = serde_json::to_string(&enabled_names)?;
        self.db
            .update_enabled_extensions(&self.id.to_string(), &enabled_json)
    }
}
