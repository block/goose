use std::collections::HashMap;
use std::sync::Arc;

use agent_client_protocol::{Agent, ClientSideConnection, ProtocolVersion};
use agent_client_protocol_schema::{
    ContentBlock, InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse,
    PromptRequest, PromptResponse, RequestPermissionOutcome, SelectedPermissionOutcome, SessionId,
    SessionNotification, SessionUpdate, SetSessionModeRequest, SetSessionModeResponse, TextContent,
};
use anyhow::{bail, Result};

use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::agent_manager::health::{AgentHealth, AgentState, AgentStatus};
use crate::agent_manager::spawner::{spawn_agent, SpawnedAgent};
use crate::registry::manifest::{AgentDistribution, RegistryEntry, RegistryEntryDetail};

pub struct AgentHandle {
    tx: mpsc::Sender<AgentCommand>,
    pub info: InitializeResponse,
    pub agent_id: String,
    collected_text: Arc<Mutex<Vec<String>>>,
    health: Arc<AgentHealth>,
}

impl AgentHandle {
    pub async fn new_session(&self, req: NewSessionRequest) -> Result<NewSessionResponse> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(AgentCommand::NewSession {
                req,
                reply: reply_tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("agent connection closed"))?;
        let result = reply_rx.await?;
        match &result {
            Ok(_) => self.health.record_success().await,
            Err(_) => self.health.record_failure().await,
        }
        result
    }

    pub async fn prompt(&self, req: PromptRequest) -> Result<PromptResponse> {
        self.collected_text.lock().await.clear();
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(AgentCommand::Prompt {
                req,
                reply: reply_tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("agent connection closed"))?;
        let result = reply_rx.await?;
        match &result {
            Ok(_) => self.health.record_success().await,
            Err(_) => self.health.record_failure().await,
        }
        result
    }

    pub async fn drain_text(&self) -> Vec<String> {
        std::mem::take(&mut *self.collected_text.lock().await)
    }

    pub async fn health_status(&self) -> AgentStatus {
        AgentStatus {
            agent_id: self.agent_id.clone(),
            state: self.health.state().await,
            consecutive_failures: self.health.consecutive_failures(),
            last_activity_secs_ago: self.health.last_activity().await.elapsed().as_secs(),
        }
    }

    pub fn is_channel_alive(&self) -> bool {
        !self.tx.is_closed()
    }

    pub async fn set_mode(&self, req: SetSessionModeRequest) -> Result<SetSessionModeResponse> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(AgentCommand::SetMode {
                req,
                reply: reply_tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("agent connection closed"))?;
        reply_rx.await?
    }

    pub async fn shutdown(self) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(AgentCommand::Shutdown { reply: reply_tx })
            .await;
        reply_rx.await.unwrap_or(Ok(()))
    }
}

enum AgentCommand {
    NewSession {
        req: NewSessionRequest,
        reply: oneshot::Sender<Result<NewSessionResponse>>,
    },
    Prompt {
        req: PromptRequest,
        reply: oneshot::Sender<Result<PromptResponse>>,
    },
    SetMode {
        req: SetSessionModeRequest,
        reply: oneshot::Sender<Result<SetSessionModeResponse>>,
    },
    Shutdown {
        reply: oneshot::Sender<Result<()>>,
    },
}

struct OrchestratorClient {
    collected_text: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait(?Send)]
impl agent_client_protocol::Client for OrchestratorClient {
    async fn request_permission(
        &self,
        args: agent_client_protocol_schema::RequestPermissionRequest,
    ) -> agent_client_protocol_schema::Result<agent_client_protocol_schema::RequestPermissionResponse>
    {
        let option_id = args
            .options
            .first()
            .map(|o| o.option_id.clone())
            .unwrap_or_else(|| "allow_once".into());
        Ok(
            agent_client_protocol_schema::RequestPermissionResponse::new(
                RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id)),
            ),
        )
    }

    async fn session_notification(
        &self,
        args: SessionNotification,
    ) -> agent_client_protocol_schema::Result<()> {
        if let SessionUpdate::AgentMessageChunk(chunk) = args.update {
            if let ContentBlock::Text(text) = chunk.content {
                self.collected_text.lock().await.push(text.text.clone());
            }
        }
        Ok(())
    }
}

