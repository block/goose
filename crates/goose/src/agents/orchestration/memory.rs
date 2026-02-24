use crate::agents::dispatch::{DispatchResult, DispatchStatus};
use crate::agents::orchestrator_agent::{OrchestratorPlan, SubTask};
use crate::session::SessionManager;
use anyhow::Result;
use serde::{Deserialize, Serialize};

const EXTENSION_NAME: &str = "orchestration";
const EXTENSION_VERSION: &str = "v1";

const MAX_RUNS: usize = 50;
const MAX_ROOT_REQUEST_CHARS: usize = 8_000;
const MAX_TASK_OUTPUT_CHARS: usize = 20_000;
const MAX_AGGREGATED_OUTPUT_CHARS: usize = 30_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrchestrationTaskStatus {
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl From<DispatchStatus> for OrchestrationTaskStatus {
    fn from(value: DispatchStatus) -> Self {
        match value {
            DispatchStatus::Completed => Self::Completed,
            DispatchStatus::Failed => Self::Failed,
            DispatchStatus::Cancelled => Self::Cancelled,
            DispatchStatus::TimedOut => Self::TimedOut,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationTaskRecord {
    pub task_id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub agent_name: String,
    pub mode_slug: String,
    pub description: String,
    pub strategy: String,
    pub status: OrchestrationTaskStatus,
    pub duration_ms: u64,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRunRecord {
    pub run_id: String,
    pub created_at_ms: i64,
    pub max_concurrency: usize,
    pub root_request: String,
    pub aggregated_output: String,
    pub tasks: Vec<OrchestrationTaskRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrchestrationStateV1 {
    #[serde(default)]
    pub runs: Vec<OrchestrationRunRecord>,
}

fn truncate(mut s: String, max_len: usize) -> String {
    if s.len() <= max_len {
        return s;
    }
    s.truncate(max_len);
    s.push('…');
    s
}

fn tasks_to_records(tasks: &[SubTask], results: &[DispatchResult]) -> Vec<OrchestrationTaskRecord> {
    tasks
        .iter()
        .zip(results.iter())
        .map(|(task, result)| OrchestrationTaskRecord {
            task_id: task.task_id.clone(),
            depends_on: task.depends_on.clone(),
            agent_name: task.routing.agent_name.clone(),
            mode_slug: task.routing.mode_slug.clone(),
            description: truncate(task.sub_task_description.clone(), MAX_ROOT_REQUEST_CHARS),
            strategy: result.strategy.clone(),
            status: result.status.clone().into(),
            duration_ms: result.duration_ms,
            output: truncate(result.output.clone(), MAX_TASK_OUTPUT_CHARS),
        })
        .collect()
}

/// Persist a summary of a compound orchestration run into the session's extension_data.
///
/// This is best-effort storage for observability/replay and downstream KG ingestion.
/// It must never fail the overall user request.
pub async fn persist_orchestration_run(
    session_manager: &SessionManager,
    session_id: &str,
    root_request: &str,
    plan: &OrchestratorPlan,
    results: &[DispatchResult],
    max_concurrency: usize,
    aggregated_output: &str,
) -> Result<()> {
    if !plan.is_compound || plan.tasks.len() <= 1 {
        return Ok(());
    }

    let mut session = session_manager.get_session(session_id, false).await?;

    let existing: OrchestrationStateV1 = session
        .extension_data
        .get_extension_state(EXTENSION_NAME, EXTENSION_VERSION)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let mut state = existing;

    let run = OrchestrationRunRecord {
        run_id: uuid::Uuid::new_v4().to_string(),
        created_at_ms: chrono::Utc::now().timestamp_millis(),
        max_concurrency,
        root_request: truncate(root_request.to_string(), MAX_ROOT_REQUEST_CHARS),
        aggregated_output: truncate(aggregated_output.to_string(), MAX_AGGREGATED_OUTPUT_CHARS),
        tasks: tasks_to_records(&plan.tasks, results),
    };

    state.runs.push(run);
    if state.runs.len() > MAX_RUNS {
        let start = state.runs.len() - MAX_RUNS;
        state.runs = state.runs.split_off(start);
    }

    session.extension_data.set_extension_state(
        EXTENSION_NAME,
        EXTENSION_VERSION,
        serde_json::to_value(&state)?,
    );

    session_manager
        .update(session_id)
        .extension_data(session.extension_data)
        .apply()
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::intent_router::RoutingDecision;
    use crate::session::SessionType;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_plan() -> OrchestratorPlan {
        OrchestratorPlan {
            is_compound: true,
            tasks: vec![
                SubTask {
                    task_id: "a".into(),
                    depends_on: vec![],
                    routing: RoutingDecision {
                        agent_name: "Developer Agent".into(),
                        mode_slug: "write".into(),
                        confidence: 0.9,
                        reasoning: "test".into(),
                    },
                    sub_task_description: "Do A".into(),
                },
                SubTask {
                    task_id: "b".into(),
                    depends_on: vec!["a".into()],
                    routing: RoutingDecision {
                        agent_name: "QA Agent".into(),
                        mode_slug: "review".into(),
                        confidence: 0.8,
                        reasoning: "test".into(),
                    },
                    sub_task_description: "Do B".into(),
                },
            ],
        }
    }

    fn make_results() -> Vec<DispatchResult> {
        vec![
            DispatchResult {
                task_description: "Do A".into(),
                agent_name: "Developer Agent".into(),
                strategy: "AgentReply".into(),
                output: "A done".into(),
                status: DispatchStatus::Completed,
                duration_ms: 10,
            },
            DispatchResult {
                task_description: "Do B".into(),
                agent_name: "QA Agent".into(),
                strategy: "AgentReply".into(),
                output: "B done".into(),
                status: DispatchStatus::Failed,
                duration_ms: 20,
            },
        ]
    }

    #[tokio::test]
    async fn persist_orchestration_run_writes_extension_data() {
        let temp_dir = TempDir::new().unwrap();
        let sm = SessionManager::new(temp_dir.path().join("data"));
        let session = sm
            .create_session(PathBuf::from("/tmp"), "test".into(), SessionType::User)
            .await
            .unwrap();

        let plan = make_plan();
        let results = make_results();

        persist_orchestration_run(
            &sm,
            &session.id,
            "root request",
            &plan,
            &results,
            3,
            "aggregated",
        )
        .await
        .unwrap();

        let stored = sm.get_session(&session.id, false).await.unwrap();
        let value = stored
            .extension_data
            .get_extension_state(EXTENSION_NAME, EXTENSION_VERSION)
            .cloned()
            .expect("orchestration state should be present");

        let state: OrchestrationStateV1 = serde_json::from_value(value).unwrap();
        assert_eq!(state.runs.len(), 1);
        assert_eq!(state.runs[0].tasks.len(), 2);
        assert_eq!(state.runs[0].tasks[0].task_id, "a");
        assert_eq!(state.runs[0].tasks[1].depends_on, vec!["a".to_string()]);
    }
}
