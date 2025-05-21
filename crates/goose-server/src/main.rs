// Items from lib.rs (goose_server crate) will be used via `goose_server::...`

mod commands;

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

    // The commands::agent::run() and commands::mcp::run() will need to
    // use goose_server::configuration, goose_server::logging, goose_server::state,
    // goose_server::routes, etc.
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
