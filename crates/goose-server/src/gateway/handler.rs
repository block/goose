use std::path::PathBuf;
use std::sync::Arc;

use futures::StreamExt;
use tokio_util::sync::CancellationToken;

use goose::agents::{AgentEvent, ExtensionConfig, SessionConfig};
use goose::config::extensions::get_enabled_extensions;
use goose::config::paths::Paths;
use goose::config::Config;
use goose::conversation::message::{Message, MessageContent};
use goose::execution::manager::AgentManager;
use goose::model::ModelConfig;
use goose::session::SessionType;
use goose::session::{EnabledExtensionsState, ExtensionState, Session};

use super::pairing::PairingStore;
use super::{Gateway, GatewayConfig, IncomingMessage, OutgoingMessage, PairingState, PlatformUser};

#[derive(Clone)]
pub struct GatewayHandler {
    agent_manager: Arc<AgentManager>,
    pairing_store: Arc<PairingStore>,
    gateway: Arc<dyn Gateway>,
    config: GatewayConfig,
}

impl GatewayHandler {
    pub fn new(
        agent_manager: Arc<AgentManager>,
        pairing_store: Arc<PairingStore>,
        gateway: Arc<dyn Gateway>,
        config: GatewayConfig,
    ) -> Self {
        Self {
            agent_manager,
            pairing_store,
            gateway,
            config,
        }
    }

    pub async fn handle_message(&self, message: IncomingMessage) -> anyhow::Result<()> {
        let pairing = self.pairing_store.get(&message.user).await?;

        match pairing {
            PairingState::Unpaired => {
                if let Some(gateway_type) = self.try_consume_code(message.text.trim()).await? {
                    if gateway_type == self.config.gateway_type {
                        self.complete_pairing(&message.user).await?;
                    } else {
                        self.gateway
                            .send_message(
                                &message.user,
                                OutgoingMessage::Error {
                                    message: "That code is for a different gateway.".into(),
                                },
                            )
                            .await?;
                    }
                } else {
                    self.gateway
                        .send_message(
                            &message.user,
                            OutgoingMessage::Text {
                                body: "Welcome! Enter your pairing code to connect to Goose."
                                    .into(),
                            },
                        )
                        .await?;
                }
            }
            PairingState::PendingCode { code, expires_at } => {
                let now = chrono::Utc::now().timestamp();
                if now > expires_at {
                    self.pairing_store
                        .set(&message.user, PairingState::Unpaired)
                        .await?;
                    self.gateway
                        .send_message(
                            &message.user,
                            OutgoingMessage::Text {
                                body: "Your pairing code expired. Please request a new one.".into(),
                            },
                        )
                        .await?;
                } else if message.text.trim().eq_ignore_ascii_case(&code) {
                    self.complete_pairing(&message.user).await?;
                } else {
                    self.gateway
                        .send_message(
                            &message.user,
                            OutgoingMessage::Text {
                                body: "Invalid code. Please try again.".into(),
                            },
                        )
                        .await?;
                }
            }
            PairingState::Paired { session_id, .. } => {
                self.relay_to_session(&message, &session_id).await?;
            }
        }

        Ok(())
    }

