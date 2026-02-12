use std::path::PathBuf;
use std::sync::Arc;

use futures::StreamExt;
use tokio_util::sync::CancellationToken;

use goose::agents::{AgentEvent, SessionConfig};
use goose::config::paths::Paths;
use goose::conversation::message::{Message, MessageContent};
use goose::execution::manager::AgentManager;
use goose::session::SessionType;

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

    async fn relay_to_session(
        &self,
        message: &IncomingMessage,
        session_id: &str,
    ) -> anyhow::Result<()> {
        self.gateway
            .send_message(&message.user, OutgoingMessage::Typing)
            .await?;

        let agent = self
            .agent_manager
            .get_or_create_agent(session_id.to_string())
            .await?;

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
