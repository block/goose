use std::sync::Arc;

use crate::configuration;
use crate::state;
use anyhow::Result;
use axum::middleware;
use etcetera::{choose_app_strategy, AppStrategy};
use goose::agents::Agent;
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

    let new_agent = Agent::new();

    // Only initialize provider and extensions when running in standalone goosed mode
    // This prevents breaking the Electron app which manages its own provider setup
    if std::env::var("GOOSE_STANDALONE_MODE").unwrap_or_else(|_| "false".to_string()) == "true" {
        tracing::info!("Running in standalone mode - initializing provider and extensions");

        // Initialize provider like the CLI does
        let config = goose::config::Config::global();

        let provider_name: String = config
            .get_param("GOOSE_PROVIDER")
            .expect("No provider configured. Run 'goose configure' first");

        let model_name: String = config
            .get_param("GOOSE_MODEL")
            .expect("No model configured. Run 'goose configure' first");

        let model_config = goose::model::ModelConfig::new(&model_name)
            .expect("Failed to create model configuration");

        let provider = goose::providers::create(&provider_name, model_config)
            .expect("Failed to create provider");

        new_agent
            .update_provider(provider)
            .await
            .expect("Failed to update agent provider");
    }

    let agent_ref = Arc::new(new_agent);

    let app_state = state::AppState::new(agent_ref.clone());

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
