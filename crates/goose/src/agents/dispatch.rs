//! Dispatcher — bridges orchestrator routing decisions to execution backends.
//!
//! The Dispatcher trait abstracts over three execution strategies:
//! - **InProcess**: delegates to a local agent via the AgentPool
//! - **A2A**: delegates to a remote agent via the A2A HTTP client
//! - **ACP**: delegates to an external ACP agent process (future)
//!
//! The orchestrator produces a plan, and the dispatcher executes each sub-task
//! through the appropriate backend based on the DelegationStrategy.

use crate::agents::delegation::DelegationStrategy;
use crate::agents::orchestrator_agent::SubTask;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, instrument, warn};

/// Result of dispatching a single sub-task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResult {
    pub task_description: String,
    pub agent_name: String,
    pub strategy: String,
    pub output: String,
    pub status: DispatchStatus,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DispatchStatus {
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

/// Events emitted during dispatch for observability.
#[derive(Debug, Clone)]
pub enum DispatchEvent {
    Started {
        task_index: usize,
        agent_name: String,
        strategy: String,
    },
    Progress {
        task_index: usize,
        message: String,
    },
    Completed {
        task_index: usize,
        result: DispatchResult,
    },
    Failed {
        task_index: usize,
        error: String,
    },
}

/// The core dispatch trait — abstracts over execution backends.
#[async_trait]
pub trait Dispatcher: Send + Sync {
    /// Dispatch a single sub-task using the given strategy.
    async fn dispatch_one(
        &self,
        sub_task: &SubTask,
        strategy: &DelegationStrategy,
    ) -> Result<DispatchResult>;

    /// Dispatch multiple sub-tasks, potentially in parallel.
    /// Returns results in the same order as the input tasks.
    async fn dispatch_all(
        &self,
        tasks: &[(SubTask, DelegationStrategy)],
    ) -> Vec<Result<DispatchResult>> {
        let mut results = Vec::with_capacity(tasks.len());
        for (sub_task, strategy) in tasks {
            results.push(self.dispatch_one(sub_task, strategy).await);
        }
        results
    }
}

/// In-process dispatcher — executes sub-tasks via a local agent reply loop.
///
/// For compound requests, the orchestrator has already configured the agent
/// context via `apply_routing()`. This dispatcher sends the sub-task description
/// to the provider for a single-turn completion.
pub struct InProcessDispatcher {
    provider: Arc<dyn crate::providers::base::Provider>,
    session_id: String,
    event_tx: Option<broadcast::Sender<DispatchEvent>>,
}

impl InProcessDispatcher {
    pub fn new(provider: Arc<dyn crate::providers::base::Provider>, session_id: String) -> Self {
        Self {
            provider,
            session_id,
            event_tx: None,
        }
    }

    pub fn with_events(mut self, tx: broadcast::Sender<DispatchEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    fn emit(&self, event: DispatchEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event);
        }
    }
}

