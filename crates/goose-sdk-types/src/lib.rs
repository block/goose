//! Shared types for the Goose SDK.
//!
//! These wire types are used by both the ACP client/server path and the
//! in-process uniffi bindings, keeping a single source of truth for Goose's
//! custom `_goose/*` JSON-RPC methods.

pub mod custom_notifications;
pub mod custom_requests;
