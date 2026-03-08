//! A2A (Agent-to-Agent) Protocol library for Rust.
//!
//! Implements the A2A Protocol v1.0 RC as defined in the authoritative `a2a.proto` specification.
//! This crate provides types, client, and server components for building A2A-compliant agents.

pub mod error;
pub mod jsonrpc;
pub mod types;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

pub use error::A2AError;
pub use types::*;
