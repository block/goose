//! What an accepted peer is allowed to do once connected.
//!
//! The scope is decided by the **accepting** node per peer key (in its
//! allowlist), not carried in any token — trust is key-based and local.

use serde::{Deserialize, Serialize};

/// Capability granted to a connected peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Full ACP control: create/drive sessions and answer tool-permission
    /// prompts. This is effectively remote shell access to the host and should
    /// only be granted to trusted peers.
    #[default]
    Control,
    /// Attach to an existing live session and control it.
    Attach,
    /// Observe session activity without the ability to approve tool
    /// permissions or mutate state.
    Observe,
}
