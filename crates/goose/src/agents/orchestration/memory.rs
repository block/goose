use crate::agents::dispatch::{DispatchResult, DispatchStatus};
use crate::agents::extension_manager::ExtensionManager;
use crate::agents::orchestrator_agent::{OrchestratorPlan, SubTask};
use crate::session::SessionManager;
use anyhow::Result;
use rmcp::model::{CallToolRequestParams, JsonObject};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use std::borrow::Cow;
use std::path::Path;

const EXTENSION_NAME: &str = "orchestration";
const EXTENSION_VERSION: &str = "v1";

const KG_EXTENSION_NAME: &str = "knowledge_graph_memory";
const KG_TOOL_CREATE_ENTITIES: &str = "create_entities";
const KG_TOOL_CREATE_RELATIONS: &str = "create_relations";
const KG_TOOL_ADD_OBSERVATIONS: &str = "add_observations";

const MAX_RUNS: usize = 50;
const MAX_ROOT_REQUEST_CHARS: usize = 8_000;
const MAX_TASK_OUTPUT_CHARS: usize = 20_000;
const MAX_AGGREGATED_OUTPUT_CHARS: usize = 30_000;

const MAX_KG_OBSERVATION_CHARS: usize = 4_000;

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

fn kg_entity_name(run_id: &str, task_id: Option<&str>) -> String {
    match task_id {
        Some(task_id) => format!("orchestration/task/{run_id}/{task_id}"),
        None => format!("orchestration/run/{run_id}"),
    }
}

fn truncate_observation(mut s: String) -> String {
    if s.len() <= MAX_KG_OBSERVATION_CHARS {
        return s;
    }
    s.truncate(MAX_KG_OBSERVATION_CHARS);
    s.push('…');
    s
}

