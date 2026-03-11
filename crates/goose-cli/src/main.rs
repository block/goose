use anyhow::Result;
use goose_cli::cli::cli;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = goose_cli::logging::setup_logging(None) {
        eprintln!("Warning: Failed to initialize logging: {}", e);
    }

    // Install a non-terminating SIGINT handler so that interactive prompts
    // (e.g. cliclack) can clean up the terminal cursor when Ctrl+C is pressed.
    // Without this, the `console` crate raises SIGINT on Ctrl+C and the default
    // handler kills the process before cursor restoration can run.
    // The loop ensures there is always an active listener so that SIGINT is never
    // silently discarded after the first Ctrl+C.
    tokio::spawn(async {
        loop {
            tokio::signal::ctrl_c().await.ok();
        }
    });

    let result = cli().await;

    if goose::otel::otlp::is_otlp_initialized() {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        goose::otel::otlp::shutdown_otlp();
    }

    result
}
