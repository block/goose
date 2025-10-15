use anyhow::Result;
use goose_cli::cli::cli;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = goose_cli::logging::setup_logging(None, None) {
        eprintln!("Warning: Failed to initialize telemetry: {}", e);
    }

    let result = cli().await;

    // Shutdown OTLP providers if they were initialized
    if goose::tracing::is_otlp_initialized() {
        // Give batch exporters a moment to flush pending data
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Shutdown providers (calls shutdown on meter provider and tracer provider)
        goose::tracing::shutdown_otlp();
    }

    result
}
