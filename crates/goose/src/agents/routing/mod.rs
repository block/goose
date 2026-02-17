//! Message routing — intent classification, persona selection, and mode dispatch.
//!
//! This module groups routing components that decide which agent/mode handles a message:
//! - `IntentRouter` — classifies user intent and routes to the right persona/agent
//! - `UniversalMode` — shared modes (ask, plan, write, review) available to all personas
//! - `RoutingEval` — evaluation framework for routing quality
//!
//! # Usage
//! ```rust,ignore
//! use goose::agents::routing::{IntentRouter, RoutingDecision, UniversalMode};
//! ```

pub use super::intent_router::{IntentRouter, RoutingDecision};
pub use super::routing_eval::{self, RoutingEvalCase, RoutingEvalSet};
pub use super::universal_mode::UniversalMode;
