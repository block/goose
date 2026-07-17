//! Error types for the roaming transport.

use thiserror::Error;

/// Errors produced by the roaming subsystem.
#[derive(Debug, Error)]
pub enum RoamingError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("identity error: {0}")]
    Identity(String),

    #[error("invite error: {0}")]
    Invite(String),

    #[error("connection rejected: {0}")]
    Rejected(String),

    #[error("transport error: {0}")]
    Transport(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// A reason a peer was refused at connection-accept time. Kept coarse on the
/// wire so we never disclose *why* a specific key was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// Peer's key is not on the allowlist.
    NotAllowlisted,
    /// Peer's key has been revoked.
    Revoked,
    /// The presented invite/capability is invalid or expired.
    InvalidCapability,
}

impl RejectReason {
    pub fn code(self) -> &'static str {
        match self {
            RejectReason::NotAllowlisted => "not_allowlisted",
            RejectReason::Revoked => "revoked",
            RejectReason::InvalidCapability => "invalid_capability",
        }
    }
}
