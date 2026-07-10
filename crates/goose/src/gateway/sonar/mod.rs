mod protocol;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config::paths::Paths;

use super::{
    Gateway, GatewayConfig, GatewayHandler, IncomingMessage, OutgoingMessage, PlatformUser,
};
use protocol::{BridgeCommand, BridgeEvent, MAX_PROTOCOL_LINE_BYTES, PROTOCOL_VERSION};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const START_TIMEOUT: Duration = Duration::from_secs(45);

type PendingResponses = Arc<Mutex<HashMap<String, oneshot::Sender<anyhow::Result<()>>>>>;

#[derive(Clone, Debug, Deserialize)]
struct SonarPlatformConfig {
    #[serde(default = "default_bridge_path")]
    bridge_path: String,
    #[serde(default)]
    home: Option<PathBuf>,
    #[serde(default)]
    relays: Vec<String>,
    controllers: Vec<String>,
}

fn default_bridge_path() -> String {
    "goose-sonar-bridge".to_string()
}

#[derive(Clone)]
pub struct SonarGateway {
    config: SonarPlatformConfig,
    controllers: Arc<HashSet<String>>,
    command_tx: Arc<RwLock<Option<mpsc::Sender<BridgeCommand>>>>,
    pending: PendingResponses,
    group_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    info: Arc<StdRwLock<HashMap<String, String>>>,
}

