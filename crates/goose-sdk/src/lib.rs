//! Goose SDK.
//!
//! With default features this crate re-exports the shared SDK wire types from
//! `goose-sdk-types` so you can build an Agent Client Protocol (ACP) client
//! that talks to `goose acp` over stdio.
//!
//! With `--features uniffi` the crate additionally compiles as a
//! `cdylib`/`staticlib` and exposes a small in-process API to Python and Kotlin
//! via [uniffi-rs](https://github.com/mozilla/uniffi-rs).
//!
//! The published uniffi surface is intentionally a single `ping` -> `pong`
//! round-trip. It exists as a working scaffold for adding the real Goose SDK
//! API: replace [`bindings`] with the actual implementation.

pub use goose_sdk_types::{custom_notifications, custom_requests};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!("aaif_goose");

#[cfg(feature = "uniffi")]
pub mod bindings;
