use anyhow::Result;
use goose_mcp::DeveloperRouter;
use mcp_server::router::RouterService;
use mcp_server::{ByteTransport, Server};
use tokio::io::{stdin, stdout};

pub async fn run_server(name: &str) -> Result<()> {
    tracing::info!("Starting MCP server");

    let router = match name {
        "developer" => Some(RouterService(DeveloperRouter::new())),
        _ => None,
    };

    // Create and run the server
    let server = Server::new(router.unwrap_or_else(|| panic!("Unknown server requested {}", name)));
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server initialized and ready to handle requests");
    Ok(server.run(transport).await?)
}
