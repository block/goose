use crate::routes::errors::ErrorResponse;
use axum::http::StatusCode;
use goose::agents::Agent;
use goose::config::Config;
use goose::model::ModelConfig;
use goose::providers::create;
use goose::session::Session;
use std::sync::Arc;
use tracing::warn;

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
    working_dir: &std::path::Path,
) -> Result<(), ErrorResponse> {
    let working_dir_buf = working_dir.to_path_buf();
    let enabled_configs = goose::config::get_enabled_extensions();
    let extension_futures = enabled_configs
        .into_iter()
        .map(|config| {
            let config_clone = config.clone();
            let agent_ref = agent.clone();
            let wd = working_dir_buf.clone();

            async move {
                if let Err(e) = agent_ref
                    .add_extension(config_clone.clone(), Some(wd))
                    .await
                {
                    warn!("Failed to load extension {}: {}", config_clone.name(), e);
                }
                Ok::<_, ErrorResponse>(())
            }
        })
        .collect::<Vec<_>>();

    futures::future::join_all(extension_futures).await;
    Ok(())
}
