use anyhow::{anyhow, Result};
use goose_mcp::{
    AutoVisualiserRouter, ComputerControllerRouter, DeveloperServer, MemoryRouter, TutorialRouter,
};
use mcp_server::router::RouterService;
use mcp_server::{BoundedService, ByteTransport, Server};
use rmcp::{transport::stdio, ServiceExt};
use tokio::io::{stdin, stdout};

pub async fn run(name: &str) -> Result<()> {
    crate::logging::setup_logging(Some(&format!("mcp-{name}")))?;

    if name == "googledrive" || name == "google_drive" {
        return Err(anyhow!(
            "the built-in Google Drive extension has been removed"
        ));
    }

    tracing::info!("Starting MCP server");

    if name == "developer" {
        let service = DeveloperServer::new()
            .serve(stdio())
            .await
            .inspect_err(|e| {
                tracing::error!("serving error: {:?}", e);
            })?;

        service.waiting().await?;
        return Ok(());
    }

    if name == "autovisualiser" {
        let service = AutoVisualiserRouter::new()
            .serve(stdio())
            .await
            .inspect_err(|e| {
                tracing::error!("serving error: {:?}", e);
            })?;

        service.waiting().await?;
        return Ok(());
    }

    let router: Option<Box<dyn BoundedService>> = match name {
        "computercontroller" => Some(Box::new(RouterService(ComputerControllerRouter::new()))),
        "memory" => Some(Box::new(RouterService(MemoryRouter::new()))),
        "tutorial" => Some(Box::new(RouterService(TutorialRouter::new()))),
        _ => None,
    };

    let server = Server::new(router.unwrap_or_else(|| panic!("Unknown server requested {}", name)));
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server initialized and ready to handle requests");
    Ok(server.run(transport).await?)
}
