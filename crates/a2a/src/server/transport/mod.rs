//! Transport handlers for the A2A server.

mod jsonrpc_handler;

#[cfg(feature = "axum")]
mod axum_router;

pub use jsonrpc_handler::JsonRpcHandler;

#[cfg(feature = "axum")]
pub use axum_router::create_a2a_router;
