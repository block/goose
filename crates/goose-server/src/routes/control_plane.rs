use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

// ── Policy Management ──────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePolicyRuleRequest {
    pub id: String,
    pub description: Option<String>,
    pub priority: i32,
    pub effect: PolicyEffect,
    pub actions: Vec<String>,
    pub resources: Vec<String>,
    pub auth_methods: Option<Vec<String>>,
    pub required_roles: Option<Vec<String>>,
    pub tenant: Option<String>,
    pub reason: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicyEffect {
    Allow,
    Deny,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyRuleResponse {
    pub id: String,
    pub description: Option<String>,
    pub priority: i32,
    pub effect: String,
    pub actions: Vec<String>,
    pub resources: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyListResponse {
    pub rules: Vec<PolicyRuleResponse>,
    pub total: usize,
}

async fn list_policies(State(state): State<Arc<AppState>>) -> Json<PolicyListResponse> {
    let engine = state.policy_store.engine_for(None).await;
    let rules: Vec<PolicyRuleResponse> = engine
        .rules()
        .iter()
        .map(|r| PolicyRuleResponse {
            id: r.id.clone(),
            description: Some(r.description.clone()),
            priority: r.priority,
            effect: format!("{:?}", r.effect),
            actions: r.actions.clone(),
            resources: r.resources.clone(),
        })
        .collect();
    let total = rules.len();
    Json(PolicyListResponse { rules, total })
}

async fn create_policy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePolicyRuleRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let mut builder = goose::policy::PolicyRuleBuilder::new(&req.id).priority(req.priority);

    builder = match req.effect {
        PolicyEffect::Allow => builder.allow(),
        PolicyEffect::Deny => builder.deny(),
    };

    for action in &req.actions {
        builder = builder.actions(vec![action.clone()]);
    }
    for resource in &req.resources {
        builder = builder.resources(vec![resource.clone()]);
    }

    if let Some(desc) = req.description {
        builder = builder.description(&desc);
    }
    if let Some(methods) = req.auth_methods {
        builder = builder.auth_methods(methods);
    }
    if let Some(roles) = req.required_roles {
        builder = builder.required_roles(roles);
    }
    if let Some(tenant) = &req.tenant {
        builder = builder.tenant(tenant);
    }
    if let Some(reason) = req.reason {
        builder = builder.reason(&reason);
    }

    let rule = builder.build();

    if let Some(tenant) = &req.tenant {
        state.policy_store.add_tenant_rule(tenant, rule).await;
    } else {
        state.policy_store.add_rule(rule).await;
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": req.id, "status": "created" })),
    ))
}

