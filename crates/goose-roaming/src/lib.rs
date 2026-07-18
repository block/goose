//! Peer-to-peer roaming transport for goose agents.
//!
//! This crate lets a goose agent expose itself over the internet via
//! [iroh](https://iroh.computer) so that a remote ACP client (another goose,
//! the desktop app, or any ACP client) can connect to and drive it through a
//! relay, with no open ports.
//!
//! # Building blocks
//!
//! * [`RoamingIdentity`] — a persisted ed25519 node key whose public half is
//!   the iroh endpoint id (self-certifying at the QUIC-TLS handshake).
//! * [`SignedInvite`] — a signed, TTL'd capability token carrying the host's
//!   endpoint id + relay URLs + a [`Scope`], optionally bound to specific
//!   client keys.
//! * [`TrustBook`] — local allow/revoke state and single-use redemption
//!   tracking.
//! * [`RoamingNode`] — owns the iroh endpoint + router, hosts agents over the
//!   `goose-acp/1` ALPN, and dials remote agents.
//!
//! The crate deliberately knows nothing about goose's agent internals: hosting
//! is driven through the [`AcpStreamServer`] trait, which the integration layer
//! implements by calling goose's generic `acp::server::serve`. This keeps the
//! heavy iroh dependency out of the `goose` core crate entirely.

mod broker;
mod directory;
mod error;
mod frame;
mod handshake;
mod identity;
mod invite;
mod node;
mod peerbook;
mod relay;
mod trust;

pub use broker::{Refused, Role, Route, Router, SessionBroker, SubscriberId};
pub use directory::{Direction, Directory, PeerEntry};
#[doc(inline)]
pub use iroh::EndpointId;

/// Parse an [`EndpointId`] (a peer's public key) from its string form.
pub fn parse_endpoint_id(s: &str) -> Result<EndpointId, RoamingError> {
    s.parse()
        .map_err(|e| RoamingError::Identity(format!("invalid endpoint id `{s}`: {e}")))
}
pub use error::{RejectReason, RoamingError};
pub use handshake::{ClientHello, HostAck};
pub use identity::{default_key_path, RoamingIdentity};
pub use invite::{InviteClaims, Scope, SignedInvite};
pub use node::{
    AcpStreamServer, InviteOptions, RoamingClientStream, RoamingConfig, RoamingNode,
    ROAMING_ACP_ALPN,
};
pub use peerbook::{PeerBook, PeerRecord};
pub use relay::{RelayEntry, RelaySettings};
pub use trust::{TrustBook, TrustPolicy};
