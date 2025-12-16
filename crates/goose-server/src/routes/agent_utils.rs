use crate::routes::errors::ErrorResponse;
use axum::http::StatusCode;
use goose::agents::Agent;
use goose::config::Config;
use goose::model::ModelConfig;
use goose::providers::create;
use goose::session::extension_data::ExtensionState;
use goose::session::{EnabledExtensionsState, Session, SessionManager};
use std::sync::Arc;
use tracing::{error, warn};

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

pub async fn restore_agent_extensions(
    agent: Arc<Agent>,
    session: &Session,
) -> Result<(), ErrorResponse> {
    // Set the agent's working directory before adding extensions
    agent.set_working_dir(session.working_dir.clone()).await;

    // Try to load session-specific extensions first, fall back to global config
    let enabled_configs = EnabledExtensionsState::from_extension_data(&session.extension_data)
        .map(|state| state.extensions)
        .unwrap_or_else(goose::config::get_enabled_extensions);

    let extension_futures = enabled_configs
        .into_iter()
        .map(|config| {
            let config_clone = config.clone();
            let agent_ref = agent.clone();

            async move {
                if let Err(e) = agent_ref.add_extension(config_clone.clone()).await {
                    warn!("Failed to load extension {}: {}", config_clone.name(), e);
                }
                Ok::<_, ErrorResponse>(())
            }
        })
        .collect::<Vec<_>>();

    futures::future::join_all(extension_futures).await;
    Ok(())
}

pub async fn persist_session_extensions(
    agent: &Arc<Agent>,
    session_id: &str,
) -> Result<(), ErrorResponse> {
    let current_extensions = agent.extension_manager.get_extension_configs().await;
    let extensions_state = EnabledExtensionsState::new(current_extensions);

    // Get the current session to access its extension_data
    let session = SessionManager::get_session(session_id, false)
        .await
        .map_err(|e| {
            error!("Failed to get session for persisting extensions: {}", e);
            ErrorResponse {
                message: format!("Failed to get session: {}", e),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    let mut extension_data = session.extension_data.clone();
    extensions_state
        .to_extension_data(&mut extension_data)
        .map_err(|e| {
            error!("Failed to serialize extension state: {}", e);
            ErrorResponse {
                message: format!("Failed to serialize extension state: {}", e),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    SessionManager::update_session(session_id)
        .extension_data(extension_data)
        .apply()
        .await
        .map_err(|e| {
            error!("Failed to persist extension state: {}", e);
            ErrorResponse {
                message: format!("Failed to persist extension state: {}", e),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(())
}
