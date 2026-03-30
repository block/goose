use anyhow::Result;
use goose_cli::cli::cli;

#[tokio::main]
async fn main() -> Result<()> {
    // The server subcommand sets up its own logging, so skip CLI logging for it
    let is_server = std::env::args().nth(1).is_some_and(|arg| arg == "server");
    if !is_server {
        if let Err(e) = goose_cli::logging::setup_logging(None) {
            eprintln!("Warning: Failed to initialize logging: {}", e);
        }
    }

    let result = cli().await;

    #[cfg(feature = "otel")]
    if goose::otel::otlp::is_otlp_initialized() {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        goose::otel::otlp::shutdown_otlp();
    }

    result
}
