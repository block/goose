use crate::configuration;
use crate::state;
use anyhow::Result;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

pub async fn run() -> Result<()> {
    // Initialize logging
    crate::logging::setup_logging(Some("goosed"))?;

    // Load configuration
    let settings = configuration::Settings::new()?;

    // load secret key from GOOSE_SERVER__SECRET_KEY environment variable
    let secret_key =
        std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());

    // Create app state - agent will start as None
    let state = state::AppState::new(secret_key.clone()).await?;

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
