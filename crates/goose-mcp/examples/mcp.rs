// An example script to run an MCP server using the rmcp SDK
use anyhow::Result;
use goose_mcp::MemoryServer;
use rmcp::{transport::stdio, ServiceExt};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Set up file appender for logging
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "goose-mcp-example.log");

    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(file_appender)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Starting MCP server using rmcp SDK");

    // Create an instance of our memory server
    let memory_server = MemoryServer::new();

    // Create the transport and run the server
    let (stdin, stdout) = stdio();
    memory_server.serve((stdin, stdout)).await?;

    Ok(())
}
