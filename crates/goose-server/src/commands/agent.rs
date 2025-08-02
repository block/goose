use std::sync::Arc;

use crate::configuration;
use crate::state;
use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy};
use goose::agents::Agent;
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

    let new_agent = Agent::new();
    let agent_ref = Arc::new(new_agent);

    let app_state = state::AppState::new(agent_ref.clone(), secret_key.clone()).await;

    let schedule_file_path = choose_app_strategy(APP_STRATEGY.clone())?
        .data_dir()
        .join("schedules.json");

    let scheduler_instance = SchedulerFactory::create(schedule_file_path).await?;
    app_state.set_scheduler(scheduler_instance.clone()).await;

    // NEW: Provide scheduler access to the agent
    agent_ref.set_scheduler(scheduler_instance).await;

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
