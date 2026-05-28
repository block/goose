//! Trait abstractions that decouple `goose`'s embeddable primitives (currently
//! in `agents/`, `providers/`, `mcp_utils`, `skills`, `recipe`, `conversation`,
//! `context_mgmt`, `permission`) from the application-runtime pieces that will
//! stay in this crate (`session/`, `oauth/`, `scheduler/`, `posthog`, etc.).
//!
//! These traits are the contract that the future `goose-core` crate will export
//! and that `crates/goose` will keep implementing. Today, both live here — this
//! module is the first step of the carve-out series described in
//! `docs/brainstorms/2026-05-28-goose-core-carve-out.md`.

pub mod oauth;
pub mod session;

pub use oauth::{OAuthError, OAuthProvider};
pub use session::{SessionContextProvider, SessionUpdate};
