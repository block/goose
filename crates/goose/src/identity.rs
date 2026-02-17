//! Dual Identity System for Goose agents and users.
//!
//! Every action in Goose carries two identities:
//! - **UserIdentity**: who initiated the request (human or system)
//! - **AgentIdentity**: which agent instance is executing
//!
//! Agent IDs never hide user IDs — both are always available for tracing.
//! By default, Goose runs with a guest user. OAuth/SAML can be added later.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Identity of the human or system user who initiated a request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserIdentity {
    /// Stable user identifier (UUID for guest, provider-specific for OAuth/SAML)
    pub id: String,
    /// Display name
    pub name: String,
    /// Authentication method used
    pub auth_method: AuthMethod,
    /// Optional tenant/organization for multi-tenant deployments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// How the user authenticated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// No authentication — local single-user mode (default)
    Guest,
    /// OAuth 2.0 / OpenID Connect (Google, Azure, GitHub, etc.)
    Oidc { provider: String, subject: String },
    /// API key authentication
    ApiKey,
    /// Service-to-service (internal agent spawning another agent)
    ServiceAccount { service_name: String },
}

/// Identity of an agent instance executing work.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentIdentity {
    /// Unique instance ID (generated per spawn/session)
    pub id: String,
    /// Agent kind/type (e.g., "developer", "qa", "security", "goose")
    pub kind: String,
    /// Human-readable persona name (e.g., "Developer Agent", "QA Agent")
    pub persona: String,
    /// Parent agent ID if this is a sub-agent (for compound execution tracing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// The user who initiated the chain of execution
    pub spawned_by: String,
}

/// Combined execution context carrying both identities.
/// This is threaded through every execution path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionIdentity {
    pub user: UserIdentity,
    pub agent: AgentIdentity,
}

impl UserIdentity {
    /// Create a guest user identity (default, no auth required).
    pub fn guest() -> Self {
        Self {
            id: format!("guest-{}", Uuid::new_v4()),
            name: "Guest".to_string(),
            auth_method: AuthMethod::Guest,
            tenant: None,
        }
    }

    /// Create a guest user with a stable ID (for session persistence).
    pub fn guest_stable(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: "Guest".to_string(),
            auth_method: AuthMethod::Guest,
            tenant: None,
        }
    }

    /// Create an OIDC-authenticated user identity.
    pub fn oidc(
        subject: impl Into<String>,
        name: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        let subject = subject.into();
        let provider = provider.into();
        Self {
            id: format!("oidc-{}-{}", provider, subject),
            name: name.into(),
            auth_method: AuthMethod::Oidc { provider, subject },
            tenant: None,
        }
    }

    pub fn with_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = Some(tenant.into());
        self
    }

    pub fn is_guest(&self) -> bool {
        matches!(self.auth_method, AuthMethod::Guest)
    }
}

impl AgentIdentity {
    /// Create a new agent identity for a fresh instance.
    pub fn new(kind: impl Into<String>, persona: impl Into<String>, user_id: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind: kind.into(),
            persona: persona.into(),
            parent_id: None,
            spawned_by: user_id.to_string(),
        }
    }

    /// Create a sub-agent identity (child of another agent).
    pub fn sub_agent(
        kind: impl Into<String>,
        persona: impl Into<String>,
        parent: &AgentIdentity,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind: kind.into(),
            persona: persona.into(),
            parent_id: Some(parent.id.clone()),
            spawned_by: parent.spawned_by.clone(),
        }
    }
}

impl ExecutionIdentity {
    pub fn new(user: UserIdentity, agent: AgentIdentity) -> Self {
        Self { user, agent }
    }

    /// Create a default guest execution identity.
    pub fn guest(agent_kind: &str, agent_persona: &str) -> Self {
        let user = UserIdentity::guest();
        let agent = AgentIdentity::new(agent_kind, agent_persona, &user.id);
        Self { user, agent }
    }

    /// Convert to A2A-compatible metadata map for message propagation.
    pub fn to_a2a_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "goose_user_id": self.user.id,
            "goose_user_name": self.user.name,
            "goose_auth_method": self.user.auth_method,
            "goose_agent_id": self.agent.id,
            "goose_agent_kind": self.agent.kind,
            "goose_agent_persona": self.agent.persona,
            "goose_agent_parent_id": self.agent.parent_id,
            "goose_spawned_by": self.agent.spawned_by,
        })
    }

    /// Extract from A2A metadata map (reverse of to_a2a_metadata).
    pub fn from_a2a_metadata(meta: &serde_json::Value) -> Option<Self> {
        let user_id = meta.get("goose_user_id")?.as_str()?;
        let user_name = meta
            .get("goose_user_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let agent_id = meta.get("goose_agent_id")?.as_str()?;
        let agent_kind = meta.get("goose_agent_kind")?.as_str()?;
        let agent_persona = meta.get("goose_agent_persona")?.as_str()?;
        let parent_id = meta
            .get("goose_agent_parent_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        let spawned_by = meta
            .get("goose_spawned_by")
            .and_then(|v| v.as_str())
            .unwrap_or(user_id);

        Some(Self {
            user: UserIdentity {
                id: user_id.to_string(),
                name: user_name.to_string(),
                auth_method: serde_json::from_value(
                    meta.get("goose_auth_method").cloned().unwrap_or_default(),
                )
                .unwrap_or(AuthMethod::Guest),
                tenant: None,
            },
            agent: AgentIdentity {
                id: agent_id.to_string(),
                kind: agent_kind.to_string(),
                persona: agent_persona.to_string(),
                parent_id,
                spawned_by: spawned_by.to_string(),
            },
        })
    }
}

impl fmt::Display for UserIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.auth_method {
            AuthMethod::Guest => write!(f, "guest:{}", self.id),
            AuthMethod::Oidc { provider, .. } => write!(f, "oidc:{}:{}", provider, self.name),
            AuthMethod::ApiKey => write!(f, "apikey:{}", self.id),
            AuthMethod::ServiceAccount { service_name } => {
                write!(f, "service:{}", service_name)
            }
        }
    }
}

