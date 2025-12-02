mod action_handler;
mod app;
mod components;
mod headless;
mod runner;
mod services;
mod state;
mod tui;
mod utils;

use anyhow::Result;
use app::App;
use clap::{Parser, Subcommand};
use goose_client::Client;
use runner::{run_event_loop, run_recipe_event_loop};
use services::config::TuiConfig;
use services::events::EventHandler;
use state::AppState;
use std::fs;
use std::io::{IsTerminal, Read};
use std::path::PathBuf;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use uuid::Uuid;

/// Configure a session with the global provider and extensions.
/// Returns (provider, model) for state initialization.
pub async fn configure_session_from_global(
    client: &Client,
    session_id: &str,
) -> (String, Option<String>) {
    let global_config = goose::config::Config::global();
    let provider = global_config
        .get_goose_provider()
        .unwrap_or_else(|_| "openai".to_string());
    let model = global_config.get_goose_model().ok();

    if let Err(e) = client
        .update_provider(session_id, provider.clone(), model.clone())
        .await
    {
        tracing::error!("Failed to update provider: {e}");
    }

    for ext in goose::config::get_enabled_extensions() {
        if let Err(e) = client.add_extension(session_id, ext.clone()).await {
            tracing::error!("Failed to add extension {}: {}", ext.name(), e);
        }
    }

    (provider, model)
}

#[derive(Parser)]
#[command(author, version, about = "A terminal user interface for Goose")]
struct Cli {
    #[arg(long, help = "Resume an existing session by ID")]
    session: Option<String>,

    #[arg(long, value_name = "FILE", help = "Run a recipe file")]
    recipe: Option<PathBuf>,

    #[arg(long, help = "Run in headless mode (plain text output, no TUI)")]
    headless: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(hide = true)]
    Mcp { name: String },
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
        .with_ansi(false)
        .init();

    Ok(guard)
}

/// Read stdin if it's piped (not a TTY), returns None otherwise.
fn read_stdin_if_piped() -> Option<String> {
    if std::io::stdin().is_terminal() {
        return None;
    }

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return None;
    }

    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Mcp { name }) = cli.command {
        return run_mcp_server(name);
    }

    // Read stdin before starting async runtime (must be done synchronously)
    let stdin_input = read_stdin_if_piped();

    let secret_key = Uuid::new_v4().to_string();
    std::env::set_var("GOOSE_SERVER__SECRET_KEY", &secret_key);
    std::env::set_var("GOOSE_PORT", "0");

    tokio::runtime::Runtime::new()?.block_on(run_tui(
        cli.session,
        cli.recipe,
        cli.headless,
        stdin_input,
        secret_key,
    ))
}

fn run_mcp_server(name: String) -> Result<()> {
    use goose_mcp::mcp_server_runner::{serve, McpCommand};
    use goose_mcp::{
        AutoVisualiserRouter, ComputerControllerServer, DeveloperServer, MemoryServer,
        TutorialServer,
    };
    use std::str::FromStr;

    tokio::runtime::Runtime::new()?.block_on(async {
        goose_server::logging::setup_logging(Some(&format!("mcp-{name}")))?;
        let server = McpCommand::from_str(&name)
            .map_err(|e| anyhow::anyhow!("Invalid MCP server: {}", e))?;
        match server {
            McpCommand::AutoVisualiser => serve(AutoVisualiserRouter::new()).await?,
            McpCommand::ComputerController => serve(ComputerControllerServer::new()).await?,
            McpCommand::Memory => serve(MemoryServer::new()).await?,
            McpCommand::Tutorial => serve(TutorialServer::new()).await?,
            McpCommand::Developer => serve(DeveloperServer::new()).await?,
        }
        Ok(())
    })
}

fn load_recipe(path: &PathBuf) -> Result<goose::recipe::Recipe> {
    let content = fs::read_to_string(path)?;
    let recipe_dir = path.parent().map(|p| p.to_string_lossy().to_string());
    goose::recipe::validate_recipe::validate_recipe_template_from_content(&content, recipe_dir)
}

