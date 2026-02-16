use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::agents::intent_router::IntentRouter;
use crate::agents::routing_eval::{self, RoutingEvalCase, RoutingEvalSet};

// ── Types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvalDataset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub cases: Vec<EvalTestCase>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvalTestCase {
    pub id: String,
    pub input: String,
    pub expected_agent: String,
    pub expected_mode: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvalDatasetSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub case_count: i64,
    pub last_run_accuracy: Option<f64>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDatasetRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub cases: Vec<CreateTestCaseRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTestCaseRequest {
    pub input: String,
    pub expected_agent: String,
    pub expected_mode: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvalRunSummary {
    pub id: String,
    pub dataset_id: String,
    pub dataset_name: String,
    pub version_tag: String,
    pub goose_version: String,
    pub started_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub total_cases: i64,
    pub correct: i64,
    pub overall_accuracy: f64,
    pub agent_accuracy: f64,
    pub mode_accuracy: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvalRunDetail {
    pub id: String,
    pub dataset_id: String,
    pub dataset_name: String,
    pub version_tag: String,
    pub goose_version: String,
    pub started_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub total_cases: i64,
    pub correct: i64,
    pub agent_correct: i64,
    pub overall_accuracy: f64,
    pub agent_accuracy: f64,
    pub mode_accuracy: f64,
    pub status: String,
    pub per_agent: Vec<AgentResult>,
    pub failures: Vec<FailureDetail>,
    pub confusion_matrix: ConfusionMatrix,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentResult {
    pub agent: String,
    pub pass: i64,
    pub fail: i64,
    pub accuracy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FailureDetail {
    pub input: String,
    pub expected_agent: String,
    pub expected_mode: String,
    pub actual_agent: String,
    pub actual_mode: String,
    pub confidence: f32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfusionMatrix {
    pub labels: Vec<String>,
    pub matrix: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvalOverview {
    pub overall_accuracy: f64,
    pub agent_accuracy: f64,
    pub mode_accuracy: f64,
    pub total_test_cases: i64,
    pub total_runs: i64,
    pub last_run_status: String,
    pub last_run_at: Option<DateTime<Utc>>,
    pub accuracy_delta: f64,
    pub agent_accuracy_delta: f64,
    pub mode_accuracy_delta: f64,
    pub accuracy_trend: Vec<AccuracyTrendPoint>,
    pub per_agent_accuracy: Vec<AgentResult>,
    pub regressions: Vec<RegressionAlert>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccuracyTrendPoint {
    pub run_id: String,
    pub date: DateTime<Utc>,
    pub version_tag: String,
    pub overall_accuracy: f64,
    pub agent_accuracy: f64,
    pub mode_accuracy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegressionAlert {
    pub description: String,
    pub severity: String,
    pub run_id: String,
    pub version_tag: String,
    pub delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunEvalRequest {
    pub dataset_id: String,
    #[serde(default)]
    pub version_tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TopicAnalytics {
    pub topic: String,
    pub case_count: i64,
    pub accuracy: f64,
    pub agent_distribution: Vec<TopicAgentDistribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TopicAgentDistribution {
    pub agent: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunComparison {
    pub baseline: RunComparisonSide,
    pub candidate: RunComparisonSide,
    pub overall_delta: f64,
    pub agent_delta: f64,
    pub mode_delta: f64,
    pub new_failures: Vec<FailureDetail>,
    pub fixed_cases: Vec<FixedCase>,
    pub per_agent_delta: Vec<AgentDelta>,
    pub correlation: CorrelationInsight,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunComparisonSide {
    pub run_id: String,
    pub version_tag: String,
    pub goose_version: String,
    pub dataset_name: String,
    pub started_at: DateTime<Utc>,
    pub overall_accuracy: f64,
    pub agent_accuracy: f64,
    pub mode_accuracy: f64,
    pub total_cases: i64,
    pub correct: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FixedCase {
    pub input: String,
    pub agent: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentDelta {
    pub agent: String,
    pub baseline_accuracy: f64,
    pub candidate_accuracy: f64,
    pub delta: f64,
    pub baseline_cases: i64,
    pub candidate_cases: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationInsight {
    pub version_changed: bool,
    pub goose_version_delta: String,
    pub version_tag_delta: String,
    pub summary: String,
}

// ── Stored run data (JSON blobs in DB) ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredMetrics {
    per_agent: HashMap<String, StoredAgentMetrics>,
    per_mode: HashMap<String, StoredModeMetrics>,
    confusion_matrix: Vec<StoredConfusionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAgentMetrics {
    total: usize,
    correct: usize,
    accuracy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredModeMetrics {
    total: usize,
    correct: usize,
    accuracy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredConfusionEntry {
    expected: String,
    actual: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredResult {
    input: String,
    expected_agent: String,
    expected_mode: String,
    actual_agent: String,
    actual_mode: String,
    confidence: f32,
    agent_correct: bool,
    mode_correct: bool,
    fully_correct: bool,
    tags: Vec<String>,
}

// ── Storage impl ───────────────────────────────────────────────────

pub struct EvalStorage<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> EvalStorage<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    // ── Dataset CRUD ───────────────────────────────────────────────

    pub async fn list_datasets(&self) -> Result<Vec<EvalDatasetSummary>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, i64, String, String)>(
            r#"
            SELECT
                d.id, d.name, d.description, d.tags_json,
                (SELECT COUNT(*) FROM eval_test_cases WHERE dataset_id = d.id) as case_count,
                d.created_at, d.updated_at
            FROM eval_datasets d
            ORDER BY d.updated_at DESC
            "#,
        )
        .fetch_all(self.pool)
        .await?;

        let mut datasets = Vec::new();
        for (id, name, description, tags_json, case_count, created_at, updated_at) in rows {
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            let last_run = sqlx::query_as::<_, (f64, String)>(
                "SELECT overall_accuracy, started_at FROM eval_runs WHERE dataset_id = ? ORDER BY started_at DESC LIMIT 1",
            )
            .bind(&id)
            .fetch_optional(self.pool)
            .await?;

            datasets.push(EvalDatasetSummary {
                id,
                name,
                description,
                tags,
                case_count,
                last_run_accuracy: last_run.as_ref().map(|r| r.0),
                last_run_at: last_run.and_then(|r| r.1.parse::<DateTime<Utc>>().ok()),
                created_at: created_at.parse().unwrap_or_default(),
                updated_at: updated_at.parse().unwrap_or_default(),
            });
        }

        Ok(datasets)
    }

    pub async fn get_dataset(&self, id: &str) -> Result<EvalDataset> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, name, description, tags_json, created_at, updated_at FROM eval_datasets WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Dataset not found"))?;

        let tags: Vec<String> = serde_json::from_str(&row.3).unwrap_or_default();

        let case_rows = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, input, expected_agent, expected_mode, tags_json FROM eval_test_cases WHERE dataset_id = ? ORDER BY sort_order",
        )
        .bind(id)
        .fetch_all(self.pool)
        .await?;

        let cases = case_rows
            .into_iter()
            .map(|(cid, input, agent, mode, ctags_json)| {
                let ctags: Vec<String> = serde_json::from_str(&ctags_json).unwrap_or_default();
                EvalTestCase {
                    id: cid,
                    input,
                    expected_agent: agent,
                    expected_mode: mode,
                    tags: ctags,
                }
            })
            .collect();

        Ok(EvalDataset {
            id: row.0,
            name: row.1,
            description: row.2,
            tags,
            cases,
            created_at: row.4.parse().unwrap_or_default(),
            updated_at: row.5.parse().unwrap_or_default(),
        })
    }

    pub async fn create_dataset(&self, req: CreateDatasetRequest) -> Result<EvalDataset> {
        let id = format!(
            "ds_{}",
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("0")
        );
        let tags_json = serde_json::to_string(&req.tags)?;

        sqlx::query(
            "INSERT INTO eval_datasets (id, name, description, tags_json) VALUES (?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&tags_json)
        .execute(self.pool)
        .await?;

        for (i, case) in req.cases.iter().enumerate() {
            let case_id = format!(
                "tc_{}",
                uuid::Uuid::new_v4()
                    .to_string()
                    .split('-')
                    .next()
                    .unwrap_or("0")
            );
            let case_tags_json = serde_json::to_string(&case.tags)?;
            sqlx::query(
                "INSERT INTO eval_test_cases (id, dataset_id, input, expected_agent, expected_mode, tags_json, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&case_id)
            .bind(&id)
            .bind(&case.input)
            .bind(&case.expected_agent)
            .bind(&case.expected_mode)
            .bind(&case_tags_json)
            .bind(i as i32)
            .execute(self.pool)
            .await?;
        }

        self.get_dataset(&id).await
    }

    pub async fn update_dataset(&self, id: &str, req: CreateDatasetRequest) -> Result<EvalDataset> {
        let tags_json = serde_json::to_string(&req.tags)?;

        sqlx::query(
            "UPDATE eval_datasets SET name = ?, description = ?, tags_json = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&req.name)
        .bind(&req.description)
        .bind(&tags_json)
        .bind(id)
        .execute(self.pool)
        .await?;

        // Replace all cases
        sqlx::query("DELETE FROM eval_test_cases WHERE dataset_id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;

        for (i, case) in req.cases.iter().enumerate() {
            let case_id = format!(
                "tc_{}",
                uuid::Uuid::new_v4()
                    .to_string()
                    .split('-')
                    .next()
                    .unwrap_or("0")
            );
            let case_tags_json = serde_json::to_string(&case.tags)?;
            sqlx::query(
                "INSERT INTO eval_test_cases (id, dataset_id, input, expected_agent, expected_mode, tags_json, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&case_id)
            .bind(id)
            .bind(&case.input)
            .bind(&case.expected_agent)
            .bind(&case.expected_mode)
            .bind(&case_tags_json)
            .bind(i as i32)
            .execute(self.pool)
            .await?;
        }

        self.get_dataset(id).await
    }

    pub async fn delete_dataset(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM eval_test_cases WHERE dataset_id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;
        sqlx::query("DELETE FROM eval_runs WHERE dataset_id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;
        sqlx::query("DELETE FROM eval_datasets WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // ── Run eval ───────────────────────────────────────────────────

    pub async fn run_eval(&self, req: RunEvalRequest) -> Result<EvalRunDetail> {
        let dataset = self.get_dataset(&req.dataset_id).await?;
        let router = IntentRouter::new();
        let goose_version = env!("CARGO_PKG_VERSION").to_string();

        // Build eval set from stored cases
        let eval_cases: Vec<RoutingEvalCase> = dataset
            .cases
            .iter()
            .map(|c| RoutingEvalCase {
                input: c.input.clone(),
                expected_agent: c.expected_agent.clone(),
                expected_mode: c.expected_mode.clone(),
                tags: c.tags.clone(),
            })
            .collect();

        let eval_set = RoutingEvalSet {
            test_cases: eval_cases,
        };

        let start = std::time::Instant::now();
        let results = routing_eval::evaluate(&router, &eval_set);
        let metrics = routing_eval::compute_metrics(&results);
        let duration_ms = start.elapsed().as_millis() as i64;

        // Determine status
        let status = if metrics.overall_accuracy >= 0.95 {
            "pass"
        } else if metrics.overall_accuracy >= 0.85 {
            "degraded"
        } else {
            "fail"
        };

        // Build stored results with tags
        let stored_results: Vec<StoredResult> = results
            .iter()
            .zip(dataset.cases.iter())
            .map(|(r, c)| StoredResult {
                input: r.input.clone(),
                expected_agent: r.expected_agent.clone(),
                expected_mode: r.expected_mode.clone(),
                actual_agent: r.actual_agent.clone(),
                actual_mode: r.actual_mode.clone(),
                confidence: r.confidence,
                agent_correct: r.agent_correct,
                mode_correct: r.mode_correct,
                fully_correct: r.fully_correct,
                tags: c.tags.clone(),
            })
            .collect();

        let stored_metrics = StoredMetrics {
            per_agent: metrics
                .per_agent
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        StoredAgentMetrics {
                            total: v.total,
                            correct: v.correct,
                            accuracy: v.accuracy,
                        },
                    )
                })
                .collect(),
            per_mode: metrics
                .per_mode
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        StoredModeMetrics {
                            total: v.total,
                            correct: v.correct,
                            accuracy: v.accuracy,
                        },
                    )
                })
                .collect(),
            confusion_matrix: metrics
                .confusion_matrix
                .iter()
                .map(|e| StoredConfusionEntry {
                    expected: e.expected.clone(),
                    actual: e.actual.clone(),
                    count: e.count,
                })
                .collect(),
        };

        let run_id = format!(
            "run_{}",
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("0")
        );
        let metrics_json = serde_json::to_string(&stored_metrics)?;
        let results_json = serde_json::to_string(&stored_results)?;

        sqlx::query(
            r#"INSERT INTO eval_runs
            (id, dataset_id, version_tag, goose_version, duration_ms, total_cases, correct, agent_correct,
             overall_accuracy, agent_accuracy, mode_accuracy, status, metrics_json, results_json)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&run_id)
        .bind(&req.dataset_id)
        .bind(&req.version_tag)
        .bind(&goose_version)
        .bind(duration_ms)
        .bind(metrics.total as i64)
        .bind(metrics.correct as i64)
        .bind(metrics.agent_correct as i64)
        .bind(metrics.overall_accuracy)
        .bind(metrics.agent_accuracy)
        .bind(metrics.mode_accuracy_given_agent)
        .bind(status)
        .bind(&metrics_json)
        .bind(&results_json)
        .execute(self.pool)
        .await?;

        self.get_run_detail(&run_id).await
    }

    // ── Run history ────────────────────────────────────────────────

    pub async fn list_runs(
        &self,
        dataset_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<EvalRunSummary>> {
        let (query, bind_val) = if let Some(ds_id) = dataset_id {
            (
                r#"SELECT r.id, r.dataset_id, d.name, r.version_tag, r.goose_version,
                   r.started_at, r.duration_ms, r.total_cases, r.correct,
                   r.overall_accuracy, r.agent_accuracy, r.mode_accuracy, r.status
                   FROM eval_runs r JOIN eval_datasets d ON r.dataset_id = d.id
                   WHERE r.dataset_id = ?
                   ORDER BY r.started_at DESC LIMIT ?"#
                    .to_string(),
                Some(ds_id.to_string()),
            )
        } else {
            (
                r#"SELECT r.id, r.dataset_id, d.name, r.version_tag, r.goose_version,
                   r.started_at, r.duration_ms, r.total_cases, r.correct,
                   r.overall_accuracy, r.agent_accuracy, r.mode_accuracy, r.status
                   FROM eval_runs r JOIN eval_datasets d ON r.dataset_id = d.id
                   ORDER BY r.started_at DESC LIMIT ?"#
                    .to_string(),
                None,
            )
        };

        let rows = if let Some(ds_id) = bind_val {
            sqlx::query_as::<
                _,
                (
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    i64,
                    i64,
                    i64,
                    f64,
                    f64,
                    f64,
                    String,
                ),
            >(&query)
            .bind(ds_id)
            .bind(limit)
            .fetch_all(self.pool)
            .await?
        } else {
            sqlx::query_as::<
                _,
                (
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    i64,
                    i64,
                    i64,
                    f64,
                    f64,
                    f64,
                    String,
                ),
            >(&query)
            .bind(limit)
            .fetch_all(self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|r| EvalRunSummary {
                id: r.0,
                dataset_id: r.1,
                dataset_name: r.2,
                version_tag: r.3,
                goose_version: r.4,
                started_at: r.5.parse().unwrap_or_default(),
                duration_ms: r.6,
                total_cases: r.7,
                correct: r.8,
                overall_accuracy: r.9,
                agent_accuracy: r.10,
                mode_accuracy: r.11,
                status: r.12,
            })
            .collect())
    }

    pub async fn get_run_detail(&self, run_id: &str) -> Result<EvalRunDetail> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                String,
                i64,
                i64,
                i64,
                i64,
                f64,
                f64,
                f64,
                String,
                String,
                String,
            ),
        >(
            r#"SELECT r.id, r.dataset_id, d.name, r.version_tag, r.goose_version,
               r.duration_ms, r.total_cases, r.correct, r.agent_correct,
               r.overall_accuracy, r.agent_accuracy, r.mode_accuracy,
               r.status, r.metrics_json, r.results_json
               FROM eval_runs r JOIN eval_datasets d ON r.dataset_id = d.id
               WHERE r.id = ?"#,
        )
        .bind(run_id)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Run not found"))?;

        let started_at_row =
            sqlx::query_scalar::<_, String>("SELECT started_at FROM eval_runs WHERE id = ?")
                .bind(run_id)
                .fetch_one(self.pool)
                .await?;

        let stored_metrics: StoredMetrics =
            serde_json::from_str(&row.13).unwrap_or(StoredMetrics {
                per_agent: HashMap::new(),
                per_mode: HashMap::new(),
                confusion_matrix: Vec::new(),
            });

        let stored_results: Vec<StoredResult> = serde_json::from_str(&row.14).unwrap_or_default();

        let per_agent: Vec<AgentResult> = stored_metrics
            .per_agent
            .into_iter()
            .map(|(agent, m)| AgentResult {
                agent,
                pass: m.correct as i64,
                fail: (m.total - m.correct) as i64,
                accuracy: m.accuracy,
            })
            .collect();

        let failures: Vec<FailureDetail> = stored_results
            .iter()
            .filter(|r| !r.fully_correct)
            .map(|r| FailureDetail {
                input: r.input.clone(),
                expected_agent: r.expected_agent.clone(),
                expected_mode: r.expected_mode.clone(),
                actual_agent: r.actual_agent.clone(),
                actual_mode: r.actual_mode.clone(),
                confidence: r.confidence,
                tags: r.tags.clone(),
            })
            .collect();

        // Build confusion matrix
        let mut all_agents: Vec<String> = stored_results
            .iter()
            .flat_map(|r| vec![r.expected_agent.clone(), r.actual_agent.clone()])
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        all_agents.sort();

        let agent_idx: HashMap<String, usize> = all_agents
            .iter()
            .enumerate()
            .map(|(i, a)| (a.clone(), i))
            .collect();

        let n = all_agents.len();
        let mut matrix = vec![vec![0i64; n]; n];
        for r in &stored_results {
            if let (Some(&ei), Some(&ai)) = (
                agent_idx.get(&r.expected_agent),
                agent_idx.get(&r.actual_agent),
            ) {
                matrix[ei][ai] += 1;
            }
        }

        Ok(EvalRunDetail {
            id: row.0,
            dataset_id: row.1,
            dataset_name: row.2,
            version_tag: row.3,
            goose_version: row.4,
            started_at: started_at_row.parse().unwrap_or_default(),
            duration_ms: row.5,
            total_cases: row.6,
            correct: row.7,
            agent_correct: row.8,
            overall_accuracy: row.9,
            agent_accuracy: row.10,
            mode_accuracy: row.11,
            status: row.12,
            per_agent,
            failures,
            confusion_matrix: ConfusionMatrix {
                labels: all_agents,
                matrix,
            },
        })
    }

    // ── Overview / analytics ───────────────────────────────────────

    pub async fn get_overview(&self) -> Result<EvalOverview> {
        let runs = self.list_runs(None, 50).await?;

        if runs.is_empty() {
            return Ok(EvalOverview {
                overall_accuracy: 0.0,
                agent_accuracy: 0.0,
                mode_accuracy: 0.0,
                total_test_cases: 0,
                total_runs: 0,
                last_run_status: "none".to_string(),
                last_run_at: None,
                accuracy_delta: 0.0,
                agent_accuracy_delta: 0.0,
                mode_accuracy_delta: 0.0,
                accuracy_trend: Vec::new(),
                per_agent_accuracy: Vec::new(),
                regressions: Vec::new(),
            });
        }

        let latest = &runs[0];
        let previous = runs.get(1);

        let accuracy_delta = previous
            .map(|p| latest.overall_accuracy - p.overall_accuracy)
            .unwrap_or(0.0);
        let agent_accuracy_delta = previous
            .map(|p| latest.agent_accuracy - p.agent_accuracy)
            .unwrap_or(0.0);
        let mode_accuracy_delta = previous
            .map(|p| latest.mode_accuracy - p.mode_accuracy)
            .unwrap_or(0.0);

        let total_test_cases = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM eval_test_cases")
            .fetch_one(self.pool)
            .await?;

        // Accuracy trend (last 50 runs)
        let accuracy_trend: Vec<AccuracyTrendPoint> = runs
            .iter()
            .rev()
            .map(|r| AccuracyTrendPoint {
                run_id: r.id.clone(),
                date: r.started_at,
                version_tag: r.version_tag.clone(),
                overall_accuracy: r.overall_accuracy,
                agent_accuracy: r.agent_accuracy,
                mode_accuracy: r.mode_accuracy,
            })
            .collect();

        // Per-agent accuracy from latest run
        let latest_detail = self.get_run_detail(&latest.id).await?;
        let per_agent_accuracy = latest_detail.per_agent;

        // Detect regressions (accuracy drops > 2%)
        let mut regressions = Vec::new();
        for window in runs.windows(2) {
            let newer = &window[0];
            let older = &window[1];
            let delta = newer.overall_accuracy - older.overall_accuracy;
            if delta < -0.02 {
                regressions.push(RegressionAlert {
                    description: format!(
                        "Overall accuracy dropped {:.1}% from {} to {}",
                        delta * 100.0,
                        older.version_tag,
                        newer.version_tag
                    ),
                    severity: if delta < -0.05 { "high" } else { "medium" }.to_string(),
                    run_id: newer.id.clone(),
                    version_tag: newer.version_tag.clone(),
                    delta,
                });
            }
        }

        Ok(EvalOverview {
            overall_accuracy: latest.overall_accuracy,
            agent_accuracy: latest.agent_accuracy,
            mode_accuracy: latest.mode_accuracy,
            total_test_cases,
            total_runs: runs.len() as i64,
            last_run_status: latest.status.clone(),
            last_run_at: Some(latest.started_at),
            accuracy_delta,
            agent_accuracy_delta,
            mode_accuracy_delta,
            accuracy_trend,
            per_agent_accuracy,
            regressions,
        })
    }

    // ── Topic analytics ────────────────────────────────────────────

    pub async fn get_topic_analytics(&self) -> Result<Vec<TopicAnalytics>> {
        // Get the latest run's results for topic analysis
        let latest_run_id = sqlx::query_scalar::<_, String>(
            "SELECT id FROM eval_runs ORDER BY started_at DESC LIMIT 1",
        )
        .fetch_optional(self.pool)
        .await?;

        let Some(run_id) = latest_run_id else {
            return Ok(Vec::new());
        };

        let results_json =
            sqlx::query_scalar::<_, String>("SELECT results_json FROM eval_runs WHERE id = ?")
                .bind(&run_id)
                .fetch_one(self.pool)
                .await?;

        let stored_results: Vec<StoredResult> =
            serde_json::from_str(&results_json).unwrap_or_default();

        // Group by tag
        let mut tag_stats: HashMap<String, (i64, i64, HashMap<String, i64>)> = HashMap::new();
        for r in &stored_results {
            for tag in &r.tags {
                let entry = tag_stats
                    .entry(tag.clone())
                    .or_insert_with(|| (0, 0, HashMap::new()));
                entry.0 += 1; // total
                if r.fully_correct {
                    entry.1 += 1; // correct
                }
                *entry.2.entry(r.expected_agent.clone()).or_insert(0) += 1;
            }
        }

        let mut topics: Vec<TopicAnalytics> = tag_stats
            .into_iter()
            .map(|(topic, (total, correct, agent_dist))| {
                let accuracy = if total > 0 {
                    correct as f64 / total as f64
                } else {
                    0.0
                };
                let agent_distribution = agent_dist
                    .into_iter()
                    .map(|(agent, count)| TopicAgentDistribution { agent, count })
                    .collect();
                TopicAnalytics {
                    topic,
                    case_count: total,
                    accuracy,
                    agent_distribution,
                }
            })
            .collect();

        topics.sort_by(|a, b| b.case_count.cmp(&a.case_count));
        Ok(topics)
    }

    // ── Run comparison ─────────────────────────────────────────────

    pub async fn compare_runs(
        &self,
        baseline_id: &str,
        candidate_id: &str,
    ) -> Result<RunComparison> {
        let baseline = self.get_run_detail(baseline_id).await?;
        let candidate = self.get_run_detail(candidate_id).await?;

        // Load stored results for both runs
        let baseline_results_json =
            sqlx::query_scalar::<_, String>("SELECT results_json FROM eval_runs WHERE id = ?")
                .bind(baseline_id)
                .fetch_one(self.pool)
                .await?;

        let candidate_results_json =
            sqlx::query_scalar::<_, String>("SELECT results_json FROM eval_runs WHERE id = ?")
                .bind(candidate_id)
                .fetch_one(self.pool)
                .await?;

        let baseline_results: Vec<StoredResult> =
            serde_json::from_str(&baseline_results_json).unwrap_or_default();
        let candidate_results: Vec<StoredResult> =
            serde_json::from_str(&candidate_results_json).unwrap_or_default();

        // Build input→result maps
        let baseline_map: HashMap<String, &StoredResult> = baseline_results
            .iter()
            .map(|r| (r.input.clone(), r))
            .collect();
        // candidate_map not needed - we iterate candidate_results directly

        // Find new failures (passed in baseline, failed in candidate)
        let new_failures: Vec<FailureDetail> = candidate_results
            .iter()
            .filter(|c| {
                !c.fully_correct
                    && baseline_map
                        .get(&c.input)
                        .map(|b| b.fully_correct)
                        .unwrap_or(false)
            })
            .map(|r| FailureDetail {
                input: r.input.clone(),
                expected_agent: r.expected_agent.clone(),
                expected_mode: r.expected_mode.clone(),
                actual_agent: r.actual_agent.clone(),
                actual_mode: r.actual_mode.clone(),
                confidence: r.confidence,
                tags: r.tags.clone(),
            })
            .collect();

        // Find fixed cases (failed in baseline, passed in candidate)
        let fixed_cases: Vec<FixedCase> = candidate_results
            .iter()
            .filter(|c| {
                c.fully_correct
                    && baseline_map
                        .get(&c.input)
                        .map(|b| !b.fully_correct)
                        .unwrap_or(false)
            })
            .map(|r| FixedCase {
                input: r.input.clone(),
                agent: r.actual_agent.clone(),
                mode: r.actual_mode.clone(),
            })
            .collect();

        // Per-agent deltas
        let mut all_agents: std::collections::HashSet<String> = std::collections::HashSet::new();
        for r in &baseline_results {
            all_agents.insert(r.expected_agent.clone());
        }
        for r in &candidate_results {
            all_agents.insert(r.expected_agent.clone());
        }

        let mut per_agent_delta: Vec<AgentDelta> = Vec::new();
        for agent in &all_agents {
            let b_cases: Vec<&StoredResult> = baseline_results
                .iter()
                .filter(|r| &r.expected_agent == agent)
                .collect();
            let c_cases: Vec<&StoredResult> = candidate_results
                .iter()
                .filter(|r| &r.expected_agent == agent)
                .collect();

            let b_correct = b_cases.iter().filter(|r| r.fully_correct).count() as f64;
            let c_correct = c_cases.iter().filter(|r| r.fully_correct).count() as f64;

            let b_acc = if b_cases.is_empty() {
                0.0
            } else {
                b_correct / b_cases.len() as f64
            };
            let c_acc = if c_cases.is_empty() {
                0.0
            } else {
                c_correct / c_cases.len() as f64
            };

            per_agent_delta.push(AgentDelta {
                agent: agent.clone(),
                baseline_accuracy: b_acc,
                candidate_accuracy: c_acc,
                delta: c_acc - b_acc,
                baseline_cases: b_cases.len() as i64,
                candidate_cases: c_cases.len() as i64,
            });
        }

        per_agent_delta.sort_by(|a, b| {
            a.delta
                .partial_cmp(&b.delta)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Correlation insight
        let version_changed = baseline.version_tag != candidate.version_tag;
        let goose_version_delta = if baseline.goose_version == candidate.goose_version {
            "same".to_string()
        } else {
            format!("{} → {}", baseline.goose_version, candidate.goose_version)
        };
        let version_tag_delta = if baseline.version_tag == candidate.version_tag {
            "same".to_string()
        } else {
            format!("{} → {}", baseline.version_tag, candidate.version_tag)
        };

        let overall_delta = candidate.overall_accuracy - baseline.overall_accuracy;
        let summary = if overall_delta.abs() < 0.01 {
            "No significant accuracy change detected.".to_string()
        } else if overall_delta > 0.0 {
            let mut msg = format!(
                "Accuracy improved by {:.1}% ({} fixed cases).",
                overall_delta * 100.0,
                fixed_cases.len()
            );
            if version_changed {
                msg.push_str(&format!(
                    " Version changed from {} to {}.",
                    baseline.version_tag, candidate.version_tag
                ));
            }
            msg
        } else {
            let mut msg = format!(
                "Accuracy regressed by {:.1}% ({} new failures).",
                overall_delta.abs() * 100.0,
                new_failures.len()
            );
            if version_changed {
                msg.push_str(&format!(
                    " Regression may correlate with version change {} → {}.",
                    baseline.version_tag, candidate.version_tag
                ));
            }
            if !per_agent_delta.is_empty() {
                let worst = &per_agent_delta[0];
                if worst.delta < -0.05 {
                    msg.push_str(&format!(
                        " Agent '{}' most affected ({:.1}% drop).",
                        worst.agent,
                        worst.delta.abs() * 100.0
                    ));
                }
            }
            msg
        };

        Ok(RunComparison {
            baseline: RunComparisonSide {
                run_id: baseline.id,
                version_tag: baseline.version_tag,
                goose_version: baseline.goose_version,
                dataset_name: baseline.dataset_name,
                started_at: baseline.started_at,
                overall_accuracy: baseline.overall_accuracy,
                agent_accuracy: baseline.agent_accuracy,
                mode_accuracy: baseline.mode_accuracy,
                total_cases: baseline.total_cases,
                correct: baseline.correct,
            },
            candidate: RunComparisonSide {
                run_id: candidate.id,
                version_tag: candidate.version_tag,
                goose_version: candidate.goose_version,
                dataset_name: candidate.dataset_name,
                started_at: candidate.started_at,
                overall_accuracy: candidate.overall_accuracy,
                agent_accuracy: candidate.agent_accuracy,
                mode_accuracy: candidate.mode_accuracy,
                total_cases: candidate.total_cases,
                correct: candidate.correct,
            },
            overall_delta,
            agent_delta: candidate.agent_accuracy - baseline.agent_accuracy,
            mode_delta: candidate.mode_accuracy - baseline.mode_accuracy,
            new_failures,
            fixed_cases,
            per_agent_delta,
            correlation: CorrelationInsight {
                version_changed,
                goose_version_delta,
                version_tag_delta,
                summary,
            },
        })
    }
}
