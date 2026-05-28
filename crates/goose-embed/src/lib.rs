//! # goose-embed
//!
//! Embeddable Rust SDK for the goose AI agent.
//!
//! Use this crate to build your own Rust program that runs goose's agent loop
//! with custom providers, MCP extensions, recipes, and skills, without
//! shelling out to the `goose` binary or talking ACP over stdio.
//!
//! ## Status — Spike
//!
//! This is an API-discovery spike. It is implemented as a thin facade over the
//! full [`goose`] crate, so depending on it pulls the same dependency tree as
//! depending on `goose` directly. The dependency-minimization story comes later,
//! when a slim `goose-core` crate is carved out and `goose-embed` is re-emerged
//! on top of it. See `docs/brainstorms/2026-05-28-goose-embed-rust-sdk.md` for
//! the roadmap.
//!
//! ## Quick start
//!
//! ```no_run
//! use futures::StreamExt;
//! use goose_embed::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let goose = Goose::builder()
//!         .provider("anthropic", "claude-sonnet-4")
//!         .working_dir(std::env::current_dir()?)
//!         .build()
//!         .await?;
//!
//!     let mut stream = goose.reply("What is 2 + 2?").await?;
//!     while let Some(event) = stream.next().await {
//!         if let Ok(AgentEvent::Message(message)) = event {
//!             println!("{}", message.as_concat_text());
//!         }
//!     }
//!     Ok(())
//! }
//! ```

pub mod builder;
pub mod permission;

mod handle;

pub use builder::{Goose, GooseBuilder};
pub use handle::ReplyStream;
pub use permission::{AutoApprove, DenyAll, PermissionDecider, PermissionRequest};

/// Re-exports of the goose types you'll most commonly need when wiring up an
/// embedded agent. Glob-import via `use goose_embed::prelude::*;`.
pub mod prelude {
    pub use crate::{
        AutoApprove, DenyAll, Goose, GooseBuilder, PermissionDecider, PermissionRequest,
        ReplyStream,
    };
    pub use goose::agents::{AgentEvent, ExtensionConfig};
    pub use goose::conversation::message::Message;
    pub use goose::permission::{Permission, PermissionConfirmation};
    pub use goose::recipe::Recipe;
}
