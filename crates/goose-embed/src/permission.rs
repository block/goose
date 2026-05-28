//! Permission handling for embedded agents.
//!
//! When a tool call needs confirmation, goose's agent loop pauses and waits for
//! a decision. In the CLI this is a TUI prompt; in the desktop app it's a
//! modal. For an embedded agent, the host program supplies a
//! [`PermissionDecider`] that decides programmatically.

use async_trait::async_trait;

use goose::permission::permission_confirmation::PrincipalType;
pub use goose::permission::{Permission, PermissionConfirmation};

/// A pending permission request surfaced to the embedder.
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub request_id: String,
    pub tool_name: String,
    pub security_prompt: Option<String>,
}

/// Decides what permission to grant for a tool call.
///
/// Embedders implement this trait to decide programmatically. Two ready-made
/// impls are provided: [`AutoApprove`] (grants `AllowOnce` for everything) and
/// [`DenyAll`] (denies everything).
///
/// Implementations must be `Send + Sync` — the decider is called from inside
/// the stream wrapper returned by [`crate::Goose::reply`], which may be polled
/// on any executor thread.
#[async_trait]
pub trait PermissionDecider: Send + Sync {
    async fn decide(&self, request: PermissionRequest) -> Permission;
}

/// Grants `AllowOnce` for every tool call. Useful for trusted automation
/// pipelines, tests, and the smallest possible embed example.
///
/// Do not use in adversarial or multi-tenant settings — every tool call will
/// be approved, including ones the model invented on its own.
pub struct AutoApprove;

#[async_trait]
impl PermissionDecider for AutoApprove {
    async fn decide(&self, _request: PermissionRequest) -> Permission {
        Permission::AllowOnce
    }
}

/// Denies every tool call. Useful as a safety default while wiring up a real
/// decision policy.
pub struct DenyAll;

#[async_trait]
impl PermissionDecider for DenyAll {
    async fn decide(&self, _request: PermissionRequest) -> Permission {
        Permission::DenyOnce
    }
}

pub(crate) fn confirmation_for(permission: Permission) -> PermissionConfirmation {
    PermissionConfirmation {
        principal_type: PrincipalType::Tool,
        permission,
    }
}
