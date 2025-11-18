use anyhow::Result;
use clap::ValueEnum;
use rmcp::{transport::stdio, ServiceExt};

#[derive(Clone, ValueEnum)]
pub enum McpCommand {
    AutoVisualiser,
    ComputerController,
    Developer,
    Memory,
    Tutorial,
}

impl McpCommand {
    pub fn name(&self) -> &str {
        match self {
            McpCommand::AutoVisualiser => "autovisualiser",
            McpCommand::ComputerController => "computercontroller",
            McpCommand::Developer => "developer",
            McpCommand::Memory => "memory",
            McpCommand::Tutorial => "tutorial",
        }
    }
}

pub async fn serve<S>(server: S) -> Result<()>
where
    S: rmcp::ServerHandler,
{
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;

    Ok(())
}
