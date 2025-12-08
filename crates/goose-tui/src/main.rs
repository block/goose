mod action_handler;
mod app;
mod at_mention;
mod cli;
mod components;
mod headless;
mod hidden_blocks;
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

    #[arg(short = 'n', long, value_name = "NAME", help = "Name for the session")]
    name: Option<String>,

    #[arg(
        short = 'r',
        long,
        help = "Resume the most recent session (or by --name)"
    )]
    resume: bool,

    #[arg(long, value_name = "FILE", help = "Run a recipe file")]
    recipe: Option<PathBuf>,

    #[arg(long, help = "Run in headless mode (plain text output, no TUI)")]
    headless: bool,

    #[arg(
        short = 't',
        long = "text",
        value_name = "TEXT",
        help = "Input text to send directly"
    )]
    text: Option<String>,

    #[arg(long, help = "Run in CLI mode (interactive terminal, no TUI)")]
    cli: bool,

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
    let cli_args = Cli::parse();

    if let Some(Commands::Mcp { name }) = cli_args.command {
        return run_mcp_server(name);
    }

    let stdin_input = read_stdin_if_piped();
    let text_input = match (stdin_input, cli_args.text) {
        (Some(stdin), Some(text)) => Some(format!("{stdin}\n\n{text}")),
        (Some(stdin), None) => Some(stdin),
        (None, Some(text)) => Some(text),
        (None, None) => None,
    };

    let secret_key = Uuid::new_v4().to_string();
    std::env::set_var("GOOSE_SERVER__SECRET_KEY", &secret_key);
    std::env::set_var("GOOSE_PORT", "0");

    tokio::runtime::Runtime::new()?.block_on(run_tui(
        cli_args.session,
        cli_args.name,
        cli_args.resume,
        cli_args.recipe,
        cli_args.headless,
        cli_args.cli,
        text_input,
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

async fn resolve_session_id(
    client: &Client,
    session_id: Option<String>,
    name: Option<String>,
    resume: bool,
) -> Result<Option<String>> {
    if let Some(id) = session_id {
        return Ok(Some(id));
    }

    if let Some(ref session_name) = name {
        let sessions = client.list_sessions().await?;
        let found = sessions
            .iter()
            .find(|s| s.name == *session_name || s.id == *session_name);

        if let Some(existing) = found {
            if resume {
                return Ok(Some(existing.id.clone()));
            }
        } else if resume {
            anyhow::bail!("No session found with name '{}'", session_name);
        }
    }

    if resume && name.is_none() {
        let sessions = client.list_sessions().await?;
        let session_id = sessions
            .first()
            .map(|s| s.id.clone())
            .ok_or_else(|| anyhow::anyhow!("No sessions found to resume"))?;
        return Ok(Some(session_id));
    }

    Ok(None)
}

#[allow(clippy::too_many_arguments)]
async fn run_tui(
    session: Option<String>,
    name: Option<String>,
    resume: bool,
    recipe: Option<PathBuf>,
    headless: bool,
    cli_mode: bool,
    text_input: Option<String>,
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

    let resolved_session = resolve_session_id(&client, session, name.clone(), resume).await?;

    let result = if let Some(recipe_path) = recipe {
        run_recipe_mode(client, cwd, recipe_path, headless).await
    } else if let Some(prompt) = text_input {
        run_text_mode(client, cwd, resolved_session, name, prompt).await
    } else if headless {
        anyhow::bail!("--headless requires either --recipe or --text (or piped stdin)")
    } else if cli_mode {
        run_cli_mode(client, cwd, resolved_session, name).await
    } else {
        run_interactive_mode(client, cwd, resolved_session, name).await
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
    let config = TuiConfig::load()?;
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

    let cwd_analysis = if config.smart_context {
        action_handler::fetch_cwd_analysis_sync(&client, &initial_session.id).await
    } else {
        None
    };

    let use_headless = headless || !std::io::stdout().is_terminal();

    if use_headless {
        headless::run_headless(client, initial_session.id, prompt, cwd_analysis).await
    } else {
        run_recipe_tui_mode(client, initial_session, prompt, cwd_analysis).await
    }
}

async fn run_text_mode(
    client: Client,
    cwd: std::path::PathBuf,
    session: Option<String>,
    name: Option<String>,
    prompt: String,
) -> Result<()> {
    info!("Running with text input");

    let config = TuiConfig::load()?;

    let (initial_session, is_new_session) = if let Some(id) = session {
        info!("Resuming session: {}", id);
        (client.resume_agent(&id).await?, false)
    } else {
        let new_session = client
            .start_agent(cwd.to_string_lossy().to_string())
            .await?;

        if let Some(ref session_name) = name {
            if let Err(e) = client
                .update_session_name(&new_session.id, session_name)
                .await
            {
                tracing::warn!("Failed to set session name: {}", e);
            }
        }

        (new_session, true)
    };

    configure_session_from_global(&client, &initial_session.id).await;

    let has_messages = initial_session
        .conversation
        .as_ref()
        .is_some_and(|c| !c.messages().is_empty());

    let cwd_analysis = if config.smart_context && is_new_session && !has_messages {
        action_handler::fetch_cwd_analysis_sync(&client, &initial_session.id).await
    } else {
        None
    };

    headless::run_headless(client, initial_session.id, prompt, cwd_analysis).await
}

async fn run_recipe_tui_mode(
    client: Client,
    initial_session: goose::session::Session,
    prompt: String,
    cwd_analysis: Option<String>,
) -> Result<()> {
    let config = TuiConfig::load()?;
    let mut state = AppState::new(initial_session.id.clone(), config, None, None);

    state.messages = initial_session
        .conversation
        .map(|c| c.messages().clone())
        .unwrap_or_default();

    if let Some(analysis) = cwd_analysis {
        state.cwd_analysis = state::CwdAnalysisState::Complete(analysis);
    }

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
    name: Option<String>,
) -> Result<()> {
    let config = TuiConfig::load()?;
    let event_handler = EventHandler::new();
    let tx = event_handler.sender();

    let (initial_session, is_new_session) = if let Some(id) = session {
        info!("Resuming agent session: {}", id);
        (client.resume_agent(&id).await?, false)
    } else {
        let new_session = client
            .start_agent(cwd.to_string_lossy().to_string())
            .await?;

        if let Some(ref session_name) = name {
            if let Err(e) = client
                .update_session_name(&new_session.id, session_name)
                .await
            {
                tracing::warn!("Failed to set session name: {}", e);
            }
        }

        (new_session, true)
    };

    let (provider, model) = configure_session_from_global(&client, &initial_session.id).await;

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

    if state.config.smart_context && is_new_session && state.messages.is_empty() {
        state.cwd_analysis = state::CwdAnalysisState::Pending;
        action_handler::spawn_cwd_analysis(&initial_session.id, &client, &tx);
    }

    let terminal = tui::init()?;
    let app = App::new();

    let result = run_event_loop(terminal, app, event_handler, state, client).await;

    tui::restore()?;
    result
}

async fn run_cli_mode(
    client: Client,
    cwd: std::path::PathBuf,
    session: Option<String>,
    name: Option<String>,
) -> Result<()> {
    info!("Starting CLI mode");

    let config = TuiConfig::load()?;

    let (initial_session, is_new_session) = if let Some(id) = session {
        info!("Resuming session: {}", id);
        (client.resume_agent(&id).await?, false)
    } else {
        let new_session = client
            .start_agent(cwd.to_string_lossy().to_string())
            .await?;

        if let Some(ref session_name) = name {
            if let Err(e) = client
                .update_session_name(&new_session.id, session_name)
                .await
            {
                tracing::warn!("Failed to set session name: {}", e);
            }
        }

        (new_session, true)
    };

    let (provider, model) = configure_session_from_global(&client, &initial_session.id).await;

    let context_limit = model
        .as_ref()
        .and_then(|m| {
            goose::model::ModelConfig::new(m)
                .ok()
                .map(|c| c.context_limit())
        })
        .unwrap_or(goose_tui::DEFAULT_CONTEXT_LIMIT);

    let has_messages = initial_session
        .conversation
        .as_ref()
        .is_some_and(|c| !c.messages().is_empty());

    let cwd_analysis = if config.smart_context && is_new_session && !has_messages {
        action_handler::fetch_cwd_analysis_sync(&client, &initial_session.id).await
    } else {
        None
    };

    cli::run_cli(
        client,
        initial_session.id,
        provider,
        model,
        context_limit,
        cwd_analysis,
    )
    .await
}
