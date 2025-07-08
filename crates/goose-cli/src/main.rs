use anyhow::Result;
use goose_cli::cli::cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Check if this is an MCP process - if so, don't initialize telemetry
    let args: Vec<String> = std::env::args().collect();
    let is_mcp_process = args.len() >= 2 && args[1] == "mcp";

    if !is_mcp_process {
        if let Err(e) = goose::telemetry::init_global_telemetry().await {
            // Don't fail the application if telemetry fails to initialize
            tracing::warn!("Failed to initialize telemetry: {}", e);
        }
    }

    let result = cli().await;

    if !is_mcp_process {
        if let Err(e) = goose::telemetry::shutdown_global_telemetry().await {
            tracing::warn!("Failed to shutdown telemetry: {}", e);
        }
    }

    result
}
