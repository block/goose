mod commands;
mod configuration;
mod error;
mod logging;
mod openapi;
mod routes;
mod state;
mod tunnel;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use goose::agents::validate_extensions;
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
    /// Validate a bundled-extensions JSON file
    #[command(name = "validate-extensions")]
    ValidateExtensions {
        /// Path to the bundled-extensions JSON file
        path: PathBuf,
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
        Commands::ValidateExtensions { path } => {
            let result = validate_extensions::validate_bundled_extensions(&path)?;
            if result.errors.is_empty() {
                println!("✓ All {} extensions validated successfully.", result.total);
            } else {
                eprintln!(
                    "✗ Found {} error(s) in {} extension(s):\n",
                    result.errors.len(),
                    result.errors.len()
                );
                for (i, error) in result.errors.iter().enumerate() {
                    eprintln!("  [{}] {}", i, error);
                }
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
