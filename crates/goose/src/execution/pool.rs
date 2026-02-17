//! Agent pool for managed parallel execution of multiple agent instances.
//!
//! Provides lifecycle management (spawn, status, cancel, join) for concurrent
//! agent tasks, decoupled from any specific caller (SummonExtension, server routes, A2A).

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::agents::{Agent, AgentConfig, AgentEvent, SessionConfig};
use crate::config::permission::PermissionManager;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::session::session_manager::{SessionManager, SessionType};
use crate::session_context;
use rmcp::model::Role;

use futures::StreamExt;

/// Status of an agent instance in the pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for InstanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Metadata and handle for a running agent instance.
pub struct AgentInstance {
    pub id: String,
    pub persona: String,
    pub provider_name: String,
    pub model_name: String,
    pub started_at: Instant,
    pub turns: Arc<AtomicU32>,
    pub last_activity: Arc<AtomicU64>,
    handle: JoinHandle<Result<(Conversation, Option<String>)>>,
    cancellation_token: CancellationToken,
}

/// Result of a completed agent instance.
#[derive(Debug, Clone)]
pub struct AgentResult {
    pub id: String,
    pub persona: String,
    pub provider_name: String,
    pub model_name: String,
    pub status: InstanceStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub turns_taken: u32,
    pub duration: Duration,
}

/// Configuration for spawning an agent instance.
pub struct SpawnConfig {
    pub persona: String,
    pub instructions: String,
    pub prompt: String,
    pub working_dir: std::path::PathBuf,
    pub provider: Arc<dyn Provider>,
    pub extensions: Vec<crate::agents::extension::ExtensionConfig>,
    pub max_turns: Option<usize>,
    pub session_manager: Arc<SessionManager>,
}

/// Pool managing multiple concurrent agent instances with lifecycle control.
pub struct AgentPool {
    instances: Mutex<HashMap<String, AgentInstance>>,
    results: Mutex<Vec<AgentResult>>,
    max_instances: usize,
}

fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl AgentPool {
    pub fn new(max_instances: usize) -> Self {
        Self {
            instances: Mutex::new(HashMap::new()),
            results: Mutex::new(Vec::new()),
            max_instances,
        }
    }

    /// Number of currently running instances.
    pub async fn running_count(&self) -> usize {
        self.instances.lock().await.len()
    }

    /// Spawn a new agent instance in the pool.
    pub async fn spawn(&self, config: SpawnConfig) -> Result<String> {
        let mut instances = self.instances.lock().await;
        if instances.len() >= self.max_instances {
            return Err(anyhow!(
                "Pool at capacity ({}/{}). Wait for an instance to complete or cancel one.",
                instances.len(),
                self.max_instances,
            ));
        }

        let provider_name = config.provider.get_name().to_string();
        let model_name = config.provider.get_model_config().model_name.clone();

        let session = config
            .session_manager
            .create_session(
                config.working_dir.clone(),
                format!("pool:{}", config.persona),
                SessionType::Specialist,
            )
            .await?;

        let instance_id = session.id.clone();
        let token = CancellationToken::new();
        let turns = Arc::new(AtomicU32::new(0));
        let last_activity = Arc::new(AtomicU64::new(epoch_millis()));

        let turns_clone = Arc::clone(&turns);
        let last_activity_clone = Arc::clone(&last_activity);
        let token_clone = token.clone();
        let session_id = instance_id.clone();

        let persona = config.persona.clone();

        let handle = tokio::spawn(async move {
            run_pooled_agent(
                config,
                session_id,
                token_clone,
                turns_clone,
                last_activity_clone,
            )
            .await
        });

        let instance = AgentInstance {
            id: instance_id.clone(),
            persona,
            provider_name,
            model_name,
            started_at: Instant::now(),
            turns,
            last_activity,
            handle,
            cancellation_token: token,
        };

        instances.insert(instance_id.clone(), instance);
        Ok(instance_id)
    }

    /// Get status of a specific instance.
    pub async fn status(&self, id: &str) -> Option<InstanceSnapshot> {
        let instances = self.instances.lock().await;
        instances.get(id).map(|inst| InstanceSnapshot {
            id: inst.id.clone(),
            persona: inst.persona.clone(),
            provider_name: inst.provider_name.clone(),
            model_name: inst.model_name.clone(),
            status: if inst.handle.is_finished() {
                InstanceStatus::Completed
            } else {
                InstanceStatus::Running
            },
            turns: inst.turns.load(Ordering::Relaxed),
            elapsed: inst.started_at.elapsed(),
            last_activity_ms: inst.last_activity.load(Ordering::Relaxed),
        })
    }

    /// Get snapshots of all instances (running and recently completed).
    pub async fn status_all(&self) -> Vec<InstanceSnapshot> {
        let instances = self.instances.lock().await;
        instances
            .values()
            .map(|inst| InstanceSnapshot {
                id: inst.id.clone(),
                persona: inst.persona.clone(),
                provider_name: inst.provider_name.clone(),
                model_name: inst.model_name.clone(),
                status: if inst.handle.is_finished() {
                    InstanceStatus::Completed
                } else {
                    InstanceStatus::Running
                },
                turns: inst.turns.load(Ordering::Relaxed),
                elapsed: inst.started_at.elapsed(),
                last_activity_ms: inst.last_activity.load(Ordering::Relaxed),
            })
            .collect()
    }

    /// Cancel a specific instance.
    pub async fn cancel(&self, id: &str) -> Result<()> {
        let instances = self.instances.lock().await;
        let inst = instances
            .get(id)
            .ok_or_else(|| anyhow!("Instance '{}' not found", id))?;
        inst.cancellation_token.cancel();
        Ok(())
    }

    /// Cancel all running instances.
    pub async fn cancel_all(&self) {
        let instances = self.instances.lock().await;
        for inst in instances.values() {
            inst.cancellation_token.cancel();
        }
    }

    /// Join a specific instance, waiting for completion and returning its result.
    /// Removes the instance from the pool.
    pub async fn join(&self, id: &str) -> Result<AgentResult> {
        let instance = {
            let mut instances = self.instances.lock().await;
            instances
                .remove(id)
                .ok_or_else(|| anyhow!("Instance '{}' not found", id))?
        };

        let duration = instance.started_at.elapsed();
        let turns_taken = instance.turns.load(Ordering::Relaxed);
        let was_cancelled = instance.cancellation_token.is_cancelled();

        let result = match instance.handle.await {
            Ok(Ok((_conversation, final_output))) => AgentResult {
                id: instance.id,
                persona: instance.persona,
                provider_name: instance.provider_name,
                model_name: instance.model_name,
                status: InstanceStatus::Completed,
                output: final_output,
                error: None,
                turns_taken,
                duration,
            },
            Ok(Err(e)) => AgentResult {
                id: instance.id,
                persona: instance.persona,
                provider_name: instance.provider_name,
                model_name: instance.model_name,
                status: if was_cancelled {
                    InstanceStatus::Cancelled
                } else {
                    InstanceStatus::Failed
                },
                output: None,
                error: Some(e.to_string()),
                turns_taken,
                duration,
            },
            Err(e) => AgentResult {
                id: instance.id,
                persona: instance.persona,
                provider_name: instance.provider_name,
                model_name: instance.model_name,
                status: InstanceStatus::Failed,
                output: None,
                error: Some(format!("Task panicked: {}", e)),
                turns_taken,
                duration,
            },
        };

        self.results.lock().await.push(result.clone());
        Ok(result)
    }

    /// Join all instances, waiting for all to complete. Returns results in completion order.
    pub async fn join_all(&self) -> Vec<AgentResult> {
        let all_instances: Vec<AgentInstance> = {
            let mut instances = self.instances.lock().await;
            instances.drain().map(|(_, v)| v).collect()
        };

        let mut results = Vec::with_capacity(all_instances.len());
        for instance in all_instances {
            let duration = instance.started_at.elapsed();
            let turns_taken = instance.turns.load(Ordering::Relaxed);
            let was_cancelled = instance.cancellation_token.is_cancelled();

            let result = match instance.handle.await {
                Ok(Ok((_conversation, final_output))) => AgentResult {
                    id: instance.id,
                    persona: instance.persona,
                    provider_name: instance.provider_name,
                    model_name: instance.model_name,
                    status: InstanceStatus::Completed,
                    output: final_output,
                    error: None,
                    turns_taken,
                    duration,
                },
                Ok(Err(e)) => AgentResult {
                    id: instance.id,
                    persona: instance.persona,
                    provider_name: instance.provider_name,
                    model_name: instance.model_name,
                    status: if was_cancelled {
                        InstanceStatus::Cancelled
                    } else {
                        InstanceStatus::Failed
                    },
                    output: None,
                    error: Some(e.to_string()),
                    turns_taken,
                    duration,
                },
                Err(e) => AgentResult {
                    id: instance.id,
                    persona: instance.persona,
                    provider_name: instance.provider_name,
                    model_name: instance.model_name,
                    status: InstanceStatus::Failed,
                    output: None,
                    error: Some(format!("Task panicked: {}", e)),
                    turns_taken,
                    duration,
                },
            };

            results.push(result.clone());
            self.results.lock().await.push(result);
        }

        results
    }

    /// Collect results of finished instances without blocking.
    /// Removes completed instances from the pool and returns their results.
    pub async fn collect_finished(&self) -> Vec<AgentResult> {
        let finished_ids: Vec<String> = {
            let instances = self.instances.lock().await;
            instances
                .iter()
                .filter(|(_, inst)| inst.handle.is_finished())
                .map(|(id, _)| id.clone())
                .collect()
        };

        let mut results = Vec::new();
        for id in finished_ids {
            if let Ok(result) = self.join(&id).await {
                results.push(result);
            }
        }
        results
    }

    /// Get all completed results (historical).
    pub async fn completed_results(&self) -> Vec<AgentResult> {
        self.results.lock().await.clone()
    }
}

