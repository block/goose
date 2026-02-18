use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::identity::{AgentIdentity, UserIdentity};
use crate::policy::PolicyDecision;

/// Who performed the action.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum AuditActor {
    User {
        id: String,
        name: String,
    },
    Agent {
        id: String,
        kind: String,
        persona: String,
    },
    System,
}

impl From<&UserIdentity> for AuditActor {
    fn from(user: &UserIdentity) -> Self {
        AuditActor::User {
            id: user.id.clone(),
            name: user.name.clone(),
        }
    }
}

impl From<&AgentIdentity> for AuditActor {
    fn from(agent: &AgentIdentity) -> Self {
        AuditActor::Agent {
            id: agent.id.clone(),
            kind: agent.kind.clone(),
            persona: agent.persona.clone(),
        }
    }
}

/// What happened.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Denied { reason: String },
    Error { message: String },
}

/// A structured audit event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub actor: AuditActor,
    pub action: String,
    pub resource: String,
    pub outcome: AuditOutcome,
    pub tenant: Option<String>,
    pub parent_event_id: Option<String>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl AuditEvent {
    pub fn new(actor: AuditActor, action: impl Into<String>, resource: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            actor,
            action: action.into(),
            resource: resource.into(),
            outcome: AuditOutcome::Success,
            tenant: None,
            parent_event_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    pub fn with_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = Some(tenant.into());
        self
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_event_id = Some(parent_id.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn from_policy_decision(
        actor: AuditActor,
        action: impl Into<String>,
        resource: impl Into<String>,
        decision: &PolicyDecision,
    ) -> Self {
        let action = action.into();
        let resource = resource.into();
        let outcome = match decision {
            PolicyDecision::Allow => AuditOutcome::Success,
            PolicyDecision::Deny { reason } => AuditOutcome::Denied {
                reason: reason.clone(),
            },
            PolicyDecision::Abstain => AuditOutcome::Denied {
                reason: "no matching policy rule".to_string(),
            },
        };
        Self::new(actor, action, resource).with_outcome(outcome)
    }
}

/// Trait for audit event sinks.
#[async_trait::async_trait]
pub trait AuditSink: Send + Sync {
    async fn emit(&self, event: &AuditEvent);
}

/// Logs events via the tracing crate.
pub struct TracingSink;

#[async_trait::async_trait]
impl AuditSink for TracingSink {
    async fn emit(&self, event: &AuditEvent) {
        if let Ok(json) = serde_json::to_string(event) {
            tracing::info!(target: "audit", "{}", json);
        }
    }
}

/// Broadcasts events to subscribers (for real-time monitoring).
pub struct BroadcastSink {
    sender: broadcast::Sender<AuditEvent>,
}

impl BroadcastSink {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AuditEvent> {
        self.sender.subscribe()
    }
}

#[async_trait::async_trait]
impl AuditSink for BroadcastSink {
    async fn emit(&self, event: &AuditEvent) {
        let _ = self.sender.send(event.clone());
    }
}

/// In-memory sink for testing.
pub struct InMemorySink {
    events: Arc<RwLock<Vec<AuditEvent>>>,
}

impl InMemorySink {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn events(&self) -> Vec<AuditEvent> {
        self.events.read().await.clone()
    }
}

impl Default for InMemorySink {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AuditSink for InMemorySink {
    async fn emit(&self, event: &AuditEvent) {
        self.events.write().await.push(event.clone());
    }
}

/// Multi-sink audit logger.
pub struct AuditLogger {
    sinks: Vec<Arc<dyn AuditSink>>,
    memory_sink: Arc<InMemorySink>,
}

impl AuditLogger {
    pub fn new() -> Self {
        let memory_sink = Arc::new(InMemorySink::new());
        Self {
            sinks: vec![Arc::new(TracingSink), memory_sink.clone()],
            memory_sink,
        }
    }

    pub async fn recent_events(&self, max: usize) -> Vec<AuditEvent> {
        let all = self.memory_sink.events().await;
        let start = all.len().saturating_sub(max);
        all[start..].to_vec()
    }

    pub fn with_sink(mut self, sink: Arc<dyn AuditSink>) -> Self {
        self.sinks.push(sink);
        self
    }

    pub async fn log(&self, event: &AuditEvent) {
        for sink in &self.sinks {
            sink.emit(event).await;
        }
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::AuthMethod;

    fn test_user() -> UserIdentity {
        UserIdentity {
            id: "user-1".to_string(),
            name: "Alice".to_string(),
            auth_method: AuthMethod::Oidc {
                provider: "google".to_string(),
                subject: "alice@example.com".to_string(),
            },
            tenant: Some("acme".to_string()),
            roles: vec![],
        }
    }

    fn test_agent(user: &UserIdentity) -> AgentIdentity {
        AgentIdentity::new("developer", "Dev Agent", &user.id)
    }

    #[test]
    fn test_event_creation() {
        let user = test_user();
        let event = AuditEvent::new(AuditActor::from(&user), "execute:agent", "agent:developer");
        assert_eq!(event.action, "execute:agent");
        assert_eq!(event.resource, "agent:developer");
        assert!(matches!(event.outcome, AuditOutcome::Success));
    }

    #[test]
    fn test_event_serialization() {
        let user = test_user();
        let event = AuditEvent::new(AuditActor::from(&user), "execute:agent", "agent:developer")
            .with_tenant("acme");
        let json = serde_json::to_string(&event).unwrap();
        let parsed: AuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.action, event.action);
        assert_eq!(parsed.tenant, Some("acme".to_string()));
    }

    #[test]
    fn test_event_with_metadata() {
        let user = test_user();
        let event = AuditEvent::new(AuditActor::from(&user), "execute:agent", "agent:developer")
            .with_metadata("model", serde_json::json!("gpt-4o"))
            .with_metadata("tokens", serde_json::json!(1500));
        assert_eq!(event.metadata.len(), 2);
        assert_eq!(event.metadata["model"], serde_json::json!("gpt-4o"));
    }

    #[test]
    fn test_parent_child_chain() {
        let user = test_user();
        let parent = AuditEvent::new(
            AuditActor::from(&user),
            "execute:compound",
            "agent:orchestrator",
        );
        let child = AuditEvent::new(
            AuditActor::from(&test_agent(&user)),
            "execute:agent",
            "agent:developer",
        )
        .with_parent(&parent.id);
        assert_eq!(child.parent_event_id, Some(parent.id.clone()));
    }

    #[test]
    fn test_policy_decision_integration() {
        let user = test_user();
        let deny = PolicyDecision::Deny {
            reason: "insufficient permissions".to_string(),
        };
        let event = AuditEvent::from_policy_decision(
            AuditActor::from(&user),
            "manage:agents",
            "agent:x",
            &deny,
        );
        assert!(matches!(event.outcome, AuditOutcome::Denied { .. }));
    }

    #[tokio::test]
    async fn test_in_memory_sink() {
        let sink = Arc::new(InMemorySink::new());
        let logger = AuditLogger::new().with_sink(sink.clone());
        let user = test_user();
        let event = AuditEvent::new(AuditActor::from(&user), "execute:agent", "agent:developer");
        logger.log(&event).await;
        let events = sink.events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "execute:agent");
    }

    #[tokio::test]
    async fn test_multi_sink() {
        let sink1 = Arc::new(InMemorySink::new());
        let sink2 = Arc::new(InMemorySink::new());
        let logger = AuditLogger::new()
            .with_sink(sink1.clone())
            .with_sink(sink2.clone());
        let user = test_user();
        let event = AuditEvent::new(AuditActor::from(&user), "read:session", "session:123");
        logger.log(&event).await;
        assert_eq!(sink1.events().await.len(), 1);
        assert_eq!(sink2.events().await.len(), 1);
    }

    #[tokio::test]
    async fn test_broadcast_sink() {
        let sink = BroadcastSink::new(16);
        let mut rx = sink.subscribe();
        let user = test_user();
        let event = AuditEvent::new(AuditActor::from(&user), "execute:agent", "agent:dev");
        sink.emit(&event).await;
        let received = rx.recv().await.unwrap();
        assert_eq!(received.action, "execute:agent");
    }
}
