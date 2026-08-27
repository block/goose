//! The goose Development Kit (GDK).
//!
//! With default features this crate re-exports the shared GDK wire types from
//! `goose-sdk-types` so you can build an Agent Client Protocol (ACP) client
//! that talks to `goose acp` over stdio.
//!
//! With `--features uniffi` the crate additionally compiles as a
//! `cdylib`/`staticlib` and exposes an in-process API to Python and Kotlin via
//! [uniffi-rs](https://github.com/mozilla/uniffi-rs). The uniffi surface
//! constructs native providers, streams completions, and runs host-driven
//! OAuth device-code login (protocol in `goose-providers`; the host owns UI
//! and token storage).

pub use goose_sdk_types::{custom_notifications, custom_requests};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!("goose");

#[cfg(feature = "uniffi")]
pub mod bindings;
