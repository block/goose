use anyhow::Result;
use goose::config::Config;
use goose_mcp::{
    ComputerControllerRouter, DeveloperConfig, DeveloperRouter, GoogleDriveRouter, JetBrainsRouter,
    MemoryRouter, TutorialRouter,
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
            // Try to get developer config from config.yaml
            let dev_config = Config::global()
                .get_param::<DeveloperConfig>("developer")
                .unwrap_or_default();
            Some(Box::new(RouterService(DeveloperRouter::new(dev_config))))
        }
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
