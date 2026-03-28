mod agent;
mod app;
mod colors;
mod components;
mod markdown;
mod types;

use clap::Parser;
use iocraft::prelude::*;

use app::App;

#[derive(Parser, Debug)]
#[command(name = "goose-tui", about = "Goose terminal UI")]
pub struct Args {
    /// Send a single prompt and exit (non-interactive)
    #[arg(short, long)]
    pub text: Option<String>,

    /// Resume an existing session by ID
    #[arg(short, long)]
    pub session: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Log to a file so tracing output doesn't corrupt the TUI.
    let log_file = std::env::temp_dir().join("goose-tui.log");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(file)
        .init();

    let args = Args::parse();

    element!(App(
        initial_prompt: args.text,
        session_id: args.session,
    ))
    .fullscreen()
    .await?;

    Ok(())
}
