use crate::configuration;
use crate::state;
use anyhow::Result;
use axum::middleware;
use axum_server::Handle;
use goose_server::auth::check_token;
use goose_server::tls::self_signed_config;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

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

    let secret_key =
        std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());

    let app_state = state::AppState::new().await?;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = crate::routes::configure(app_state.clone(), secret_key.clone())
        .layer(middleware::from_fn_with_state(
            secret_key.clone(),
            check_token,
        ))
        .layer(cors);

    let addr = settings.socket_addr();
    let tls_config = self_signed_config().await?;

    let handle = Handle::new();
    let shutdown_handle = handle.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        shutdown_handle.graceful_shutdown(None);
    });

    info!("listening on https://{}", addr);

    let tunnel_manager = app_state.tunnel_manager.clone();
    tokio::spawn(async move {
        tunnel_manager.check_auto_start().await;
    });

    axum_server::bind_rustls(addr, tls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await?;

    if goose::otel::otlp::is_otlp_initialized() {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        goose::otel::otlp::shutdown_otlp();
    }

    info!("server shutdown complete");
    Ok(())
}
