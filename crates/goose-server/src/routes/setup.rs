use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::{routing::post, Json, Router};
use goose::config::signup_nanogpt::{complete_nanogpt_auth, configure_nanogpt};
use goose::config::signup_openrouter::OpenRouterAuth;
use goose::config::signup_tetrate::{configure_tetrate, TetrateAuth};
use goose::config::{configure_openrouter, Config};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct SetupResponse {
    pub success: bool,
    pub message: String,
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/handle_openrouter", post(start_openrouter_setup))
        .route("/handle_tetrate", post(start_tetrate_setup))
        .route("/handle_nanogpt", post(start_nanogpt_setup))
        .with_state(state)
}

#[utoipa::path(
    post,
    path = "/handle_openrouter",
    responses(
        (status = 200, body=SetupResponse)
    ),
)]
async fn start_openrouter_setup() -> Result<Json<SetupResponse>, ErrorResponse> {
    let mut auth_flow = OpenRouterAuth::new()
        .map_err(|e| ErrorResponse::internal(format!("Failed to initialize auth flow: {}", e)))?;

    match auth_flow.complete_flow().await {
        Ok(api_key) => {
            let config = Config::global();

            if let Err(e) = configure_openrouter(config, api_key) {
                return Ok(Json(SetupResponse {
                    success: false,
                    message: format!("Failed to configure OpenRouter: {}", e),
                }));
            }

            Ok(Json(SetupResponse {
                success: true,
                message: "OpenRouter setup completed successfully".to_string(),
            }))
        }
        Err(e) => Ok(Json(SetupResponse {
            success: false,
            message: format!("Setup failed: {}", e),
        })),
    }
}

#[utoipa::path(
    post,
    path = "/handle_tetrate",
    responses(
        (status = 200, body=SetupResponse)
    ),
)]
async fn start_tetrate_setup() -> Result<Json<SetupResponse>, ErrorResponse> {
    let mut auth_flow = TetrateAuth::new()
        .map_err(|e| ErrorResponse::internal(format!("Failed to initialize auth flow: {}", e)))?;

    match auth_flow.complete_flow().await {
        Ok(api_key) => {
            let config = Config::global();

            if let Err(e) = configure_tetrate(config, api_key) {
                return Ok(Json(SetupResponse {
                    success: false,
                    message: format!("Failed to configure Tetrate Agent Router Service: {}", e),
                }));
            }

            Ok(Json(SetupResponse {
                success: true,
                message: "Tetrate Agent Router Service setup completed successfully".to_string(),
            }))
        }
        Err(e) => Ok(Json(SetupResponse {
            success: false,
            message: format!("Setup failed: {}", e),
        })),
    }
}

#[utoipa::path(
    post,
    path = "/handle_nanogpt",
    responses(
        (status = 200, body=SetupResponse)
    ),
)]
async fn start_nanogpt_setup() -> Result<Json<SetupResponse>, ErrorResponse> {
    match complete_nanogpt_auth().await {
        Ok(api_key) => {
            let config = Config::global();

            if let Err(e) = configure_nanogpt(config, api_key) {
                return Ok(Json(SetupResponse {
                    success: false,
                    message: format!("Failed to configure NanoGPT: {}", e),
                }));
            }

            Ok(Json(SetupResponse {
                success: true,
                message: "NanoGPT setup completed successfully".to_string(),
            }))
        }
        Err(e) => Ok(Json(SetupResponse {
            success: false,
            message: format!("Setup failed: {}", e),
        })),
    }
}