async fn delete_policy(
    State(state): State<Arc<AppState>>,
    Path(rule_id): Path<String>,
) -> StatusCode {
    let removed = state.policy_store.remove_rule(&rule_id).await;
    if removed {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

// ── Quota Management ───────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateQuotaRequest {
    pub scope: QuotaScopeRequest,
    pub resource: String,
    pub window: String,
    pub max_value: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaScopeRequest {
    #[serde(rename = "type")]
    pub scope_type: String,
    pub tenant: Option<String>,
    pub user: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaStatusResponse {
    pub scope: String,
    pub resource: String,
    pub window: String,
    pub limit: u64,
    pub used: u64,
    pub remaining: u64,
    pub allowed: bool,
}

async fn list_quotas(State(state): State<Arc<AppState>>) -> Json<Vec<serde_json::Value>> {
    let limits = state.quota_manager.list_limits().await;
    let items: Vec<serde_json::Value> = limits
        .iter()
        .map(|l| {
            serde_json::json!({
                "scope": format!("{:?}", l.scope),
                "resource": format!("{:?}", l.resource),
                "window": format!("{:?}", l.window),
                "maxCount": l.max_value,
            })
        })
        .collect();
    Json(items)
}

async fn create_quota(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateQuotaRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let scope = parse_scope(&req.scope)?;
    let resource = parse_resource(&req.resource)?;
    let window = parse_window(&req.window)?;

    state
        .quota_manager
        .add_limit(goose::quotas::QuotaLimit {
            scope,
            resource,
            window,
            max_value: req.max_value,
        })
        .await;

    Ok(StatusCode::CREATED)
}

async fn check_quota(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateQuotaRequest>,
) -> Result<Json<QuotaStatusResponse>, (StatusCode, String)> {
    let scope = parse_scope(&req.scope)?;
    let resource = parse_resource(&req.resource)?;
    let _window = parse_window(&req.window)?;

    let decision = state.quota_manager.check(&scope, &resource).await;
    let (limit, used, remaining, allowed) = match &decision {
        goose::quotas::QuotaDecision::Allowed { remaining } => (0, 0, *remaining, true),
        goose::quotas::QuotaDecision::Exceeded {
            limit,
            used,
            resets_at: _,
        } => (*limit, *used, 0, false),
    };

    Ok(Json(QuotaStatusResponse {
        scope: format!("{:?}", scope),
        resource: format!("{:?}", resource),
        window: req.window,
        limit,
        used,
        remaining,
        allowed,
    }))
}

// ── Audit Event Stream ─────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventResponse {
    pub id: String,
    pub timestamp: String,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub outcome: String,
    pub tenant: Option<String>,
}

async fn list_recent_audits(State(state): State<Arc<AppState>>) -> Json<Vec<AuditEventResponse>> {
    let events = state.audit_logger.recent_events(100).await;
    let items: Vec<AuditEventResponse> = events
        .iter()
        .map(|e| AuditEventResponse {
            id: e.id.to_string(),
            timestamp: e.timestamp.to_rfc3339(),
            actor: format!("{:?}", e.actor),
            action: e.action.clone(),
            resource: e.resource.clone(),
            outcome: format!("{:?}", e.outcome),
            tenant: e.tenant.clone(),
        })
        .collect();
    Json(items)
}

// ── Agent Registry Management ──────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub name: String,
    pub enabled: bool,
    pub delegation: String,
    pub extensions: Vec<String>,
}

async fn list_agents(State(state): State<Arc<AppState>>) -> Json<Vec<AgentInfo>> {
    let agents = state.agent_slot_registry.all_agents().await;
    let mut result = Vec::new();
    for (name, enabled, delegation) in agents {
        let extensions = state
            .agent_slot_registry
            .get_bound_extensions(&name)
            .await
            .into_iter()
            .collect();
        result.push(AgentInfo {
            name,
            enabled,
            delegation: format!("{:?}", delegation),
            extensions,
        });
    }
    Json(result)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAgentRequest {
    pub name: String,
    pub delegation: String,
    pub url: Option<String>,
}

async fn register_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    match req.delegation.as_str() {
        "RemoteA2A" => {
            let url = req
                .url
                .ok_or((StatusCode::BAD_REQUEST, "url required for RemoteA2A".into()))?;
            state
                .agent_slot_registry
                .register_a2a_agent(&req.name, &url)
                .await;
        }
        "ExternalAcp" => {
            state
                .agent_slot_registry
                .register_acp_agent(&req.name)
                .await;
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unknown delegation type: {}", req.delegation),
            ));
        }
    }
    Ok(StatusCode::CREATED)
}

