//! The roaming handshake exchanged on a freshly-accepted bi-stream, before the
//! stream is handed to ACP.
//!
//! Flow:
//! 1. Client opens a bi-stream and sends [`ClientHello`] carrying the invite it
//!    is redeeming.
//! 2. Host verifies the invite (signature, validity, audience, client
//!    allowlist, revocation, single-use) against the *authenticated* remote id
//!    from the QUIC handshake, then replies with [`HostAck`].
//! 3. On accept, both sides treat the remainder of the stream as an ACP byte
//!    stream.

use serde::{Deserialize, Serialize};

use crate::invite::{Scope, SignedInvite};

/// First message a connecting client sends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHello {
    /// The invite the client is redeeming.
    pub invite: SignedInvite,
    /// Human-readable client label for the host's directory (best-effort, not
    /// trusted for authorization).
    pub label: Option<String>,
}

/// Host's response to a [`ClientHello`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostAck {
    /// Connection accepted with the granted scope.
    Accepted { scope: Scope, agent_id: String },
    /// Connection refused with a coarse reason code.
    Rejected { code: String },
}

impl ClientHello {
    pub fn new(invite: SignedInvite, label: Option<String>) -> Self {
        Self { invite, label }
    }
}
