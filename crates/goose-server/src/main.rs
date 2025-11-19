use clap::{Parser, Subcommand};
use goose_server::{commands, logging};

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
            logging::setup_logging(Some(&format!("mcp-{name}")))?;
            goose_mcp::mcp_server_runner::run_mcp_server(name).await?;
        }
    }

    Ok(())
}
