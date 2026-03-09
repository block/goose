//! A2A (Agent-to-Agent) protocol routes.
//!
//! Mounts spec-compliant A2A endpoints:
//!
//! - `GET  /a2a/.well-known/agent-card.json` — Agent Card discovery
//! - `POST /a2a`                             — JSON-RPC 2.0 (message/send, tasks/*)
//! - `POST /a2a/stream`                      — SSE streaming (message/sendStream)
//!
//! Uses the A2A server transport from the `a2a` crate, bridged to Goose's
//! `Agent::reply()` via `GooseAgentExecutor`.

use std::sync::Arc;

use a2a::server::request_handler::DefaultRequestHandler;
use a2a::server::store::InMemoryTaskStore;
use a2a::server::transport::create_a2a_router;
use axum::Router;

use goose::a2a_compat::{build_agent_card, skill, GooseAgentExecutor};

use crate::state::AppState;

/// Build the A2A sub-router and nest it under `/a2a`.
///
/// The agent card's interface URL is set to `https://127.0.0.1/a2a`
/// as a default. Clients typically resolve the URL from the discovery
/// endpoint they used to fetch the card.
pub fn routes(state: Arc<AppState>) -> Router {
    let card = build_agent_card(
        "Goose",
        "A general-purpose AI agent powered by LLMs",
        "https://127.0.0.1/a2a",
        vec![skill(
            "general",
            "General-purpose task execution",
            vec!["ai".into(), "agent".into()],
        )],
    );
    let store = InMemoryTaskStore::new();
    let executor = GooseAgentExecutor::new(state.agent_manager.clone());
    let handler = DefaultRequestHandler::new(card, store, executor);

    Router::new().nest("/a2a", create_a2a_router(handler))
}

#[cfg(test)]
mod tests {
    use goose::a2a_compat::{build_agent_card, skill};

    #[test]
    fn agent_card_has_required_fields() {
        let card = build_agent_card(
            "Goose",
            "A general-purpose AI agent",
            "https://127.0.0.1:3000/a2a",
            vec![skill("coding", "Write and edit code", vec!["code".into()])],
        );
        assert_eq!(card.name, "Goose");
        assert!(!card.description.is_empty());
        assert!(!card.supported_interfaces.is_empty());
        assert_eq!(
            card.supported_interfaces[0].url,
            "https://127.0.0.1:3000/a2a"
        );
        assert!(card.capabilities.unwrap().streaming);
    }
}