impl fmt::Display for AgentIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let short_id: String = self.id.chars().take(8).collect();
        write!(f, "{}:{}", self.kind, short_id)
    }
}

impl fmt::Display for ExecutionIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[user={} agent={}]", self.user, self.agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guest_user() {
        let user = UserIdentity::guest();
        assert!(user.is_guest());
        assert!(user.id.starts_with("guest-"));
        assert_eq!(user.name, "Guest");
        assert!(user.tenant.is_none());
    }

    #[test]
    fn test_guest_stable() {
        let user = UserIdentity::guest_stable("user-123");
        assert_eq!(user.id, "user-123");
        assert!(user.is_guest());
    }

    #[test]
    fn test_oidc_user() {
        let user = UserIdentity::oidc("sub-456", "Jane Doe", "google");
        assert!(!user.is_guest());
        assert_eq!(user.id, "oidc-google-sub-456");
        assert_eq!(user.name, "Jane Doe");
        match &user.auth_method {
            AuthMethod::Oidc { provider, subject } => {
                assert_eq!(provider, "google");
                assert_eq!(subject, "sub-456");
            }
            _ => panic!("Expected OIDC auth method"),
        }
    }

    #[test]
    fn test_user_with_tenant() {
        let user = UserIdentity::guest().with_tenant("acme-corp");
        assert_eq!(user.tenant, Some("acme-corp".to_string()));
    }

    #[test]
    fn test_agent_identity() {
        let agent = AgentIdentity::new("developer", "Developer Agent", "user-123");
        assert_eq!(agent.kind, "developer");
        assert_eq!(agent.persona, "Developer Agent");
        assert_eq!(agent.spawned_by, "user-123");
        assert!(agent.parent_id.is_none());
        assert!(!agent.id.is_empty());
    }

    #[test]
    fn test_sub_agent_identity() {
        let parent = AgentIdentity::new("orchestrator", "Meta-Orchestrator", "user-123");
        let child = AgentIdentity::sub_agent("developer", "Developer Agent", &parent);
        assert_eq!(child.parent_id, Some(parent.id.clone()));
        assert_eq!(child.spawned_by, "user-123"); // preserves original user
        assert_ne!(child.id, parent.id);
    }

    #[test]
    fn test_execution_identity_guest() {
        let ident = ExecutionIdentity::guest("developer", "Developer Agent");
        assert!(ident.user.is_guest());
        assert_eq!(ident.agent.kind, "developer");
        assert_eq!(ident.agent.spawned_by, ident.user.id);
    }

    #[test]
    fn test_a2a_metadata_roundtrip() {
        let ident = ExecutionIdentity::guest("qa", "QA Agent");
        let meta = ident.to_a2a_metadata();

        let recovered = ExecutionIdentity::from_a2a_metadata(&meta).unwrap();
        assert_eq!(recovered.user.id, ident.user.id);
        assert_eq!(recovered.agent.id, ident.agent.id);
        assert_eq!(recovered.agent.kind, "qa");
        assert_eq!(recovered.agent.persona, "QA Agent");
        assert_eq!(recovered.agent.spawned_by, ident.user.id);
    }

    #[test]
    fn test_display_formats() {
        let user = UserIdentity::guest_stable("guest-abc");
        assert_eq!(format!("{}", user), "guest:guest-abc");

        let oidc_user = UserIdentity::oidc("sub-1", "Alice", "google");
        assert_eq!(format!("{}", oidc_user), "oidc:google:Alice");

        let agent = AgentIdentity {
            id: "12345678-abcd-efgh".to_string(),
            kind: "developer".to_string(),
            persona: "Dev Agent".to_string(),
            parent_id: None,
            spawned_by: "user-1".to_string(),
        };
        assert_eq!(format!("{}", agent), "developer:12345678");
    }

    #[test]
    fn test_serde_roundtrip() {
        let ident = ExecutionIdentity::guest("developer", "Developer Agent");
        let json = serde_json::to_string(&ident).unwrap();
        let recovered: ExecutionIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, ident);
    }
}
