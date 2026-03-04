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
use goose_mcp::{
    mcp_server_runner::{serve, McpCommand},
    AutoVisualiserRouter, ComputerControllerServer, MemoryServer, TutorialServer,
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
    let _sentry_guard = if goose::posthog::is_telemetry_enabled() {
        Some(sentry::init((
            "https://4ded405f3749b4952425eb404e212119@o160250.ingest.us.sentry.io/4510975954124800",
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 1.0,
                environment: Some(
                    std::env::var("GOOSE_ENVIRONMENT")
                        .unwrap_or_else(|_| "development".into())
                        .into(),
                ),
                ..Default::default()
            },
        )))
    } else {
        None
    };

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
            }
        }
        Commands::ValidateExtensions { path } => {
            match validate_extensions::validate_bundled_extensions(&path) {
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
