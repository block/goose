use anyhow::Result;
use goose_cli::cli::cli;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = goose_cli::logging::setup_logging(None, None) {
        eprintln!("Warning: Failed to initialize telemetry: {}", e);
    }

    let result = cli().await;

    // Only wait for telemetry flush if OTLP is configured
    if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
        // Use a shorter, dynamic wait with max timeout
        let max_wait = tokio::time::Duration::from_millis(500);
        let start = tokio::time::Instant::now();

        // Give telemetry a chance to flush, but don't wait too long
        while start.elapsed() < max_wait {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // In future, we could check if there are pending spans/metrics here
            // For now, we just do a quick wait to allow batch exports to complete
            if start.elapsed() >= tokio::time::Duration::from_millis(200) {
                break; // Most exports should complete within 200ms
            }
        }

        // Then shutdown the providers
        goose::tracing::shutdown_otlp();
    }

    result
}
