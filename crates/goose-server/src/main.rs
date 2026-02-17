mod agent_slot_registry;
mod auth;
mod commands;
mod configuration;
mod error;
mod logging;
mod openapi;
mod routes;
mod state;
mod tunnel;

use clap::{Parser, Subcommand};
use goose::config::paths::Paths;
use goose_mcp::{
    mcp_server_runner::{serve, McpCommand},
    AutoVisualiserRouter, ComputerControllerServer, DeveloperServer, MemoryServer, TutorialServer,
};

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
        #[arg(value_parser = clap::value_parser!(McpCommand))]
        server: McpCommand,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Agent => {
            commands::agent::run().await?;
        }
        Commands::Mcp { server } => {
            logging::setup_logging(Some(&format!("mcp-{}", server.name())))?;
            match server {
                McpCommand::AutoVisualiser => serve(AutoVisualiserRouter::new()).await?,
                McpCommand::ComputerController => serve(ComputerControllerServer::new()).await?,
                McpCommand::Memory => serve(MemoryServer::new()).await?,
                McpCommand::Tutorial => serve(TutorialServer::new()).await?,
                McpCommand::Developer => {
                    let bash_env = Paths::config_dir().join(".bash_env");
                    serve(
                        DeveloperServer::new()
                            .extend_path_with_shell(true)
                            .bash_env_file(Some(bash_env)),
                    )
                    .await?
                }
            }
        }
    }

    Ok(())
}
