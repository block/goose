mod app;
mod client;
mod commands;
mod config;
mod event;
mod tui;
mod ui;

use anyhow::Result;
use app::App;
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use uuid::Uuid;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the TUI (default)
    Tui,
    /// Run an MCP server (internal use)
    Mcp {
        /// Name of the MCP server type
        name: String,
    },
}

fn setup_tui_logging() -> Result<WorkerGuard> {
    let home_dir = directories::UserDirs::new()
        .ok_or_else(|| anyhow::anyhow!("Could not find user home directory"))?;
    let log_dir = home_dir.home_dir().join(".goose").join("logs");
    fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "tui.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,goose_tui=debug"));

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(env_filter)
        .with_ansi(false) // TUI doesn't want ANSI codes in log file usually
        .init();

    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Tui) {
        Commands::Tui => {
            // 1. Setup Logging (Critical: must not print to stdout)
            let _guard = setup_tui_logging()?;
            info!("Starting Goose TUI...");

            // 2. Configure Server Environment
            // Generate a secure random token for this session
            let secret_key = Uuid::new_v4().to_string();
            env::set_var("GOOSE_SERVER__SECRET_KEY", &secret_key);
            env::set_var("GOOSE_SERVER__PORT", "0"); // Ephemeral port

            info!("Initializing embedded server...");

            // 3. Build Embedded Server
            // We use the library function we exposed in goose-server
            let (server_app, listener) = goose_server::commands::agent::build_app().await?;

            let port = listener.local_addr()?.port();
            info!("Embedded server bound to port: {}", port);

            // 4. Spawn Server in Background
            tokio::spawn(async move {
                if let Err(e) = axum::serve(listener, server_app).await {
                    tracing::error!("Server error: {}", e);
                }
            });

            // 5. Start TUI
            // Pass the port so the App knows where to connect
            let mut app = App::new(port, secret_key).await?;
            app.run().await?;
        }
        Commands::Mcp { name } => {
            goose_server::logging::setup_logging(Some(&format!("mcp-{name}")))?;
            goose_mcp::mcp_server_runner::run_mcp_server(&name).await?;
        }
    }

    Ok(())
}
