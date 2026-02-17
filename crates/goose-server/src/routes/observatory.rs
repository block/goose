use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

// ── Response types ─────────────────────────────────────────────────

/// Unified Observatory dashboard — system health + active agents + performance snapshot.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ObservatoryDashboard {
    pub health: SystemHealth,
    pub active_agents: Vec<ActiveAgent>,
    pub performance: PerformanceSnapshot,
}

/// System health indicators.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub active_sessions: i32,
    pub uptime_seconds: u64,
    pub registered_agents: usize,
    pub enabled_agents: usize,
    pub pool_instances: usize,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// A currently active or registered agent with its live status.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActiveAgent {
    pub name: String,
    pub kind: ActiveAgentKind,
    pub status: ActiveAgentStatus,
    pub mode_count: usize,
    pub current_tasks: usize,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActiveAgentKind {
    Builtin,
    A2a,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActiveAgentStatus {
    Idle,
    Working,
    Disabled,
    Disconnected,
}

/// Aggregated performance snapshot for the dashboard.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSnapshot {
    pub total_sessions_24h: i32,
    pub total_tool_calls_24h: i32,
    pub tool_error_rate_24h: f64,
    pub avg_messages_per_session: f64,
    pub top_tools: Vec<TopTool>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TopTool {
    pub name: String,
    pub calls: i32,
    pub error_rate: f64,
}

// ── Handlers ───────────────────────────────────────────────────────

/// GET /observatory/dashboard — unified system health + agents + performance.
#[utoipa::path(
    get,
    path = "/observatory/dashboard",
    tag = "Observatory",
    responses(
        (status = 200, description = "Observatory dashboard", body = ObservatoryDashboard)
    )
)]
pub async fn get_dashboard(State(state): State<Arc<AppState>>) -> Json<ObservatoryDashboard> {
    let health = build_health(&state).await;
    let active_agents = build_active_agents(&state).await;
    let performance = build_performance(&state).await;

    Json(ObservatoryDashboard {
        health,
        active_agents,
        performance,
    })
}

/// GET /observatory/active-agents — real-time agent status.
#[utoipa::path(
    get,
    path = "/observatory/active-agents",
    tag = "Observatory",
    responses(
        (status = 200, description = "Active agents list", body = Vec<ActiveAgent>)
    )
)]
pub async fn get_active_agents(State(state): State<Arc<AppState>>) -> Json<Vec<ActiveAgent>> {
    Json(build_active_agents(&state).await)
}

/// GET /observatory/health — system health check.
#[utoipa::path(
    get,
    path = "/observatory/health",
    tag = "Observatory",
    responses(
        (status = 200, description = "System health", body = SystemHealth)
    )
)]
pub async fn get_health(State(state): State<Arc<AppState>>) -> Json<SystemHealth> {
    Json(build_health(&state).await)
}

// ── Builders ───────────────────────────────────────────────────────

async fn build_health(state: &AppState) -> SystemHealth {
    use goose::agents::orchestrator_agent::OrchestratorAgent;

    let provider = Arc::new(tokio::sync::Mutex::new(None));
    let router = OrchestratorAgent::new(provider);
    let slots = router.slots();
    let registered = slots.len();
    let enabled = slots.iter().filter(|s| s.enabled).count();

    let pool_snapshots = state.agent_pool.status_all().await;
    let pool_instances = pool_snapshots.len();

    // Count active sessions from pool
    let active_sessions = pool_snapshots
        .iter()
        .filter(|s| matches!(s.status, goose::execution::pool::InstanceStatus::Running))
        .count() as i32;

    let status = if active_sessions >= 0 && enabled > 0 {
        HealthStatus::Healthy
    } else if enabled == 0 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Unhealthy
    };

    // Uptime: use process start time approximation
    let uptime_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    SystemHealth {
        status,
        active_sessions,
        uptime_seconds,
        registered_agents: registered,
        enabled_agents: enabled,
        pool_instances,
    }
}