impl SonarGateway {
    pub fn new(config: &GatewayConfig) -> anyhow::Result<Self> {
        let mut platform: SonarPlatformConfig =
            serde_json::from_value(config.platform_config.clone())?;
        if platform.controllers.is_empty() {
            anyhow::bail!("Sonar gateway requires at least one controller npub");
        }
        if platform
            .controllers
            .iter()
            .any(|npub| !npub.starts_with("npub1"))
        {
            anyhow::bail!("Sonar controller identities must use npub1 encoding");
        }
        platform.controllers.sort();
        platform.controllers.dedup();
        let controllers = Arc::new(platform.controllers.iter().cloned().collect());
        Ok(Self {
            config: platform,
            controllers,
            command_tx: Arc::new(RwLock::new(None)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            group_locks: Arc::new(Mutex::new(HashMap::new())),
            info: Arc::new(StdRwLock::new(HashMap::new())),
        })
    }

    fn bridge_home(&self) -> PathBuf {
        self.config
            .home
            .clone()
            .unwrap_or_else(|| Paths::data_dir().join("sonar-gateway"))
    }

    fn bridge_command(&self) -> Command {
        let mut command = Command::new(&self.config.bridge_path);
        command.arg("--home").arg(self.bridge_home());
        for relay in &self.config.relays {
            command.arg("--relay").arg(relay);
        }
        for controller in &self.config.controllers {
            command.arg("--controller").arg(controller);
        }
        command.arg("serve");
        command
    }

    async fn send_command(&self, command: BridgeCommand) -> anyhow::Result<()> {
        let request_id = command.request_id().to_string();
        let sender = self
            .command_tx
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Sonar bridge is not running"))?;
        let (response_tx, response_rx) = oneshot::channel();
        self.pending
            .lock()
            .await
            .insert(request_id.clone(), response_tx);

        if sender.send(command).await.is_err() {
            self.pending.lock().await.remove(&request_id);
            anyhow::bail!("Sonar bridge command channel closed");
        }

        match tokio::time::timeout(COMMAND_TIMEOUT, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => anyhow::bail!("Sonar bridge response channel closed"),
            Err(_) => {
                self.pending.lock().await.remove(&request_id);
                anyhow::bail!("Sonar bridge command timed out")
            }
        }
    }

    async fn complete_message(&self, message_id: String) -> anyhow::Result<()> {
        self.send_command(BridgeCommand::complete(
            Uuid::new_v4().to_string(),
            message_id,
        ))
        .await
    }

    async fn group_lock(&self, group_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.group_locks.lock().await;
        locks
            .entry(group_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn handle_bridge_event(
        &self,
        handler: &GatewayHandler,
        event: BridgeEvent,
    ) -> anyhow::Result<()> {
        if event.version() != PROTOCOL_VERSION {
            anyhow::bail!(
                "unsupported Sonar bridge protocol version {}",
                event.version()
            );
        }

        match event {
            BridgeEvent::Ready { npub, .. } => {
                self.info.write().unwrap().insert("npub".into(), npub);
            }
            BridgeEvent::Response {
                request_id,
                ok,
                error,
                ..
            } => {
                if let Some(response) = self.pending.lock().await.remove(&request_id) {
                    let result = if ok {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!(
                            "{}",
                            error.unwrap_or_else(|| "Sonar bridge command failed".into())
                        ))
                    };
                    let _ = response.send(result);
                }
            }
            BridgeEvent::GroupJoined {
                group_id,
                group_name,
                ..
            } => {
                let gateway = self.clone();
                tokio::spawn(async move {
                    let user = PlatformUser {
                        platform: "sonar".into(),
                        user_id: group_id,
                        display_name: Some(group_name),
                    };
                    let _ = gateway
                        .send_message(
                            &user,
                            OutgoingMessage::Text {
                                body: "Goose joined this group. An allowed controller can now send a Goose pairing code.".into(),
                            },
                        )
                        .await;
                });
            }
            BridgeEvent::Message {
                message_id,
                group_id,
                group_name,
                sender,
                content,
                ..
            } => {
                let gateway = self.clone();
                let handler = handler.clone();
                tokio::spawn(async move {
                    let group_lock = gateway.group_lock(&group_id).await;
                    let _guard = group_lock.lock().await;
                    let user = PlatformUser {
                        platform: "sonar".into(),
                        user_id: group_id,
                        display_name: Some(group_name),
                    };

                    if !gateway.controllers.contains(&sender) {
                        let _ = gateway
                            .send_message(
                                &user,
                                OutgoingMessage::Text {
                                    body: format!(
                                        "{} is not authorized to control Goose in this group.",
                                        sender
                                    ),
                                },
                            )
                            .await;
                        let _ = gateway.complete_message(message_id).await;
                        return;
                    }

                    let incoming = IncomingMessage {
                        user: user.clone(),
                        sender_label: Some(sender),
                        text: content,
                        platform_message_id: Some(message_id.clone()),
                        attachments: Vec::new(),
                    };
                    if let Err(error) = handler.handle_message(incoming).await {
                        let _ = gateway
                            .send_message(
                                &user,
                                OutgoingMessage::Text {
                                    body: format!("Goose failed to handle the command: {error}"),
                                },
                            )
                            .await;
                    }
                    let _ = gateway.complete_message(message_id).await;
                });
            }
        }
        Ok(())
    }

    async fn fail_pending(&self, message: &str) {
        let pending = std::mem::take(&mut *self.pending.lock().await);
        for (_, response) in pending {
            let _ = response.send(Err(anyhow::anyhow!(message.to_string())));
        }
    }
}

#[async_trait]
impl Gateway for SonarGateway {
    fn gateway_type(&self) -> &str {
        "sonar"
    }

    async fn start(
        &self,
        handler: GatewayHandler,
        cancel: CancellationToken,
    ) -> anyhow::Result<()> {
        let mut child = self
            .bridge_command()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Sonar bridge stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Sonar bridge stdout unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Sonar bridge stderr unavailable"))?;

        let mut output = FramedRead::new(
            stdout,
            LinesCodec::new_with_max_length(MAX_PROTOCOL_LINE_BYTES),
        );
        let ready_line = tokio::time::timeout(START_TIMEOUT, output.next())
            .await
            .map_err(|_| anyhow::anyhow!("Sonar bridge did not become ready"))?
            .ok_or_else(|| anyhow::anyhow!("Sonar bridge exited before ready"))??;
        let ready: BridgeEvent = serde_json::from_str(&ready_line)?;
        if !matches!(&ready, BridgeEvent::Ready { .. }) {
            anyhow::bail!("Sonar bridge sent a non-ready first event");
        }
        self.handle_bridge_event(&handler, ready).await?;

        let (command_tx, mut command_rx) = mpsc::channel::<BridgeCommand>(64);
        *self.command_tx.write().await = Some(command_tx);
        let writer = tokio::spawn(async move {
            let mut stdin = BufWriter::new(stdin);
            while let Some(command) = command_rx.recv().await {
                let mut line = serde_json::to_vec(&command)?;
                if line.len() > MAX_PROTOCOL_LINE_BYTES {
                    anyhow::bail!("Sonar bridge command exceeds protocol limit");
                }
                line.push(b'\n');
                stdin.write_all(&line).await?;
                stdin.flush().await?;
            }
            Ok::<(), anyhow::Error>(())
        });
        let stderr_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!(bridge = "sonar", message = %line);
            }
        });