async fn unregister_agent(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> StatusCode {
    state.agent_slot_registry.unregister_agent(&name).await;
    StatusCode::NO_CONTENT
}

// ── System Info ────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlPlaneInfo {
    pub version: String,
    pub capabilities: Vec<String>,
    pub auth_methods: Vec<String>,
    pub endpoints: Vec<EndpointInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointInfo {
    pub path: String,
    pub method: String,
    pub description: String,
}

async fn control_plane_info() -> Json<ControlPlaneInfo> {
    Json(ControlPlaneInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: vec![
            "policy-engine".into(),
            "rbac".into(),
            "audit-logging".into(),
            "quota-management".into(),
            "agent-registry".into(),
            "oidc-sso".into(),
            "tenant-isolation".into(),
            "a2a-protocol".into(),
        ],
        auth_methods: vec!["oidc".into(), "api-key".into(), "session-token".into()],
        endpoints: vec![
            EndpointInfo {
                path: "/control-plane/v1/info".into(),
                method: "GET".into(),
                description: "Control plane capabilities and version".into(),
            },
            EndpointInfo {
                path: "/control-plane/v1/policies".into(),
                method: "GET".into(),
                description: "List all policy rules".into(),
            },
            EndpointInfo {
                path: "/control-plane/v1/policies".into(),
                method: "POST".into(),
                description: "Create a policy rule".into(),
            },
            EndpointInfo {
                path: "/control-plane/v1/policies/:id".into(),
                method: "DELETE".into(),
                description: "Delete a policy rule".into(),
            },
            EndpointInfo {
                path: "/control-plane/v1/quotas".into(),
                method: "GET".into(),
                description: "List all quota limits".into(),
            },
            EndpointInfo {
                path: "/control-plane/v1/quotas".into(),
                method: "POST".into(),
                description: "Create a quota limit".into(),
            },
            EndpointInfo {
                path: "/control-plane/v1/agents".into(),
                method: "GET".into(),
                description: "List registered agents".into(),
            },
            EndpointInfo {
                path: "/control-plane/v1/agents".into(),
                method: "POST".into(),
                description: "Register an external agent".into(),
            },
            EndpointInfo {
                path: "/control-plane/v1/agents/:name".into(),
                method: "DELETE".into(),
                description: "Unregister an agent".into(),
            },
            EndpointInfo {
                path: "/control-plane/v1/audit/events".into(),
                method: "GET".into(),
                description: "List recent audit events".into(),
            },
        ],
    })
}

// ── Helpers ────────────────────────────────────────────────────────

fn parse_scope(req: &QuotaScopeRequest) -> Result<goose::quotas::QuotaScope, (StatusCode, String)> {
    match req.scope_type.as_str() {
        "global" => Ok(goose::quotas::QuotaScope::Global),
        "tenant" => {
            let tenant = req
                .tenant
                .as_ref()
                .ok_or((StatusCode::BAD_REQUEST, "tenant required".into()))?;
            Ok(goose::quotas::QuotaScope::Tenant(tenant.clone()))
        }
        "user" => {
            let user = req
                .user
                .as_ref()
                .ok_or((StatusCode::BAD_REQUEST, "user required".into()))?;
            Ok(goose::quotas::QuotaScope::User(user.clone()))
        }
        "tenant_user" => {
            let tenant = req
                .tenant
                .as_ref()
                .ok_or((StatusCode::BAD_REQUEST, "tenant required".into()))?;
            let user = req
                .user
                .as_ref()
                .ok_or((StatusCode::BAD_REQUEST, "user required".into()))?;
            Ok(goose::quotas::QuotaScope::TenantUser {
                tenant: tenant.clone(),
                user: user.clone(),
            })
        }
        other => Err((
            StatusCode::BAD_REQUEST,
            format!("unknown scope type: {other}"),
        )),
    }
}

fn parse_resource(s: &str) -> Result<goose::quotas::QuotaResource, (StatusCode, String)> {
    match s {
        "executions" => Ok(goose::quotas::QuotaResource::Executions),
        "sessions" => Ok(goose::quotas::QuotaResource::Sessions),
        "tokens" => Ok(goose::quotas::QuotaResource::Tokens),
        other => Ok(goose::quotas::QuotaResource::Custom(other.to_string())),
    }
}

fn parse_window(s: &str) -> Result<goose::quotas::QuotaWindow, (StatusCode, String)> {
    match s {
        "per_minute" => Ok(goose::quotas::QuotaWindow::PerMinute),
        "per_hour" => Ok(goose::quotas::QuotaWindow::PerHour),
        "per_day" => Ok(goose::quotas::QuotaWindow::PerDay),
        "absolute" => Ok(goose::quotas::QuotaWindow::Absolute),
        other => Err((StatusCode::BAD_REQUEST, format!("unknown window: {other}"))),
    }
}

