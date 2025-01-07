mod configuration;
mod error;
mod routes;
mod state;

use tower_http::cors::{Any, CorsLayer};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let settings = configuration::Settings::new()?;

    // load secret key from GOOSE_SERVER__SECRET_KEY environment variable
    let secret_key =
        std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());

    // Create app state
    let state = state::AppState::new(settings.provider.into_config(), secret_key.clone())?;

    // Create router with CORS support
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = routes::configure(state).layer(cors);

    // Run server
    let listener = tokio::net::TcpListener::bind(settings.server.socket_addr()).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