#[derive(Debug, Clone, Serialize)]
struct KgEntity {
    name: String,
    #[serde(rename = "entityType")]
    entity_type: String,
    observations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct KgRelation {
    from: String,
    to: String,
    #[serde(rename = "relationType")]
    relation_type: String,
}

#[derive(Debug, Clone, Serialize)]
struct KgObservation {
    #[serde(rename = "entityName")]
    entity_name: String,
    contents: Vec<String>,
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
) -> Result<Option<OrchestrationRunRecord>> {
    if !plan.is_compound || plan.tasks.len() <= 1 {
        return Ok(None);
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

    state.runs.push(run.clone());
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

    Ok(Some(run))
}

/// Attempt to ingest an orchestration run into the Knowledge Graph Memory extension.
///
/// This function is best-effort and should only be called after a compound plan executes.
pub async fn ingest_orchestration_run_to_kg(
    extension_manager: &ExtensionManager,
    session_id: &str,
    working_dir: &Path,
    run: &OrchestrationRunRecord,
) -> Result<()> {
    if !extension_manager
        .is_extension_enabled(KG_EXTENSION_NAME)
        .await
    {
        return Ok(());
    }

    let root_entity = KgEntity {
        name: "orchestration".to_string(),
        entity_type: "root".to_string(),
        observations: vec!["Root namespace for compound orchestration runs".to_string()],
    };

    let run_entity_name = kg_entity_name(&run.run_id, None);
    let run_entity = KgEntity {
        name: run_entity_name.clone(),
        entity_type: "run".to_string(),
        observations: vec![],
    };

    let mut entities = vec![root_entity, run_entity];
    let mut relations = vec![KgRelation {
        from: "orchestration".to_string(),
        to: run_entity_name.clone(),
        relation_type: "contains".to_string(),
    }];

    let mut observations = vec![KgObservation {
        entity_name: run_entity_name.clone(),
        contents: vec![
            truncate_observation(format!("run_id: {}", run.run_id)),
            truncate_observation(format!("created_at_ms: {}", run.created_at_ms)),
            truncate_observation(format!("max_concurrency: {}", run.max_concurrency)),
            truncate_observation(format!("root_request: {}", run.root_request)),
            truncate_observation(format!("aggregated_output: {}", run.aggregated_output)),
        ],
    }];

    for task in &run.tasks {
        let task_entity_name = kg_entity_name(&run.run_id, Some(&task.task_id));
        entities.push(KgEntity {
            name: task_entity_name.clone(),
            entity_type: "task".to_string(),
            observations: vec![],
        });

        relations.push(KgRelation {
            from: run_entity_name.clone(),
            to: task_entity_name.clone(),
            relation_type: "has_task".to_string(),
        });

        for dep in &task.depends_on {
            relations.push(KgRelation {
                from: task_entity_name.clone(),
                to: kg_entity_name(&run.run_id, Some(dep)),
                relation_type: "depends_on".to_string(),
            });
        }

        observations.push(KgObservation {
            entity_name: task_entity_name,
            contents: vec![
                truncate_observation(format!("task_id: {}", task.task_id)),
                truncate_observation(format!("agent_name: {}", task.agent_name)),
                truncate_observation(format!("mode_slug: {}", task.mode_slug)),
                truncate_observation(format!("status: {:?}", task.status)),
                truncate_observation(format!("duration_ms: {}", task.duration_ms)),
                truncate_observation(format!("description: {}", task.description)),
                truncate_observation(format!("output: {}", task.output)),
            ],
        });
    }

    let create_entities = CallToolRequestParams {
        meta: None,
        name: Cow::Owned(format!("{KG_EXTENSION_NAME}__{KG_TOOL_CREATE_ENTITIES}")),
        arguments: Some(JsonObject::from_iter([(
            "entities".to_string(),
            serde_json::to_value(&entities)?,
        )])),
        task: None,
    };
    let create_relations = CallToolRequestParams {
        meta: None,
        name: Cow::Owned(format!("{KG_EXTENSION_NAME}__{KG_TOOL_CREATE_RELATIONS}")),
        arguments: Some(JsonObject::from_iter([(
            "relations".to_string(),
            serde_json::to_value(&relations)?,
        )])),
        task: None,
    };
    let add_observations = CallToolRequestParams {
        meta: None,
        name: Cow::Owned(format!("{KG_EXTENSION_NAME}__{KG_TOOL_ADD_OBSERVATIONS}")),
        arguments: Some(JsonObject::from_iter([(
            "observations".to_string(),
            serde_json::to_value(&observations)?,
        )])),
        task: None,
    };

    // Tool calls are best-effort; if any fail we propagate error so the caller can log.
    let cancel = CancellationToken::default();

    extension_manager
        .dispatch_tool_call(
            session_id,
            create_entities,
            Some(working_dir),
            cancel.clone(),
        )
        .await?
        .result
        .await?;

    extension_manager
        .dispatch_tool_call(
            session_id,
            create_relations,
            Some(working_dir),
            cancel.clone(),
        )
        .await?
        .result
        .await?;

    extension_manager
        .dispatch_tool_call(session_id, add_observations, Some(working_dir), cancel)
        .await?
        .result
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

        let run = persist_orchestration_run(
            &sm,
            &session.id,
            "root request",
            &plan,
            &results,
            3,
            "aggregated",
        )
        .await
        .unwrap()
        .expect("compound run should be stored");

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

        // Ensure the returned run matches what's stored.
        assert_eq!(state.runs[0].run_id, run.run_id);
    }

    #[tokio::test]
    async fn ingest_orchestration_run_to_kg_is_noop_when_extension_disabled() {
        use tokio::sync::Mutex;

        let temp_dir = TempDir::new().unwrap();
        let sm = SessionManager::new(temp_dir.path().join("data"));
        let session = sm
            .create_session(PathBuf::from("/tmp"), "test".into(), SessionType::User)
            .await
            .unwrap();

        let provider = std::sync::Arc::new(Mutex::new(None));
        let em = ExtensionManager::new(provider, std::sync::Arc::new(sm));

        let plan = make_plan();
        let results = make_results();

        let run = OrchestrationRunRecord {
            run_id: "run_1".to_string(),
            created_at_ms: 0,
            max_concurrency: 3,
            root_request: "root".to_string(),
            aggregated_output: "agg".to_string(),
            tasks: tasks_to_records(&plan.tasks, &results),
        };

        ingest_orchestration_run_to_kg(&em, &session.id, Path::new("/tmp"), &run)
            .await
            .unwrap();
    }
}
