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
    /// 
    /// The server can be configured to bind to a specific host/interface and port in three ways:
    /// 1. Command-line arguments: --host and --port (if provided)
    /// 2. Environment variables: GOOSE__HOST and GOOSE__PORT (if CLI args not provided)
    /// 3. Default values: 127.0.0.1:3000 (if neither CLI args nor env vars are provided)
    Agent {
        /// Host or IP address to bind the server to
        #[arg(long)]
        host: Option<String>,
        
        /// Port the server should listen on
        #[arg(long)]
        port: Option<u16>,
    },
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
        Commands::Agent { host, port } => {
            commands::agent::run(host.as_deref(), *port).await?;
        }
        Commands::Mcp { name } => {
            commands::mcp::run(name).await?;
        }
    }

    Ok(())
}
