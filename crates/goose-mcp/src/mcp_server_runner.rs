use crate::{
    AutoVisualiserRouter, ComputerControllerServer, DeveloperServer, MemoryServer, TutorialServer,
};
use anyhow::{anyhow, Result};
use rmcp::{transport::stdio, ServiceExt};

/// Run an MCP server by name
///
/// This function handles the common logic for starting MCP servers.
/// The caller is responsible for setting up logging before calling this function.
pub async fn run_mcp_server(name: &str) -> Result<()> {
    if name == "googledrive" || name == "google_drive" {
        return Err(anyhow!(
            "the built-in Google Drive extension has been removed"
        ));
    }

    tracing::info!("Starting MCP server");

    match name {
        "autovisualiser" => serve_and_wait(AutoVisualiserRouter::new()).await,
        "computercontroller" => serve_and_wait(ComputerControllerServer::new()).await,
        "developer" => serve_and_wait(DeveloperServer::new()).await,
        "memory" => serve_and_wait(MemoryServer::new()).await,
        "tutorial" => serve_and_wait(TutorialServer::new()).await,
        _ => {
            tracing::warn!("Unknown MCP server name: {}", name);
            Err(anyhow!("Unknown MCP server name: {}", name))
        }
    }
}

/// Helper function to run any MCP server with common error handling
async fn serve_and_wait<S>(server: S) -> Result<()>
where
    S: rmcp::ServerHandler,
{
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;

    Ok(())
}
