use crate::routes::errors::ErrorResponse;
use axum::http::StatusCode;
use goose::agents::{Agent, ExtensionLoadResult};
use goose::session::Session;
use std::sync::Arc;
use tracing::error;

/// Restore the provider from session into the agent
/// Delegates to Agent::restore_provider_from_session
pub async fn restore_agent_provider(
    agent: &Arc<Agent>,
    session: &Session,
) -> Result<(), ErrorResponse> {
    agent
        .restore_provider_from_session(session)
        .await
        .map_err(|e| ErrorResponse {
            message: e.to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })
}

/// Load extensions from session into the agent
/// Delegates to Agent::load_extensions_from_session
pub async fn restore_agent_extensions(
    agent: Arc<Agent>,
    session: &Session,
) -> Vec<ExtensionLoadResult> {
    agent.load_extensions_from_session(session).await
}

/// Persist current extension state to session
/// Delegates to Agent::persist_extension_state
pub async fn persist_session_extensions(
    agent: &Arc<Agent>,
    session_id: &str,
) -> Result<(), ErrorResponse> {
    agent
        .persist_extension_state(session_id)
        .await
        .map_err(|e| {
            error!("Failed to persist extension state: {}", e);
            ErrorResponse {
                message: format!("Failed to persist extension state: {}", e),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })
}
