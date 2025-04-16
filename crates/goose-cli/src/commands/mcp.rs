use anyhow::Result;
use goose_mcp::{
    ComputerControllerRouter, DeveloperRouter, GoogleDriveRouter, JetBrainsRouter, MemoryRouter,
    TutorialRouter,
};
use mcp_server::router::RouterService;
use mcp_server::{BoundedService, ByteTransport, Server};
use tokio::io::{stdin, stdout};

pub async fn run_server(name: &str) -> Result<()> {
    // Initialize logging
    crate::logging::setup_logging(Some(&format!("mcp-{name}")), None)?;

    tracing::info!("Starting MCP server");

    let router: Option<Box<dyn BoundedService>> = match name {
        "developer" => {
            let config = goose::config::Config::global();
            let disable_local_hints = config.get_param("goose_disable_local_hints").unwrap_or(false);
            Some(Box::new(RouterService(DeveloperRouter::with_config(
                goose_mcp::DeveloperConfig {
                    disable_local_hints,
                },
            ))))
        },
        "computercontroller" => Some(Box::new(RouterService(ComputerControllerRouter::new()))),
        "jetbrains" => Some(Box::new(RouterService(JetBrainsRouter::new()))),
        "google_drive" | "googledrive" => {
            let router = GoogleDriveRouter::new().await;
            Some(Box::new(RouterService(router)))
        }
        "memory" => Some(Box::new(RouterService(MemoryRouter::new()))),
        "tutorial" => Some(Box::new(RouterService(TutorialRouter::new()))),
        _ => None,
    };

    // Create and run the server
    let server = Server::new(router.unwrap_or_else(|| panic!("Unknown server requested {}", name)));
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server initialized and ready to handle requests");
    Ok(server.run(transport).await?)
}
