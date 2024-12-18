use anyhow::Result;
use mcp_core::handler::ToolError;
use mcp_server::{Router, Server, ByteTransport};
use tokio::io::{stdin, stdout};
use tokio::time::Duration;
use tower::timeout::Timeout;
use tracing_subscriber::{self, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use mcp_macros::tool;

#[tool(
    name = "calculator",
    description = "Perform basic arithmetic operations",
    params(
        x = "First number in the calculation",
        y = "Second number in the calculation",
        operation = "The operation to perform (add, subtract, multiply, divide)"
    )
)]
async fn calculator(x: String, y: String, operation: String) -> mcp_core::handler::Result<i32> {
    let x: i32 = x.parse().unwrap();
    let y: i32 = y.parse().unwrap();
    match operation.as_str() {
        "add" => Ok(x + y),
        "subtract" => Ok(x - y),
        "multiply" => Ok(x * y),
        "divide" => {
            if y == 0 {
                Err(ToolError::ExecutionError("Division by zero".into()))
            } else {
                Ok(x / y)
            }
        }
        _ => Err(ToolError::InvalidParameters(format!(
            "Unknown operation: {}",
            operation
        ))),
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    // Set up file appender
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        "logs",
        "mcp-server.log",
    );

    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .with_writer(file_appender)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Starting MCP server");

    // Create a router with the calculator tool
    let router = Router::builder()
        .with_tool(Calculator::default())
        .build();

    // Add a 30 second timeout to all requests
    let router = Timeout::new(router, Duration::from_secs(30));
    let server = Server::new(router);
    
    let transport = ByteTransport::new(stdin(), stdout());
    Ok(server.run(transport).await?)
}