// ── Router ─────────────────────────────────────────────────────────

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // System info
        .route("/control-plane/v1/info", get(control_plane_info))
        // Policy management
        .route(
            "/control-plane/v1/policies",
            get(list_policies).post(create_policy),
        )
        .route("/control-plane/v1/policies/{id}", delete(delete_policy))
        // Quota management
        .route(
            "/control-plane/v1/quotas",
            get(list_quotas).post(create_quota),
        )
        .route("/control-plane/v1/quotas/check", post(check_quota))
        // Audit
        .route("/control-plane/v1/audit/events", get(list_recent_audits))
        // Agent registry
        .route(
            "/control-plane/v1/agents",
            get(list_agents).post(register_agent),
        )
        .route("/control-plane/v1/agents/{name}", delete(unregister_agent))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scope_req(
        scope_type: &str,
        tenant: Option<&str>,
        user: Option<&str>,
    ) -> QuotaScopeRequest {
        QuotaScopeRequest {
            scope_type: scope_type.to_string(),
            tenant: tenant.map(|s| s.to_string()),
            user: user.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_parse_scope_global() {
        let req = make_scope_req("global", None, None);
        let scope = parse_scope(&req);
        assert!(matches!(scope, Ok(goose::quotas::QuotaScope::Global)));
    }

    #[test]
    fn test_parse_scope_tenant() {
        let req = make_scope_req("tenant", Some("acme"), None);
        let scope = parse_scope(&req);
        assert!(matches!(scope, Ok(goose::quotas::QuotaScope::Tenant(ref t)) if t == "acme"));
    }

    #[test]
    fn test_parse_scope_user() {
        let req = make_scope_req("user", None, Some("alice"));
        let scope = parse_scope(&req);
        assert!(matches!(scope, Ok(goose::quotas::QuotaScope::User(ref u)) if u == "alice"));
    }

    #[test]
    fn test_parse_scope_tenant_user() {
        let req = make_scope_req("tenant_user", Some("acme"), Some("alice"));
        let scope = parse_scope(&req);
        match scope {
            Ok(goose::quotas::QuotaScope::TenantUser {
                ref tenant,
                ref user,
            }) => {
                assert_eq!(tenant, "acme");
                assert_eq!(user, "alice");
            }
            _ => panic!("Expected TenantUser"),
        }
    }

    #[test]
    fn test_parse_scope_missing_tenant() {
        let req = make_scope_req("tenant", None, None);
        assert!(parse_scope(&req).is_err());
    }

    #[test]
    fn test_parse_scope_missing_user() {
        let req = make_scope_req("user", None, None);
        assert!(parse_scope(&req).is_err());
    }

    #[test]
    fn test_parse_resource() {
        assert!(matches!(
            parse_resource("executions"),
            Ok(goose::quotas::QuotaResource::Executions)
        ));
        assert!(matches!(
            parse_resource("sessions"),
            Ok(goose::quotas::QuotaResource::Sessions)
        ));
        assert!(matches!(
            parse_resource("tokens"),
            Ok(goose::quotas::QuotaResource::Tokens)
        ));
        assert!(
            matches!(parse_resource("unknown"), Ok(goose::quotas::QuotaResource::Custom(ref s)) if s == "unknown")
        );
    }

    #[test]
    fn test_parse_window() {
        assert!(matches!(
            parse_window("per_minute"),
            Ok(goose::quotas::QuotaWindow::PerMinute)
        ));
        assert!(matches!(
            parse_window("per_hour"),
            Ok(goose::quotas::QuotaWindow::PerHour)
        ));
        assert!(matches!(
            parse_window("per_day"),
            Ok(goose::quotas::QuotaWindow::PerDay)
        ));
        assert!(matches!(
            parse_window("absolute"),
            Ok(goose::quotas::QuotaWindow::Absolute)
        ));
        assert!(parse_window("unknown").is_err());
    }
}