pub struct AgentClientManager {
    agents: Arc<Mutex<HashMap<String, AgentHandle>>>,
}

impl Default for AgentClientManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentClientManager {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn connect_agent(&self, agent_id: String, entry: &RegistryEntry) -> Result<()> {
        let dist = match &entry.detail {
            RegistryEntryDetail::Agent(detail) => detail
                .distribution
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("agent has no distribution info"))?,
            _ => bail!("registry entry is not an agent"),
        };
        self.connect_with_distribution(agent_id, dist).await
    }

    pub async fn connect_with_distribution(
        &self,
        agent_id: String,
        distribution: &AgentDistribution,
    ) -> Result<()> {
        let distribution = distribution.clone();
        let id = agent_id.clone();
        let collected_text = Arc::new(Mutex::new(Vec::new()));
        let text_ref = collected_text.clone();

        let (handle_tx, handle_rx) = oneshot::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime");
            let local = LocalSet::new();
            local.block_on(&rt, async move {
                match run_agent_connection(id.clone(), &distribution, cmd_rx, text_ref).await {
                    Ok((info, io_task)) => {
                        let _ = handle_tx.send(Ok(AgentHandle {
                            tx: cmd_tx,
                            info,
                            agent_id: id,
                            collected_text,
                            health: Arc::new(AgentHealth::default()),
                        }));
                        let _ = io_task.await;
                    }
                    Err(e) => {
                        let _ = handle_tx.send(Err(e));
                    }
                }
            });
        });

        let handle = handle_rx.await??;
        self.agents.lock().await.insert(agent_id, handle);
        Ok(())
    }

    pub async fn prompt_agent(&self, agent_id: &str, req: PromptRequest) -> Result<PromptResponse> {
        let agents = self.agents.lock().await;
        let handle = agents
            .get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("agent '{agent_id}' not connected"))?;
        handle.prompt(req).await
    }

    pub async fn prompt_agent_text(
        &self,
        agent_id: &str,
        session_id: &SessionId,
        instructions: &str,
    ) -> Result<String> {
        let prompt = vec![ContentBlock::Text(TextContent::new(
            instructions.to_string(),
        ))];
        let req = PromptRequest::new(session_id.clone(), prompt);

        self.prompt_agent(agent_id, req).await?;

        let agents = self.agents.lock().await;
        let handle = agents
            .get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("agent '{agent_id}' not connected"))?;
        let texts = handle.drain_text().await;
        Ok(texts.join(""))
    }

    pub async fn new_session(
        &self,
        agent_id: &str,
        req: NewSessionRequest,
    ) -> Result<NewSessionResponse> {
        let agents = self.agents.lock().await;
        let handle = agents
            .get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("agent '{agent_id}' not connected"))?;
        handle.new_session(req).await
    }

    pub async fn set_mode(
        &self,
        agent_id: &str,
        req: SetSessionModeRequest,
    ) -> Result<SetSessionModeResponse> {
        let agents = self.agents.lock().await;
        let handle = agents
            .get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("agent '{agent_id}' not connected"))?;
        handle.set_mode(req).await
    }

    pub async fn list_agents(&self) -> Vec<String> {
        self.agents.lock().await.keys().cloned().collect()
    }

    pub async fn get_agent_info(&self, agent_id: &str) -> Option<InitializeResponse> {
        let agents = self.agents.lock().await;
        agents.get(agent_id).map(|h| h.info.clone())
    }

    pub async fn disconnect_agent(&self, agent_id: &str) -> Result<()> {
        let handle = self
            .agents
            .lock()
            .await
            .remove(agent_id)
            .ok_or_else(|| anyhow::anyhow!("agent '{agent_id}' not connected"))?;
        handle.shutdown().await
    }

    pub async fn shutdown_all(&self) {
        let handles: Vec<_> = self.agents.lock().await.drain().collect();
        for (_, handle) in handles {
            let _ = handle.shutdown().await;
        }
    }

    pub async fn agent_health(&self, agent_id: &str) -> Result<AgentStatus> {
        let agents = self.agents.lock().await;
        let handle = agents
            .get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("agent '{agent_id}' not connected"))?;

        if !handle.is_channel_alive() {
            return Ok(AgentStatus {
                agent_id: agent_id.to_string(),
                state: AgentState::Dead,
                consecutive_failures: handle.health.consecutive_failures(),
                last_activity_secs_ago: handle.health.last_activity().await.elapsed().as_secs(),
            });
        }

        Ok(handle.health_status().await)
    }

    pub async fn all_agent_health(&self) -> Vec<AgentStatus> {
        let agents = self.agents.lock().await;
        let mut statuses = Vec::with_capacity(agents.len());
        for handle in agents.values() {
            statuses.push(handle.health_status().await);
        }
        statuses
    }

    pub async fn prune_dead_agents(&self) -> Vec<String> {
        let mut agents = self.agents.lock().await;
        let mut dead = Vec::new();
        let mut to_remove = Vec::new();
        for (id, handle) in agents.iter() {
            let state = handle.health.state().await;
            if state == AgentState::Dead || !handle.is_channel_alive() {
                dead.push(id.clone());
                to_remove.push(id.clone());
            }
        }
        for id in &to_remove {
            if let Some(handle) = agents.remove(id) {
                let _ = handle.shutdown().await;
            }
        }
        dead
    }
}

