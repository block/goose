use crate::routes::errors::ErrorResponse;
use axum::http::StatusCode;
use goose::agents::{Agent, ExtensionLoadResult};
use goose::config::Config;
use goose::model::ModelConfig;
use goose::providers::create;
use goose::session::Session;
use std::sync::Arc;
use tracing::error;

pub async fn restore_agent_provider(
    agent: &Arc<Agent>,
    session: &Session,
    session_id: &str,
) -> Result<(), ErrorResponse> {
    let config = Config::global();
    let provider_name = session
        .provider_name
        .clone()
        .or_else(|| config.get_goose_provider().ok())
        .ok_or_else(|| ErrorResponse {
            message: "Could not configure agent: missing provider".into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let model_config = match session.model_config.clone() {
        Some(saved_config) => saved_config,
        None => {
            let model_name = config.get_goose_model().map_err(|_| ErrorResponse {
                message: "Could not configure agent: missing model".into(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
            ModelConfig::new(&model_name).map_err(|e| ErrorResponse {
                message: format!("Could not configure agent: invalid model {}", e),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            })?
        }
    };

    let provider = create(&provider_name, model_config)
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Could not create provider: {}", e),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    agent
        .update_provider(provider, session_id)
        .await
        .map_err(|e| ErrorResponse {
            message: format!("Could not configure agent: {}", e),
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