        let result = loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    break Ok(());
                }
                line = output.next() => {
                    match line {
                        Some(Ok(line)) => {
                            let event: BridgeEvent = match serde_json::from_str(&line) {
                                Ok(event) => event,
                                Err(error) => break Err(error.into()),
                            };
                            if let Err(error) = self.handle_bridge_event(&handler, event).await {
                                break Err(error);
                            }
                        }
                        Some(Err(error)) => break Err(error.into()),
                        None => break Err(anyhow::anyhow!("Sonar bridge stdout closed")),
                    }
                }
                status = child.wait() => {
                    break match status {
                        Ok(status) => Err(anyhow::anyhow!("Sonar bridge exited with {status}")),
                        Err(error) => Err(error.into()),
                    };
                }
            }
        };

        *self.command_tx.write().await = None;
        self.fail_pending("Sonar bridge stopped").await;
        writer.abort();
        stderr_task.abort();
        if child.try_wait()?.is_none() {
            child.kill().await?;
        }
        result
    }

    async fn send_message(
        &self,
        user: &PlatformUser,
        message: OutgoingMessage,
    ) -> anyhow::Result<()> {
        match message {
            OutgoingMessage::Text { body } => {
                self.send_command(BridgeCommand::send(
                    Uuid::new_v4().to_string(),
                    user.user_id.clone(),
                    body,
                ))
                .await
            }
            OutgoingMessage::Typing => Ok(()),
        }
    }

    async fn validate_config(&self) -> anyhow::Result<()> {
        let output = tokio::time::timeout(
            Duration::from_secs(10),
            Command::new(&self.config.bridge_path)
                .arg("--version")
                .output(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Sonar bridge version check timed out"))??;
        if !output.status.success() {
            anyhow::bail!("Sonar bridge version check failed");
        }
        Ok(())
    }

    fn info(&self) -> HashMap<String, String> {
        self.info.read().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(value: serde_json::Value) -> GatewayConfig {
        GatewayConfig {
            gateway_type: "sonar".into(),
            platform_config: value,
            max_sessions: 0,
        }
    }

    #[test]
    fn requires_controller_allowlist() {
        let result = SonarGateway::new(&config(serde_json::json!({
            "controllers": []
        })));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_npub_controllers() {
        let result = SonarGateway::new(&config(serde_json::json!({
            "controllers": ["hex-key"]
        })));
        assert!(result.is_err());
    }

    #[test]
    fn deduplicates_controllers() {
        let gateway = SonarGateway::new(&config(serde_json::json!({
            "controllers": ["npub1controller", "npub1controller"]
        })))
        .unwrap();
        assert_eq!(gateway.controllers.len(), 1);
    }

    #[tokio::test]
    async fn failed_command_without_bridge_does_not_leak_pending_response() {
        let gateway = SonarGateway::new(&config(serde_json::json!({
            "controllers": ["npub1controller"]
        })))
        .unwrap();

        let result = gateway
            .send_command(BridgeCommand::complete("request".into(), "message".into()))
            .await;

        assert!(result.is_err());
        assert!(gateway.pending.lock().await.is_empty());
    }
}
