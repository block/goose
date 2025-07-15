use anyhow::Result;
use goose_cli::cli::cli;

#[tokio::main]
async fn main() -> Result<()> {
    let telemetry_init_result = goose::telemetry::init_global_telemetry().await;

    if let Err(e) = &telemetry_init_result {
        eprintln!("⚠️ Telemetry initialization failed: {}", e);
        eprintln!("   This may be due to configuration issues or connectivity problems.");
        eprintln!("   The application will continue without telemetry.");
    }

    let result = cli().await;

    if let Err(e) = goose::telemetry::shutdown_global_telemetry().await {
        eprintln!("⚠️ Failed to shutdown telemetry: {}", e);
    }

    result
}
