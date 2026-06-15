//! In-process uniffi bindings for the Goose SDK.
//!
//! This is the published API surface exposed to Python and Kotlin. Right now it
//! is a minimal `ping` -> `pong` round-trip that proves the uniffi
//! infrastructure end to end without depending on the `goose` core crate.
//!
//! To build the real SDK, add `goose` (and whatever else you need) as
//! dependencies and replace the [`Client`] methods below with the actual
//! agent surface.

use std::sync::Arc;

/// Errors surfaced across the uniffi boundary.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum GooseError {
    #[error("{0}")]
    Generic(String),
}

/// A reply to a [`Client::ping`] call.
#[derive(Debug, Clone, uniffi::Record)]
pub struct Pong {
    /// Echo of the message that was pinged.
    pub message: String,
}

/// The top-level entry point for the Goose SDK.
///
/// This is the object that consuming languages instantiate. Today it only knows
/// how to answer a ping; extend it with the real agent API.
#[derive(uniffi::Object)]
pub struct Client {}

#[uniffi::export]
impl Client {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    /// Round-trip a message through the SDK. Returns a [`Pong`] echoing the
    /// supplied `message`, prefixed with `pong: `.
    pub fn ping(&self, message: String) -> Result<Pong, GooseError> {
        if message.is_empty() {
            return Err(GooseError::Generic("message must not be empty".into()));
        }
        Ok(Pong {
            message: format!("pong: {message}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_returns_pong() {
        let client = Client::new();
        let pong = client.ping("aaif.io".into()).expect("ping should succeed");
        assert_eq!(pong.message, "pong: aaif.io");
    }

    #[test]
    fn empty_ping_errors() {
        let client = Client::new();
        assert!(client.ping(String::new()).is_err());
    }
}
