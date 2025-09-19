
use crate::configuration;
use crate::state;
use anyhow::Result;
use axum::middleware;
use etcetera::{choose_app_strategy, AppStrategy};
use goose::config::APP_STRATEGY;
use goose::scheduler_factory::SchedulerFactory;
use goose_server::auth::check_token;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use goose::providers::pricing::initialize_pricing_cache;

pub async fn run() -> Result<()> {
    // Initialize logging and telemetry
    crate::logging::setup_logging(Some("goosed"))?;

    let settings = configuration::Settings::new()?;

    // Initialize pricing cache on startup
    tracing::info!("Initializing pricing cache...");
    if let Err(e) = initialize_pricing_cache().await {
        tracing::warn!(
            "Failed to initialize pricing cache: {}. Pricing data may not be available.",
            e
        );
    }

    let secret_key =
        std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());

    let app_state = state::AppState::new();

    let schedule_file_path = choose_app_strategy(APP_STRATEGY.clone())?
        .data_dir()
        .join("schedules.json");

    let scheduler_instance = SchedulerFactory::create(schedule_file_path).await?;
    app_state.set_scheduler(scheduler_instance.clone()).await;

    // Configure default provider on the agent manager
    app_state
        .agent_manager()
        .configure_default_provider()
        .await?;
    app_state
        .agent_manager()
        .configure_default_provider()
        .await?;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = crate::routes::configure(app_state)
        .layer(middleware::from_fn_with_state(
            secret_key.clone(),
            check_token,
        ))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(settings.socket_addr()).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
