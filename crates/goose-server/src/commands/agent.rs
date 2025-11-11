use crate::configuration;
use crate::state;
use anyhow::Result;
use axum::middleware;
use goose_server::auth::check_token;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use goose::providers::pricing::initialize_pricing_cache;

// Graceful shutdown signal
#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

    tokio::select! {
        _ = sigint.recv() => {},
        _ = sigterm.recv() => {},
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

pub async fn run() -> Result<()> {
    crate::logging::setup_logging(Some("goosed"))?;

    let settings = configuration::Settings::new()?;

    if let Err(e) = initialize_pricing_cache().await {
        tracing::warn!(
            "Failed to initialize pricing cache: {}. Pricing data may not be available.",
            e
        );
    }

    let secret_key =
        std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());

    let app_state = state::AppState::new().await?;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = crate::routes::configure(app_state.clone())
        .layer(middleware::from_fn_with_state(
            secret_key.clone(),
            check_token,
        ))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(settings.socket_addr()).await?;
    let local_addr = listener.local_addr()?;
    info!("listening on {}", local_addr);

    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        // a very short pause for goosed to start, I found without this, reconnections may struggle
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        app_state_clone.auto_start_tunnel().await;
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    info!("server shutdown complete");
    Ok(())
}
