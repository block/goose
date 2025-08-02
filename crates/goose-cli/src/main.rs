use anyhow::Result;
use goose_cli::cli::cli;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = goose_cli::logging::setup_logging(None, None) {
        eprintln!("Warning: Failed to initialize telemetry: {}", e);
    }

    cli().await
}
