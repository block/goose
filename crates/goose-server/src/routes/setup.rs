use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use goose::config::signup_openrouter::OpenRouterAuth;
use goose::config::{configure_openrouter, Config};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

// Global mutex to ensure only one OAuth flow at a time
static OAUTH_FLOW_MUTEX: Lazy<Arc<Mutex<()>>> = Lazy::new(|| Arc::new(Mutex::new(())));

#[derive(Serialize)]
pub struct SetupResponse {
    pub success: bool,
    pub message: String,
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/setup/openrouter/start", post(start_openrouter_setup))
        .with_state(state)
}

async fn start_openrouter_setup(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<SetupResponse>, StatusCode> {
    tracing::info!("Starting OpenRouter setup flow");

    // Try to acquire the mutex with a timeout to prevent concurrent OAuth flows
    let _lock = match tokio::time::timeout(
        std::time::Duration::from_secs(1),
        OAUTH_FLOW_MUTEX.lock(),
    )
    .await
    {
        Ok(lock) => lock,
        Err(_) => {
            tracing::warn!("OAuth flow is already in progress");
            return Ok(Json(SetupResponse {
                success: false,
                message: "Authentication flow is already in progress. Please wait.".to_string(),
            }));
        }
    };

    tracing::info!("Acquired OAuth flow lock");

    // Run the existing PKCE flow
    let mut auth_flow = OpenRouterAuth::new().map_err(|e| {
        tracing::error!("Failed to initialize auth flow: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Auth flow initialized, starting complete_flow");

    match auth_flow.complete_flow().await {
        Ok(api_key) => {
            // The complete_flow only returns the API key, we need to save the configuration
            tracing::info!("Got API key, configuring OpenRouter...");

            // Configure everything using the common function
            let config = Config::global();

            // Use the common configuration function
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_oauth_flow_mutex() {
        // Test that the OAuth flow mutex is properly initialized and prevents concurrent flows
        let lock1 = OAUTH_FLOW_MUTEX.try_lock();
        assert!(lock1.is_ok(), "First lock should succeed");

        // Try to acquire second lock while first is held
        let lock2_result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            OAUTH_FLOW_MUTEX.lock(),
        )
        .await;

        assert!(
            lock2_result.is_err(),
            "Second lock should timeout while first is held"
        );

        // Drop first lock
        drop(lock1);

        // Now second lock should succeed
        let lock2 = OAUTH_FLOW_MUTEX.try_lock();
        assert!(
            lock2.is_ok(),
            "Second lock should succeed after first is dropped"
        );
    }
}
