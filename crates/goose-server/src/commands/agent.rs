use std::sync::Arc;

use anyhow::Result;
use goose::agents::Agent;
use goose_server::configuration;
use goose_server::{logging, routes, scheduler, state};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

pub async fn run() -> Result<()> {
    // Initialize logging
    logging::setup_logging(Some("goosed"))?;

    // Load configuration
    let settings = configuration::Settings::new()?;

    // load secret key from GOOSE_SERVER__SECRET_KEY environment variable
    let secret_key =
        std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());

    let new_agent = Agent::new();

    // Create app state with agent
    let state = state::AppState::new(Arc::new(new_agent), secret_key.clone()).await;

    // Start scheduler and attach to state
    let scheduler = scheduler::Scheduler::new(state.clone()).await?;
    state.set_scheduler(scheduler).await;

    // Create router with CORS support
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = routes::configure(state).layer(cors);

    // Run server
    let listener = tokio::net::TcpListener::bind(settings.socket_addr()).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