#[async_trait]
impl Dispatcher for InProcessDispatcher {
    #[instrument(skip(self), fields(agent = %sub_task.routing.agent_name, mode = %sub_task.routing.mode_slug))]
    async fn dispatch_one(
        &self,
        sub_task: &SubTask,
        strategy: &DelegationStrategy,
    ) -> Result<DispatchResult> {
        let start = std::time::Instant::now();
        let agent_name = sub_task.routing.agent_name.clone();

        debug!(
            strategy = %strategy,
            description = %sub_task.sub_task_description,
            "Dispatching in-process sub-task"
        );

        self.emit(DispatchEvent::Started {
            task_index: 0,
            agent_name: agent_name.clone(),
            strategy: strategy.to_string(),
        });

        let messages =
            vec![crate::conversation::message::Message::user()
                .with_text(&sub_task.sub_task_description)];

        let (response, _usage) = self
            .provider
            .complete(
                &self.session_id,
                "You are a helpful assistant executing a sub-task.",
                &messages,
                &[],
            )
            .await
            .map_err(|e| anyhow::anyhow!("Provider completion failed: {e}"))?;

        let output = response
            .content
            .iter()
            .filter_map(|c| match c {
                crate::conversation::message::MessageContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let duration_ms = start.elapsed().as_millis() as u64;

        let result = DispatchResult {
            task_description: sub_task.sub_task_description.clone(),
            agent_name: agent_name.clone(),
            strategy: strategy.to_string(),
            output,
            status: DispatchStatus::Completed,
            duration_ms,
        };

        info!(
            agent = %agent_name,
            duration_ms = duration_ms,
            "In-process sub-task completed"
        );

        self.emit(DispatchEvent::Completed {
            task_index: 0,
            result: result.clone(),
        });

        Ok(result)
    }
}

/// A2A dispatcher — executes sub-tasks via the A2A HTTP client.
pub struct A2ADispatcher {
    event_tx: Option<broadcast::Sender<DispatchEvent>>,
}

impl A2ADispatcher {
    pub fn new() -> Self {
        Self { event_tx: None }
    }

    pub fn with_events(mut self, tx: broadcast::Sender<DispatchEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    fn emit(&self, event: DispatchEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event);
        }
    }
}

impl Default for A2ADispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Dispatcher for A2ADispatcher {
    #[instrument(skip(self), fields(agent = %sub_task.routing.agent_name))]
    async fn dispatch_one(
        &self,
        sub_task: &SubTask,
        strategy: &DelegationStrategy,
    ) -> Result<DispatchResult> {
        let start = std::time::Instant::now();
        let agent_name = sub_task.routing.agent_name.clone();

        let url = match strategy {
            DelegationStrategy::RemoteA2AAgent { url } => url.clone(),
            other => anyhow::bail!("A2ADispatcher requires RemoteA2AAgent strategy, got {other}"),
        };

        debug!(
            url = %url,
            description = %sub_task.sub_task_description,
            "Dispatching A2A sub-task"
        );

        self.emit(DispatchEvent::Started {
            task_index: 0,
            agent_name: agent_name.clone(),
            strategy: strategy.to_string(),
        });

        // A2AClient requires an RPC endpoint (a JSON-RPC URL). The registry may store either:
        // - a base URL that serves /.well-known/agent-card.json, or
        // - a direct JSON-RPC endpoint.
        //
        // Prefer fetching the agent card (sets rpc_url), but if that fails, fall back to treating
        // the configured URL as the JSON-RPC endpoint.
        let mut client = a2a::client::A2AClient::new(&url);

        if client.fetch_agent_card().await.is_err() {
            let direct_card = a2a::types::agent_card::AgentCard {
                name: agent_name.clone(),
                description: "Remote A2A agent".to_string(),
                supported_interfaces: vec![a2a::types::agent_card::AgentInterface {
                    url: url.clone(),
                    protocol_binding: Some("JSONRPC".to_string()),
                    tenant: None,
                    protocol_version: None,
                }],
                provider: None,
                version: None,
                protocol_version: Some("1.0".to_string()),
                capabilities: None,
                security_schemes: serde_json::Value::Null,
                security: vec![],
                default_input_modes: vec!["text/plain".to_string()],
                default_output_modes: vec!["text/plain".to_string()],
                skills: vec![],
                documentation_url: None,
                icon_url: None,
                signatures: vec![],
            };

            client = client.with_agent_card(direct_card);
        }

        let message = a2a::types::core::Message {
            message_id: uuid::Uuid::new_v4().to_string(),
            context_id: None,
            task_id: None,
            role: a2a::types::core::Role::User,
            parts: vec![a2a::types::core::Part::text(&sub_task.sub_task_description)],
            metadata: None,
            extensions: vec![],
            reference_task_ids: vec![],
        };

        let request = a2a::types::requests::SendMessageRequest {
            message,
            configuration: None,
            metadata: None,
        };

        match client.send_message(request).await {
            Ok(response) => {
                let output = match response {
                    a2a::types::responses::SendMessageResponse::Task(task) => {
                        extract_task_output(&task)
                    }
                    a2a::types::responses::SendMessageResponse::Message(msg) => {
                        extract_message_output(&msg)
                    }
                };

                let duration_ms = start.elapsed().as_millis() as u64;

                let result = DispatchResult {
                    task_description: sub_task.sub_task_description.clone(),
                    agent_name: agent_name.clone(),
                    strategy: strategy.to_string(),
                    output,
                    status: DispatchStatus::Completed,
                    duration_ms,
                };

                info!(
                    agent = %agent_name,
                    url = %url,
                    duration_ms = duration_ms,
                    "A2A sub-task completed"
                );

                self.emit(DispatchEvent::Completed {
                    task_index: 0,
                    result: result.clone(),
                });

                Ok(result)
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                warn!(
                    agent = %agent_name,
                    url = %url,
                    error = %e,
                    "A2A sub-task failed"
                );

                self.emit(DispatchEvent::Failed {
                    task_index: 0,
                    error: e.to_string(),
                });

                Ok(DispatchResult {
                    task_description: sub_task.sub_task_description.clone(),
                    agent_name,
                    strategy: strategy.to_string(),
                    output: String::new(),
                    status: DispatchStatus::Failed,
                    duration_ms,
                })
            }
        }
    }
}

/// Composite dispatcher — routes to the correct backend based on DelegationStrategy.
pub struct CompositeDispatcher {
    in_process: InProcessDispatcher,
    a2a: A2ADispatcher,
    event_tx: Option<broadcast::Sender<DispatchEvent>>,
}

impl CompositeDispatcher {
    pub fn new(provider: Arc<dyn crate::providers::base::Provider>, session_id: String) -> Self {
        Self {
            in_process: InProcessDispatcher::new(provider, session_id),
            a2a: A2ADispatcher::new(),
            event_tx: None,
        }
    }

