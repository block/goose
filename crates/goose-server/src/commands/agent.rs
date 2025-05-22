use std::sync::Arc;

use crate::configuration;
use crate::state;
use anyhow::Result;
use goose::agents::Agent;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

pub async fn run() -> Result<()> {
    // Initialize logging
    crate::logging::setup_logging(Some("goosed"))?;

    tracing::info!("Starting goosed server...");

    // Load configuration
    let settings = configuration::Settings::new()?;
    tracing::info!("Configuration loaded: {:?}", settings);

    // load secret key from GOOSE_SERVER__SECRET_KEY environment variable
    let secret_key =
        std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());
    
    tracing::info!("Secret key loaded (length: {})", secret_key.len());

    // Log environment variables related to provider configuration
    if let Ok(provider_type) = std::env::var("GOOSE_PROVIDER__TYPE") {
        tracing::info!("GOOSE_PROVIDER__TYPE: {}", provider_type);
    } else {
        tracing::warn!("GOOSE_PROVIDER__TYPE not set");
    }
    
    if let Ok(provider_host) = std::env::var("GOOSE_PROVIDER__HOST") {
        tracing::info!("GOOSE_PROVIDER__HOST: {}", provider_host);
    } else {
        tracing::warn!("GOOSE_PROVIDER__HOST not set");
    }
    
    if let Ok(provider_model) = std::env::var("GOOSE_PROVIDER__MODEL") {
        tracing::info!("GOOSE_PROVIDER__MODEL: {}", provider_model);
    } else {
        tracing::warn!("GOOSE_PROVIDER__MODEL not set");
    }

    let new_agent = Agent::new();
    tracing::info!("Agent created");

    // Check if the agent has a provider configured immediately after creation
    match new_agent.provider().await {
        Ok(_) => tracing::info!("Agent has provider configured after creation"),
        Err(e) => tracing::warn!("Agent does not have provider configured after creation: {:?}", e),
    }

    // Create app state with agent
    let state = state::AppState::new(Arc::new(new_agent), secret_key.clone()).await;
    tracing::info!("App state created with agent");

    // Create router with CORS support
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = crate::routes::configure(state).layer(cors);

    // Run server
    let listener = tokio::net::TcpListener::bind(settings.socket_addr()).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
