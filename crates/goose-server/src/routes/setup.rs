use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use goose::config::signup_openrouter::OpenRouterAuth;
use goose::config::{configure_openrouter, Config};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub struct SetupResponse {
    pub success: bool,
    pub message: String,
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/handle_openrouter", post(start_openrouter_setup))
        .with_state(state)
}

async fn start_openrouter_setup(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<SetupResponse>, StatusCode> {
    tracing::info!("Starting OpenRouter setup flow");

    let mut auth_flow = OpenRouterAuth::new().map_err(|e| {
        tracing::error!("Failed to initialize auth flow: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Auth flow initialized, starting complete_flow");

    match auth_flow.complete_flow().await {
        Ok(api_key) => {
            tracing::info!("Got API key, configuring OpenRouter...");

            let config = Config::global();

            if let Err(e) = configure_openrouter(config, api_key) {
                tracing::error!("Failed to configure OpenRouter: {}", e);
                return Ok(Json(SetupResponse {
                    success: false,
                    message: format!("Failed to configure OpenRouter: {}", e),
                }));
            }

            tracing::info!("OpenRouter setup completed successfully");
            Ok(Json(SetupResponse {
                success: true,
                message: "OpenRouter setup completed successfully".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("OpenRouter setup failed: {}", e);
            Ok(Json(SetupResponse {
                success: false,
                message: format!("Setup failed: {}", e),
            }))
        }
    }
}