/// Read-only snapshot of an instance's state.
#[derive(Debug, Clone)]
pub struct InstanceSnapshot {
    pub id: String,
    pub persona: String,
    pub provider_name: String,
    pub model_name: String,
    pub status: InstanceStatus,
    pub turns: u32,
    pub elapsed: Duration,
    pub last_activity_ms: u64,
}

/// Run a pooled agent to completion. Follows the same pattern as specialist_handler
/// but decoupled from SummonExtension.
async fn run_pooled_agent(
    config: SpawnConfig,
    session_id: String,
    cancellation_token: CancellationToken,
    turns: Arc<AtomicU32>,
    last_activity: Arc<AtomicU64>,
) -> Result<(Conversation, Option<String>)> {
    let agent_config = AgentConfig::new(
        config.session_manager.clone(),
        PermissionManager::instance(),
        None,
        crate::config::GooseMode::Auto,
        true,
    );

    let agent = Arc::new(Agent::with_config(agent_config));

    agent
        .update_provider(config.provider.clone(), &session_id)
        .await
        .map_err(|e| anyhow!("Failed to set provider: {}", e))?;

    for ext in &config.extensions {
        if let Err(e) = agent.add_extension(ext.clone(), &session_id).await {
            tracing::debug!("Failed to add extension '{}': {}", ext.name(), e);
        }
    }

    if !config.instructions.is_empty() {
        agent.override_system_prompt(config.instructions).await;
    }

    let user_message = Message::user().with_text(config.prompt);
    let mut conversation = Conversation::new_unvalidated(vec![user_message.clone()]);

    let session_config = SessionConfig {
        id: session_id.clone(),
        schedule_id: None,
        max_turns: config.max_turns.map(|v| v as u32),
        retry_config: None,
    };

    let mut stream = session_context::with_session_id(Some(session_id.clone()), async {
        agent
            .reply(user_message, session_config, Some(cancellation_token))
            .await
    })
    .await
    .map_err(|e| anyhow!("Failed to get reply: {}", e))?;

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(AgentEvent::Message(msg)) => {
                turns.fetch_add(1, Ordering::Relaxed);
                last_activity.store(epoch_millis(), Ordering::Relaxed);
                conversation.push(msg);
            }
            Ok(AgentEvent::HistoryReplaced(updated)) => {
                conversation = updated;
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Error from pooled agent: {}", e);
                break;
            }
        }
    }

    let final_output = conversation
        .messages()
        .iter()
        .rev()
        .find(|m| m.role == Role::Assistant)
        .map(|m| {
            m.content
                .iter()
                .filter_map(|c| match c {
                    MessageContent::Text(t) => Some(t.text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        });

    Ok((conversation, final_output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_status_display() {
        assert_eq!(InstanceStatus::Running.to_string(), "running");
        assert_eq!(InstanceStatus::Completed.to_string(), "completed");
        assert_eq!(InstanceStatus::Failed.to_string(), "failed");
        assert_eq!(InstanceStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_pool_creation() {
        let pool = AgentPool::new(5);
        assert_eq!(pool.max_instances, 5);
    }

    #[tokio::test]
    async fn test_pool_capacity() {
        let pool = AgentPool::new(3);
        assert_eq!(pool.running_count().await, 0);
    }

    #[tokio::test]
    async fn test_cancel_nonexistent() {
        let pool = AgentPool::new(3);
        let result = pool.cancel("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_join_nonexistent() {
        let pool = AgentPool::new(3);
        let result = pool.join("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_status_empty_pool() {
        let pool = AgentPool::new(3);
        assert!(pool.status("anything").await.is_none());
        assert!(pool.status_all().await.is_empty());
    }

    #[tokio::test]
    async fn test_collect_finished_empty() {
        let pool = AgentPool::new(3);
        assert!(pool.collect_finished().await.is_empty());
    }

    #[tokio::test]
    async fn test_join_all_empty() {
        let pool = AgentPool::new(3);
        assert!(pool.join_all().await.is_empty());
    }

    #[tokio::test]
    async fn test_completed_results_empty() {
        let pool = AgentPool::new(3);
        assert!(pool.completed_results().await.is_empty());
    }
}
