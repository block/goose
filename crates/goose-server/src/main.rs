use std::path::PathBuf;

use clap::{Parser, Subcommand};
use goose::agents::validate_extensions;
use goose::config::paths::Paths;
use goose_mcp::mcp_server_runner::{serve, McpCommand};
use goose_mcp::{
    AutoVisualiserRouter, ComputerControllerServer, DeveloperServer, MemoryServer, TutorialServer,
};
use goose_server::{commands, logging};
use std::str::FromStr;

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

    match &cli.command {
        Commands::Agent => {
            commands::agent::run().await?;
        }
        Commands::Mcp { name } => {
            logging::setup_logging(Some(&format!("mcp-{name}")))?;
            let server = McpCommand::from_str(name)
                .map_err(|e| anyhow::anyhow!("Invalid MCP server: {}", e))?;
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
            match validate_extensions::validate_bundled_extensions(path) {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
