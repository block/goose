use anyhow::Result;
use clap::Parser;
mod computercontroller;
mod developer;
mod google_drive;
mod jetbrains;
mod memory;
mod tutorial;
use etcetera::AppStrategyArgs;
use mcp_server::router::RouterService;
use mcp_server::{BoundedService, ByteTransport, Server};
use once_cell::sync::Lazy;
use tokio::io::{stdin, stdout};

use computercontroller::ComputerControllerRouter;
use developer::DeveloperRouter;
use google_drive::GoogleDriveRouter;
use jetbrains::JetBrainsRouter;
use memory::MemoryRouter;
use tutorial::TutorialRouter;

pub async fn run(name: &str) -> Result<()> {
    tracing::info!("Starting MCP server");
    let router: Option<Box<dyn BoundedService>> = match name {
        "developer" => Some(Box::new(RouterService(DeveloperRouter::new()))),
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

pub static APP_STRATEGY: Lazy<AppStrategyArgs> = Lazy::new(|| AppStrategyArgs {
    top_level_domain: "Block".to_string(),
    author: "Block".to_string(),
    app_name: "goose".to_string(),
});

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Name of the MCP server type
    name: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    run(&cli.name).await?;

    Ok(())
}
// cargo build -p goose-mcp --release