async fn build_active_agents(state: &AppState) -> Vec<ActiveAgent> {
    use goose::agents::orchestrator_agent::OrchestratorAgent;

    let provider = Arc::new(tokio::sync::Mutex::new(None));
    let router = OrchestratorAgent::new(provider);
    let slots = router.slots();

    let pool_snapshots = state.agent_pool.status_all().await;

    let mut agents: Vec<ActiveAgent> = slots
        .iter()
        .map(|slot| {
            let running_tasks = pool_snapshots
                .iter()
                .filter(|s| {
                    s.persona == slot.name
                        && matches!(s.status, goose::execution::pool::InstanceStatus::Running)
                })
                .count();

            let status = if !slot.enabled {
                ActiveAgentStatus::Disabled
            } else if running_tasks > 0 {
                ActiveAgentStatus::Working
            } else {
                ActiveAgentStatus::Idle
            };

            ActiveAgent {
                name: slot.name.clone(),
                kind: ActiveAgentKind::Builtin,
                status,
                mode_count: slot.modes.len(),
                current_tasks: running_tasks,
            }
        })
        .collect();

    // Add pool instances that aren't mapped to builtin agent slots
    let builtin_names: std::collections::HashSet<&str> =
        slots.iter().map(|s| s.name.as_str()).collect();

    for snapshot in &pool_snapshots {
        if !builtin_names.contains(snapshot.persona.as_str()) && !snapshot.persona.is_empty() {
            let status = match snapshot.status {
                goose::execution::pool::InstanceStatus::Running => ActiveAgentStatus::Working,
                goose::execution::pool::InstanceStatus::Completed => ActiveAgentStatus::Idle,
                _ => ActiveAgentStatus::Disconnected,
            };
            agents.push(ActiveAgent {
                name: snapshot.persona.clone(),
                kind: ActiveAgentKind::A2a,
                status,
                mode_count: 0,
                current_tasks: if matches!(
                    snapshot.status,
                    goose::execution::pool::InstanceStatus::Running
                ) {
                    1
                } else {
                    0
                },
            });
        }
    }

    agents
}

async fn build_performance(state: &AppState) -> PerformanceSnapshot {
    use goose::session::tool_analytics::ToolAnalyticsStore;

    let sm = state.session_manager();
    let pool = match sm.storage().pool().await {
        Ok(p) => p,
        Err(_) => {
            return PerformanceSnapshot {
                total_sessions_24h: 0,
                total_tool_calls_24h: 0,
                tool_error_rate_24h: 0.0,
                avg_messages_per_session: 0.0,
                top_tools: vec![],
            };
        }
    };

    let store = ToolAnalyticsStore::new(pool);

    // Get live metrics for the 24h window
    let live = match store.get_live_metrics().await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Observatory live metrics unavailable: {e}");
            return PerformanceSnapshot {
                total_sessions_24h: 0,
                total_tool_calls_24h: 0,
                tool_error_rate_24h: 0.0,
                avg_messages_per_session: 0.0,
                top_tools: vec![],
            };
        }
    };

    // Get tool analytics for top tools
    let tools_result = store.get_tool_analytics(1).await;

    let top_tools: Vec<TopTool> = match tools_result {
        Ok(tools) => tools
            .tool_usage
            .into_iter()
            .take(5)
            .map(|t| TopTool {
                name: t.tool_name,
                calls: t.call_count as i32,
                error_rate: if t.call_count > 0 {
                    t.error_count as f64 / t.call_count as f64
                } else {
                    0.0
                },
            })
            .collect(),
        Err(e) => {
            tracing::warn!("Observatory tool analytics unavailable: {e}");
            vec![]
        }
    };

    PerformanceSnapshot {
        total_sessions_24h: live.active_sessions_24h,
        total_tool_calls_24h: live.tool_calls_24h,
        tool_error_rate_24h: 1.0 - live.success_rate_1h,
        avg_messages_per_session: 0.0, // TODO: compute from agent performance
        top_tools,
    }
}

// ── Routes ─────────────────────────────────────────────────────────

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/observatory/dashboard", get(get_dashboard))
        .route("/observatory/active-agents", get(get_active_agents))
        .route("/observatory/health", get(get_health))
        .with_state(state)
}
