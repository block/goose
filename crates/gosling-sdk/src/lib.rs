//! Gosling SDK.
//!
//! With default features this crate re-exports the shared SDK wire types from
//! `gosling-sdk-types` so you can build an Agent Client Protocol (ACP) client
//! that talks to `gosling acp` over stdio.
//!
//! With `--features uniffi` the crate additionally compiles as a
//! `cdylib`/`staticlib` and exposes a small in-process API to Python and Kotlin
//! via [uniffi-rs](https://github.com/mozilla/uniffi-rs).
//!
//! The published uniffi surface is intentionally a single `ping` -> `pong`
//! round-trip. It exists as a working scaffold for adding the real Gosling SDK
//! API: replace [`bindings`] with the actual implementation.

pub use gosling_sdk_types::{custom_notifications, custom_requests};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!("gosling");

#[cfg(feature = "uniffi")]
pub mod bindings;
