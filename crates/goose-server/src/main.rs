use etcetera::AppStrategyArgs;
use once_cell::sync::Lazy;

pub static APP_STRATEGY: Lazy<AppStrategyArgs> = Lazy::new(|| AppStrategyArgs {
    top_level_domain: "Block".to_string(),
    author: "Block".to_string(),
    app_name: "goose".to_string(),
});

mod commands;
mod configuration;
mod error;
mod logging;
mod openapi;
mod routes;
mod state;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the agent server
    Agent,
    /// Run the MCP server
    Mcp {
        /// Name of the MCP server type
        name: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Agent => {
            commands::agent::run().await?;
        }
        Commands::Mcp { name } => {
            commands::mcp::run(name).await?;
        }
    }

    Ok(())
}