async fn run_tui(
    session: Option<String>,
    recipe: Option<PathBuf>,
    headless: bool,
    stdin_input: Option<String>,
    secret_key: String,
) -> Result<()> {
    let _guard = setup_tui_logging()?;
    info!("Starting Goose TUI...");

    let (server_app, listener) = goose_server::commands::agent::build_app().await?;
    let port = listener.local_addr()?.port();
    info!("Embedded server bound to port: {}", port);

    let shutdown_token = CancellationToken::new();
    let server_shutdown = shutdown_token.clone();

    let server_handle = tokio::spawn(async move {
        let server = axum::serve(listener, server_app).with_graceful_shutdown(async move {
            server_shutdown.cancelled().await;
        });
        if let Err(e) = server.await {
            tracing::error!("Server error: {}", e);
        }
    });

    let client = Client::new(port, secret_key);
    let cwd = std::env::current_dir()?;

    // Priority: recipe > stdin > interactive
    let result = if let Some(recipe_path) = recipe {
        run_recipe_mode(client, cwd, recipe_path, headless).await
    } else if let Some(prompt) = stdin_input {
        run_stdin_mode(client, cwd, session, prompt).await
    } else {
        run_interactive_mode(client, cwd, session).await
    };

    shutdown_token.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;

    result
}

async fn run_recipe_mode(
    client: Client,
    cwd: std::path::PathBuf,
    recipe_path: PathBuf,
    headless: bool,
) -> Result<()> {
    let recipe = load_recipe(&recipe_path)?;
    let prompt = recipe
        .prompt
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Recipe has no prompt"))?;

    info!("Running recipe: {}", recipe.title);

    let initial_session = client
        .start_agent_with_recipe(cwd.to_string_lossy().to_string(), recipe)
        .await?;

    configure_session_from_global(&client, &initial_session.id).await;

    let use_headless = headless || !std::io::stdout().is_terminal();

    if use_headless {
        headless::run_headless(client, initial_session.id, prompt).await
    } else {
        run_recipe_tui_mode(client, initial_session, prompt).await
    }
}

async fn run_stdin_mode(
    client: Client,
    cwd: std::path::PathBuf,
    session: Option<String>,
    prompt: String,
) -> Result<()> {
    info!("Running with stdin input");

    let initial_session = if let Some(id) = session {
        info!("Resuming session: {}", id);
        client.resume_agent(&id).await?
    } else {
        client
            .start_agent(cwd.to_string_lossy().to_string())
            .await?
    };

    configure_session_from_global(&client, &initial_session.id).await;

    headless::run_headless(client, initial_session.id, prompt).await
}

async fn run_recipe_tui_mode(
    client: Client,
    initial_session: goose::session::Session,
    prompt: String,
) -> Result<()> {
    let config = TuiConfig::load()?;
    let mut state = AppState::new(initial_session.id.clone(), config, None, None);

    state.messages = initial_session
        .conversation
        .map(|c| c.messages().clone())
        .unwrap_or_default();

    let terminal = tui::init()?;
    let app = App::new();
    let event_handler = EventHandler::new();

    let result = run_recipe_event_loop(terminal, app, event_handler, state, client, prompt).await;

    tui::restore()?;
    result
}

async fn run_interactive_mode(
    client: Client,
    cwd: std::path::PathBuf,
    session: Option<String>,
) -> Result<()> {
    let initial_session = if let Some(id) = session {
        info!("Resuming agent session: {}", id);
        client.resume_agent(&id).await?
    } else {
        client
            .start_agent(cwd.to_string_lossy().to_string())
            .await?
    };

    let (provider, model) = configure_session_from_global(&client, &initial_session.id).await;

    let config = TuiConfig::load()?;
    let mut state = AppState::new(
        initial_session.id.clone(),
        config,
        Some(provider),
        model.clone(),
    );

    state.model_context_limit = model
        .as_ref()
        .and_then(|m| {
            goose::model::ModelConfig::new(m)
                .ok()
                .map(|c| c.context_limit())
        })
        .unwrap_or(goose_tui::DEFAULT_CONTEXT_LIMIT);

    state.messages = initial_session
        .conversation
        .map(|c| c.messages().clone())
        .unwrap_or_default();

    state.token_state.total_tokens = initial_session.total_tokens.unwrap_or(0);
    state.token_state.input_tokens = initial_session.input_tokens.unwrap_or(0);
    state.token_state.output_tokens = initial_session.output_tokens.unwrap_or(0);

    let terminal = tui::init()?;
    let app = App::new();
    let event_handler = EventHandler::new();

    let result = run_event_loop(terminal, app, event_handler, state, client).await;

    tui::restore()?;
    result
}