    pub fn with_events(mut self, tx: broadcast::Sender<DispatchEvent>) -> Self {
        self.in_process = self.in_process.with_events(tx.clone());
        self.a2a = self.a2a.with_events(tx.clone());
        self.event_tx = Some(tx);
        self
    }
}

#[async_trait]
impl Dispatcher for CompositeDispatcher {
    async fn dispatch_one(
        &self,
        sub_task: &SubTask,
        strategy: &DelegationStrategy,
    ) -> Result<DispatchResult> {
        match strategy {
            DelegationStrategy::InProcessSpecialist { .. } => {
                self.in_process.dispatch_one(sub_task, strategy).await
            }
            DelegationStrategy::RemoteA2AAgent { .. } => {
                self.a2a.dispatch_one(sub_task, strategy).await
            }
            DelegationStrategy::ExternalAcpAgent { .. } => {
                // ACP dispatch is not yet implemented; fall back to in-process
                warn!(
                    agent = %sub_task.routing.agent_name,
                    "ACP dispatch not yet implemented, falling back to in-process"
                );
                self.in_process.dispatch_one(sub_task, strategy).await
            }
        }
    }

    async fn dispatch_all(
        &self,
        tasks: &[(SubTask, DelegationStrategy)],
    ) -> Vec<Result<DispatchResult>> {
        if tasks.len() == 1 {
            return vec![self.dispatch_one(&tasks[0].0, &tasks[0].1).await];
        }

        // Fan-out: dispatch all sub-tasks concurrently
        info!(task_count = tasks.len(), "Fan-out compound dispatch");

        let futures: Vec<_> = tasks
            .iter()
            .map(|(sub_task, strategy)| self.dispatch_one(sub_task, strategy))
            .collect();

        futures::future::join_all(futures).await
    }
}

fn extract_task_output(task: &a2a::types::core::Task) -> String {
    task.artifacts
        .iter()
        .flat_map(|a| &a.parts)
        .filter_map(|p| match &p.content {
            a2a::types::core::PartContent::Text { text } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_message_output(msg: &a2a::types::core::Message) -> String {
    msg.parts
        .iter()
        .filter_map(|p| match &p.content {
            a2a::types::core::PartContent::Text { text } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Dispatcher that executes sub-tasks via `Agent::reply()` for full multi-turn
/// tool-using execution. Unlike `InProcessDispatcher` (which uses `Provider::complete()`
/// for single-turn LLM calls), this gives sub-tasks the complete agent experience.
#[derive(Clone)]
pub struct AgentReplyDispatcher {
    agent: std::sync::Arc<crate::agents::Agent>,
    session_id: String,
}

impl AgentReplyDispatcher {
    pub fn new(agent: std::sync::Arc<crate::agents::Agent>, session_id: String) -> Self {
        Self { agent, session_id }
    }

    pub async fn dispatch_sub_task(
        &self,
        sub_task: &SubTask,
        task_index: usize,
        cancel_token: Option<tokio_util::sync::CancellationToken>,
    ) -> DispatchResult {
        let start = std::time::Instant::now();
        let user_message =
            crate::conversation::message::Message::user().with_text(&sub_task.sub_task_description);
        let sub_config = crate::agents::types::SessionConfig {
            id: format!("{}-sub-{}", self.session_id, task_index),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        let output = match self
            .agent
            .reply(user_message, sub_config, cancel_token)
            .await
        {
            Ok(mut stream) => {
                let mut collected = String::new();
                while let Some(event) = futures::StreamExt::next(&mut stream).await {
                    match event {
                        Ok(crate::agents::AgentEvent::Message(msg)) => {
                            if msg.role == rmcp::model::Role::Assistant {
                                for content in &msg.content {
                                    if let crate::conversation::message::MessageContent::Text(t) =
                                        content
                                    {
                                        collected.push_str(&t.text);
                                    }
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::warn!(task_index, error = %e, "Sub-task error");
                            collected = format!("Error: {e}");
                            break;
                        }
                    }
                }
                collected
            }
            Err(e) => {
                return DispatchResult {
                    task_description: sub_task.sub_task_description.clone(),
                    agent_name: sub_task.routing.agent_name.clone(),
                    strategy: "AgentReply".to_string(),
                    output: format!("Failed: {e}"),
                    status: DispatchStatus::Failed,
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }
        };

        DispatchResult {
            task_description: sub_task.sub_task_description.clone(),
            agent_name: sub_task.routing.agent_name.clone(),
            strategy: "AgentReply".to_string(),
            output,
            status: DispatchStatus::Completed,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}

/// Dispatch a single compound sub-task to either in-process `AgentReplyDispatcher` or remote A2A.
async fn dispatch_compound_one(
    reply_dispatcher: &AgentReplyDispatcher,
    sub_task: &SubTask,
    a2a_url: &Option<String>,
    task_index: usize,
    cancel_token: Option<tokio_util::sync::CancellationToken>,
) -> DispatchResult {
    if let Some(url) = a2a_url {
        let a2a = A2ADispatcher::new();
        let strategy = DelegationStrategy::RemoteA2AAgent { url: url.clone() };
        a2a.dispatch_one(sub_task, &strategy)
            .await
            .unwrap_or_else(|e| DispatchResult {
                task_description: sub_task.sub_task_description.clone(),
                agent_name: sub_task.routing.agent_name.clone(),
                strategy: "RemoteA2A".to_string(),
                output: format!("A2A Error: {e}"),
                status: DispatchStatus::Failed,
                duration_ms: 0,
            })
    } else {
        reply_dispatcher
            .dispatch_sub_task(sub_task, task_index, cancel_token)
            .await
    }
}

/// Sequential compound dispatcher: `None` URL → in-process, `Some(url)` → remote A2A.
pub async fn dispatch_compound_sequential(
    reply_dispatcher: &AgentReplyDispatcher,
    tasks: &[(SubTask, Option<String>)],
    cancel_token: Option<tokio_util::sync::CancellationToken>,
) -> Vec<DispatchResult> {
    let mut results = Vec::with_capacity(tasks.len());

    for (i, (sub_task, a2a_url)) in tasks.iter().enumerate() {
        tracing::info!(
            task_index = i,
            agent = %sub_task.routing.agent_name,
            remote = a2a_url.is_some(),
            "Dispatching compound sub-task (sequential)"
        );
        results.push(
            dispatch_compound_one(reply_dispatcher, sub_task, a2a_url, i, cancel_token.clone())
                .await,
        );
    }

    results
}

/// Parallel DAG scheduler for compound tasks.
///
/// Tasks are executed when all `depends_on` task_ids have completed. Independent tasks can run in
/// parallel up to `max_concurrency`. Results are returned in the same order as `tasks`.
pub async fn dispatch_compound_dag(
    reply_dispatcher: &AgentReplyDispatcher,
    tasks: &[(SubTask, Option<String>)],
    max_concurrency: usize,
    cancel_token: Option<tokio_util::sync::CancellationToken>,
) -> Vec<DispatchResult> {
    let max_concurrency = std::cmp::max(1, max_concurrency);
    if tasks.len() <= 1 || max_concurrency == 1 {
        return dispatch_compound_sequential(reply_dispatcher, tasks, cancel_token).await;
    }

    let mut id_to_index = std::collections::HashMap::new();
    for (i, (task, _)) in tasks.iter().enumerate() {
        id_to_index.insert(task.task_id.clone(), i);
    }

    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); tasks.len()];
    let mut indegree: Vec<usize> = vec![0; tasks.len()];

    for (i, (task, _)) in tasks.iter().enumerate() {
        for dep in &task.depends_on {
            if let Some(&dep_idx) = id_to_index.get(dep) {
                indegree[i] += 1;
                dependents[dep_idx].push(i);
            }
        }
    }

    let mut ready = std::collections::BTreeSet::new();
    for (i, (task, _)) in tasks.iter().enumerate() {
        if indegree[i] == 0 {
            ready.insert((task.task_id.clone(), i));
        }
    }

    let mut results: Vec<Option<DispatchResult>> = vec![None; tasks.len()];
    let mut in_flight = futures::stream::FuturesUnordered::new();
    let mut remaining = tasks.len();

    while remaining > 0 {
        if let Some(token) = &cancel_token {
            if token.is_cancelled() {
                break;
            }
        }

        while in_flight.len() < max_concurrency {
            let Some((_, idx)) = ready.pop_first() else {
                break;
            };
            let (sub_task, a2a_url) = &tasks[idx];

            tracing::info!(
                task_index = idx,
                task_id = %sub_task.task_id,
                agent = %sub_task.routing.agent_name,
                remote = a2a_url.is_some(),
                "Dispatching compound sub-task (dag)"
            );

            let reply_dispatcher = reply_dispatcher.clone();
            let sub_task = sub_task.clone();
            let a2a_url = a2a_url.clone();
            let cancel_token = cancel_token.clone();

            in_flight.push(async move {
                let result = dispatch_compound_one(
                    &reply_dispatcher,
                    &sub_task,
                    &a2a_url,
                    idx,
                    cancel_token,
                )
                .await;
                (idx, result)
            });
        }

        if let Some((idx, result)) = futures::StreamExt::next(&mut in_flight).await {
            results[idx] = Some(result);
            remaining -= 1;

            for &dep_idx in &dependents[idx] {
                indegree[dep_idx] = indegree[dep_idx].saturating_sub(1);
                if indegree[dep_idx] == 0 {
                    let task_id = tasks[dep_idx].0.task_id.clone();
                    ready.insert((task_id, dep_idx));
                }
            }
        } else {
            // Deadlock (cycle or missing deps); fall back to sequential execution of remaining tasks.
            for (i, (sub_task, a2a_url)) in tasks.iter().enumerate() {
                if results[i].is_some() {
                    continue;
                }
                results[i] = Some(
                    dispatch_compound_one(
                        reply_dispatcher,
                        sub_task,
                        a2a_url,
                        i,
                        cancel_token.clone(),
                    )
                    .await,
                );
                remaining = remaining.saturating_sub(1);
            }
        }
    }

    results
        .into_iter()
        .map(|r| {
            r.unwrap_or(DispatchResult {
                task_description: "cancelled".to_string(),
                agent_name: "unknown".to_string(),
                strategy: "cancelled".to_string(),
                output: String::new(),
                status: DispatchStatus::Cancelled,
                duration_ms: 0,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::intent_router::RoutingDecision;

    fn test_sub_task(description: &str, agent: &str) -> SubTask {
        SubTask {
            task_id: "task-1".to_string(),
            depends_on: Vec::new(),
            routing: RoutingDecision {
                agent_name: agent.to_string(),
                mode_slug: "default".to_string(),
                confidence: 0.9,
                reasoning: "test".to_string(),
            },
            sub_task_description: description.to_string(),
        }
    }

    #[test]
    fn dispatch_result_serialization() {
        let result = DispatchResult {
            task_description: "Fix the bug".to_string(),
            agent_name: "developer".to_string(),
            strategy: "InProcessSpecialist(simple)".to_string(),
            output: "Fixed!".to_string(),
            status: DispatchStatus::Completed,
            duration_ms: 1234,
        };

        let json = serde_json::to_string(&result).unwrap();
        let roundtripped: DispatchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.agent_name, "developer");
        assert_eq!(roundtripped.status, DispatchStatus::Completed);
    }

    #[test]
    fn a2a_dispatcher_rejects_non_a2a_strategy() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let dispatcher = A2ADispatcher::new();
        let sub_task = test_sub_task("test", "developer");
        let strategy = DelegationStrategy::choose(None, None, false, false, false);

        let result = rt.block_on(dispatcher.dispatch_one(&sub_task, &strategy));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("A2ADispatcher requires RemoteA2AAgent"));
    }

    #[test]
    fn dispatch_event_variants() {
        let event = DispatchEvent::Started {
            task_index: 0,
            agent_name: "dev".to_string(),
            strategy: "InProcess".to_string(),
        };
        // Ensure the event is Debug-printable
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Started"));
    }
}
