use anyhow::Result;
use goose_cli::cli::cli;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = goose_cli::logging::setup_logging(None, None) {
        eprintln!("Warning: Failed to initialize telemetry: {}", e);
    }

    let result = cli().await;

    // Flush telemetry before exiting
    // Give metrics time to be exported
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Then shutdown the providers
    goose::tracing::shutdown_otlp();

    result
}
