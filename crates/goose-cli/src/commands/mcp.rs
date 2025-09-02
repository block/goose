use anyhow::Result;
use goose_mcp::{
    AutoVisualiserServer, ComputerControllerServer, DeveloperServer, MemoryServer, TutorialServer,
};
use rmcp::{transport::stdio, ServiceExt};

pub async fn run_server(name: &str) -> Result<()> {
    crate::logging::setup_logging(Some(&format!("mcp-{name}")), None)?;

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
    if name == "tutorial" {
        let service = TutorialServer::new()
            .serve(stdio())
            .await
            .inspect_err(|e| {
                tracing::error!("serving error: {:?}", e);
            })?;

        service.waiting().await?;
        return Ok(());
    }
    if name == "memory" {
        let service = MemoryServer::new().serve(stdio()).await.inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?;

        service.waiting().await?;
        return Ok(());
    }
    if name == "computercontroller" {
        let service = ComputerControllerServer::new()
            .serve(stdio())
            .await
            .inspect_err(|e| {
                tracing::error!("serving error: {:?}", e);
            })?;

        service.waiting().await?;
        return Ok(());
    }
    if name == "autovisualiser" {
        let service = AutoVisualiserServer::new()
            .serve(stdio())
            .await
            .inspect_err(|e| {
                tracing::error!("serving error: {:?}", e);
            })?;

        service.waiting().await?;
        return Ok(());
    }

    panic!("Unknown server requested {}", name);
}