async fn run_agent_connection(
    agent_id: String,
    distribution: &AgentDistribution,
    mut cmd_rx: mpsc::Receiver<AgentCommand>,
    collected_text: Arc<Mutex<Vec<String>>>,
) -> Result<(
    InitializeResponse,
    impl std::future::Future<Output = anyhow::Result<()>>,
)> {
    let SpawnedAgent {
        child: _child,
        stdin,
        stdout,
    } = spawn_agent(distribution).await?;

    let client = OrchestratorClient { collected_text };

    let (conn, io_task) =
        ClientSideConnection::new(client, stdin.compat_write(), stdout.compat(), |fut| {
            tokio::task::spawn_local(fut);
        });

    let io_task = tokio::task::spawn_local(async move { io_task.await.map_err(Into::into) });

    let init_req = InitializeRequest::new(ProtocolVersion::LATEST);
    let info = conn
        .initialize(init_req)
        .await
        .map_err(|e| anyhow::anyhow!("ACP initialize failed for '{}': {}", agent_id, e))?;

    tokio::task::spawn_local(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                AgentCommand::NewSession { req, reply } => {
                    let result = conn
                        .new_session(req)
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"));
                    let _ = reply.send(result);
                }
                AgentCommand::Prompt { req, reply } => {
                    let result = conn.prompt(req).await.map_err(|e| anyhow::anyhow!("{e}"));
                    let _ = reply.send(result);
                }
                AgentCommand::SetMode { req, reply } => {
                    let result = conn
                        .set_session_mode(req)
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"));
                    let _ = reply.send(result);
                }
                AgentCommand::Shutdown { reply } => {
                    let _ = reply.send(Ok(()));
                    break;
                }
            }
        }
    });

    Ok((info, async move { io_task.await? }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_manager_is_empty() {
        let mgr = AgentClientManager::default();
        assert!(mgr.list_agents().await.is_empty());
    }

    #[tokio::test]
    async fn prompt_nonexistent_agent_fails() {
        let mgr = AgentClientManager::new();
        let req = PromptRequest::new(SessionId::from("test"), vec![]);
        let result = mgr.prompt_agent("nope", req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn disconnect_nonexistent_agent_fails() {
        let mgr = AgentClientManager::new();
        let result = mgr.disconnect_agent("nope").await;
        assert!(result.is_err());
    }
}
