use anyhow::Result;
use goose_mcp::{
    Developer2Router, DeveloperRouter, JetBrainsRouter, MemoryRouter, NonDeveloperRouter,
};
use mcp_server::router::RouterService;
use mcp_server::{BoundedService, ByteTransport, Server};
use tokio::io::{stdin, stdout};

pub async fn run(name: &str) -> Result<()> {
    // Initialize logging
    crate::logging::setup_logging(Some(&format!("mcp-{name}")))?;

    tracing::info!("Starting MCP server");
    let router: Option<Box<dyn BoundedService>> = match name {
        "developer" => Some(Box::new(RouterService(DeveloperRouter::new()))),
        "developer2" => Some(Box::new(RouterService(Developer2Router::new()))),
        "nondeveloper" => Some(Box::new(RouterService(NonDeveloperRouter::new()))),
        "jetbrains" => Some(Box::new(RouterService(JetBrainsRouter::new()))),
        "memory" => Some(Box::new(RouterService(MemoryRouter::new()))),
        _ => None,
    };

    // Create and run the server
    let server = Server::new(router.unwrap_or_else(|| panic!("Unknown server requested {}", name)));
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server initialized and ready to handle requests");
    Ok(server.run(transport).await?)
}
