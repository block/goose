use crate::configuration;
use crate::state;
use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};
use goose::config::APP_STRATEGY;
use goose::scheduler_factory::SchedulerFactory;
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

    let app_state = state::AppState::new(secret_key.clone()).await;

    let schedule_file_path = choose_app_strategy(APP_STRATEGY.clone())?
        .data_dir()
        .join("schedules.json");

    let scheduler_instance = SchedulerFactory::create(schedule_file_path).await?;
    app_state.set_scheduler(scheduler_instance.clone()).await;

    // TODO: Once we have per-session agents, each agent will need scheduler access
    // For now, we'll handle this when agents are created in AgentManager

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = crate::routes::configure(app_state).layer(cors);

    let listener = tokio::net::TcpListener::bind(settings.socket_addr()).await?;
    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
