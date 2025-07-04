use anyhow::Result;
use goose_cli::cli::cli;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = goose::telemetry::init_global_telemetry().await {
        // Don't fail the application if telemetry fails to initialize
        tracing::warn!("Failed to initialize telemetry: {}", e);
    }

    let result = cli().await;

    if let Err(e) = goose::telemetry::shutdown_global_telemetry().await {
        tracing::warn!("Failed to shutdown telemetry: {}", e);
    }

    result
}