    async fn try_consume_code(&self, text: &str) -> anyhow::Result<Option<String>> {
        let normalized = text.to_uppercase().replace(['-', ' '], "");
        if normalized.len() == 6
            && normalized
                .chars()
                .all(|c| "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(c))
        {
            return self.pairing_store.consume_pending_code(&normalized).await;
        }
        Ok(None)
    }

    async fn complete_pairing(&self, user: &PlatformUser) -> anyhow::Result<()> {
        let working_dir = gateway_working_dir(&user.platform, &user.user_id);
        std::fs::create_dir_all(&working_dir)?;

        let session_name = format!(
            "{}/{}",
            user.platform,
            user.display_name.as_deref().unwrap_or(&user.user_id)
        );

        let session = self
            .agent_manager
            .session_manager()
            .create_session(working_dir, session_name, SessionType::Gateway)
            .await?;

        let manager = self.agent_manager.session_manager();
        let config = Config::global();

        // Store the current provider and model config on the session so the agent
        // can be restored after LRU eviction, matching the start_agent flow.
        let mut update = manager.update(&session.id);
        if let Ok(provider) = config.get_goose_provider() {
            update = update.provider_name(provider);
        }
        if let Ok(model_name) = config.get_goose_model() {
            if let Ok(model_config) = ModelConfig::new(&model_name) {
                update = update.model_config(model_config);
            }
        }

        // Store default extensions so load_extensions_from_session works.
        let extensions = get_enabled_extensions();
        let extensions_state = EnabledExtensionsState::new(extensions);
        let mut extension_data = session.extension_data.clone();
        if let Err(e) = extensions_state.to_extension_data(&mut extension_data) {
            tracing::warn!(error = %e, "failed to initialize gateway session extensions");
        } else {
            update = update.extension_data(extension_data);
        }

        update.apply().await?;

        let now = chrono::Utc::now().timestamp();
        self.pairing_store
            .set(
                user,
                PairingState::Paired {
                    session_id: session.id.clone(),
                    paired_at: now,
                },
            )
            .await?;

        self.gateway
            .send_message(
                user,
                OutgoingMessage::Text {
                    body: "Paired! You can now chat with Goose.".into(),
                },
            )
            .await?;

        Ok(())
    }

    /// Sync the session's provider, model, and extensions with the current
    /// global config so gateway sessions always reflect what the user has
    /// configured in the desktop app.  Returns `true` if extensions changed
    /// (which means the caller must recreate the agent so stale extension
    /// processes are torn down).
    async fn sync_session_config(&self, session: &Session) -> anyhow::Result<bool> {
        let config = Config::global();
        let manager = self.agent_manager.session_manager();

        // --- current global config ---
        let current_provider = config.get_goose_provider().ok();
        let current_model_name = config.get_goose_model().ok();
        let current_extensions = get_enabled_extensions();

        // --- what the session has ---
        let session_extensions: Vec<ExtensionConfig> =
            EnabledExtensionsState::from_extension_data(&session.extension_data)
                .map(|s| s.extensions)
                .unwrap_or_default();

        let provider_changed = current_provider.as_deref() != session.provider_name.as_deref();
        let model_changed = current_model_name.as_deref()
            != session
                .model_config
                .as_ref()
                .map(|m| m.model_name.as_str());
        let extensions_changed = current_extensions != session_extensions;

        if !provider_changed && !model_changed && !extensions_changed {
            return Ok(false);
        }

        tracing::info!(
            session_id = %session.id,
            provider_changed,
            model_changed,
            extensions_changed,
            "syncing gateway session with current config"
        );

        let mut update = manager.update(&session.id);

        if let Some(ref provider) = current_provider {
            update = update.provider_name(provider);
        }
        if let Some(ref model_name) = current_model_name {
            if let Ok(model_config) = ModelConfig::new(model_name) {
                update = update.model_config(model_config);
            }
        }

        if extensions_changed {
            let extensions_state = EnabledExtensionsState::new(current_extensions);
            let mut extension_data = session.extension_data.clone();
            if let Err(e) = extensions_state.to_extension_data(&mut extension_data) {
                tracing::warn!(error = %e, "failed to update gateway session extensions");
            } else {
                update = update.extension_data(extension_data);
            }
        }

        update.apply().await?;
        Ok(extensions_changed)
    }

    async fn relay_to_session(
        &self,
        message: &IncomingMessage,
        session_id: &str,
    ) -> anyhow::Result<()> {
        self.gateway
            .send_message(&message.user, OutgoingMessage::Typing)
            .await?;

        let session = self
            .agent_manager
            .session_manager()
            .get_session(session_id, false)
            .await?;

        // Sync provider/model/extensions with the user's current desktop config.
        // If extensions changed we must tear down the old agent so stale
        // extension processes don't linger.
        let extensions_changed = self.sync_session_config(&session).await?;
        if extensions_changed {
            let _ = self.agent_manager.remove_session(session_id).await;
        }

        let agent = self
            .agent_manager
            .get_or_create_agent(session_id.to_string())
            .await?;

        // Re-read the session after sync so restore picks up the new values.
        let session = self
            .agent_manager
            .session_manager()
            .get_session(session_id, false)
            .await?;

        // Ensure provider is configured (handles first use and LRU eviction).
        if let Err(e) = agent.restore_provider_from_session(&session).await {
            self.gateway
                .send_message(
                    &message.user,
                    OutgoingMessage::Error {
                        message: format!("Failed to configure provider: {e}"),
                    },
                )
                .await?;
            return Ok(());
        }

        // Load extensions (skips any already loaded on the agent).
        agent.load_extensions_from_session(&session).await;

        let cancel = CancellationToken::new();
        let user_message = Message::user().with_text(&message.text);

        let session_config = SessionConfig {
            id: session_id.to_string(),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        let mut stream = match agent
            .reply(user_message, session_config, Some(cancel))
            .await
        {
            Ok(s) => s,
            Err(e) => {
                self.gateway
                    .send_message(
                        &message.user,
                        OutgoingMessage::Error {
                            message: format!("Failed to start agent: {e}"),
                        },
                    )
                    .await?;
                return Ok(());
            }
        };

        let mut response_text = String::new();

        while let Some(event) = stream.next().await {
            match event {
                Ok(AgentEvent::Message(msg)) => {
                    if msg.role == rmcp::model::Role::Assistant {
                        for content in &msg.content {
                            if let MessageContent::Text(t) = content {
                                if !response_text.is_empty() {
                                    response_text.push('\n');
                                }
                                response_text.push_str(&t.text);
                            }
                        }
                    }
                }
                Ok(AgentEvent::McpNotification(_)) => {}
                Ok(AgentEvent::ModelChange { .. }) => {}
                Ok(AgentEvent::HistoryReplaced(_)) => {}
                Err(e) => {
                    tracing::error!(error = %e, "agent stream error");
                    self.gateway
                        .send_message(
                            &message.user,
                            OutgoingMessage::Error {
                                message: format!("Agent error: {e}"),
                            },
                        )
                        .await?;
                    return Ok(());
                }
            }
        }

        if response_text.is_empty() {
            response_text = "(No response)".to_string();
        }

        self.gateway
            .send_message(
                &message.user,
                OutgoingMessage::Text {
                    body: response_text,
                },
            )
            .await?;

        Ok(())
    }
}

fn gateway_working_dir(platform: &str, user_id: &str) -> PathBuf {
    Paths::config_dir()
        .join("gateway")
        .join(platform)
        .join(user_id)
}
