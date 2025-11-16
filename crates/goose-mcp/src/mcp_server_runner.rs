use crate::{
    AutoVisualiserRouter, ComputerControllerServer, DeveloperServer, MemoryServer, TutorialServer,
};
use anyhow::{anyhow, Result};
use rmcp::{transport::stdio, ServiceExt};
use std::{env, os::unix::process::CommandExt, process::Command};

const DEV_TOOL_NO_FORK_ENV: &str = "GOOSE_DEVELOPER_MCP_NO_SHELL";

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
        "developer" => {
            if cfg!(unix)
                && env::var(DEV_TOOL_NO_FORK_ENV)
                    .unwrap_or_default()
                    .is_empty()
            {
                let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
                let command: String = env::args().collect::<Vec<_>>().join(" ");
                let err = Command::new(&shell)
                    .env(DEV_TOOL_NO_FORK_ENV, "1")
                    .arg("-l")
                    .arg("-c")
                    .arg(command)
                    .exec();
                eprintln!("exec failed: {}", err);
                std::process::exit(1);
            } else {
                serve_and_wait(DeveloperServer::new()).await
            }
        }
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
