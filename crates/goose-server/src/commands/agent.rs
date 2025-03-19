use crate::configuration;
use crate::state;
use anyhow::Result;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

pub async fn run(host: Option<&str>, port: Option<u16>) -> Result<()> {
    // Initialize logging
    crate::logging::setup_logging(Some("goosed"))?;

    // Load configuration
    let mut settings = configuration::Settings::new()?;
    
    // Override settings with CLI arguments if provided
    if let Some(host_value) = host {
        settings.host = host_value.to_string();
    }
    
    if let Some(port_value) = port {
        settings.port = port_value;
    }

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
