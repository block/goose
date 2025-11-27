use std::str::FromStr;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};

#[derive(Clone, Debug)]
pub enum McpCommand {
    AutoVisualiser,
    ComputerController,
    Developer,
    Memory,
    Tutorial,
}

impl FromStr for McpCommand {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(' ', "").as_str() {
            "autovisualiser" => Ok(McpCommand::AutoVisualiser),
            "computercontroller" => Ok(McpCommand::ComputerController),
            "developer" => Ok(McpCommand::Developer),
            "memory" => Ok(McpCommand::Memory),
            "tutorial" => Ok(McpCommand::Tutorial),
            _ => Err(format!("Invalid command: {}", s)),
        }
    }
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
