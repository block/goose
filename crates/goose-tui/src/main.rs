mod app;
mod components;
mod headless;
mod services;
mod state;
mod tui;
mod utils;

use anyhow::Result;
use app::App;
use clap::{Parser, Subcommand};
use components::Component;
use goose_client::Client;
use services::config::TuiConfig;
use services::events::{Event, EventHandler};
use state::action::Action;
use state::AppState;
use std::fs;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use uuid::Uuid;

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

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Mcp { name }) = cli.command {
        use goose_mcp::mcp_server_runner::{serve, McpCommand};
        use goose_mcp::{
            AutoVisualiserRouter, ComputerControllerServer, DeveloperServer, MemoryServer,
            TutorialServer,
        };
        use std::str::FromStr;

        return tokio::runtime::Runtime::new()?.block_on(async {
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
        });
    }

    let secret_key = Uuid::new_v4().to_string();
    std::env::set_var("GOOSE_SERVER__SECRET_KEY", &secret_key);
    std::env::set_var("GOOSE_PORT", "0");

    tokio::runtime::Runtime::new()?.block_on(run_tui(
        cli.session,
        cli.recipe,
        cli.headless,
        secret_key,
    ))
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

    let result = if let Some(recipe_path) = recipe {
        run_recipe_mode(client, cwd, recipe_path, headless).await
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

    let global_config = goose::config::Config::global();
    let provider = global_config
        .get_goose_provider()
        .unwrap_or_else(|_| "openai".to_string());
    let model = global_config.get_goose_model().ok();

    if let Err(e) = client
        .update_provider(&initial_session.id, provider, model)
        .await
    {
        tracing::error!("Failed to update provider: {e}");
    }

    for ext in goose::config::get_enabled_extensions() {
        if let Err(e) = client.add_extension(&initial_session.id, ext.clone()).await {
            tracing::error!("Failed to add extension {}: {}", ext.name(), e);
        }
    }

    let use_headless = headless || !std::io::stdout().is_terminal();

    if use_headless {
        headless::run_headless(client, initial_session.id, prompt).await
    } else {
        run_recipe_tui_mode(client, initial_session, prompt).await
    }
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

    let global_config = goose::config::Config::global();
    let provider = global_config
        .get_goose_provider()
        .unwrap_or_else(|_| "openai".to_string());
    let model = global_config.get_goose_model().ok();

    if let Err(e) = client
        .update_provider(&initial_session.id, provider.clone(), model.clone())
        .await
    {
        tracing::error!("Failed to update provider: {e}");
    }

    for ext in goose::config::get_enabled_extensions() {
        if let Err(e) = client.add_extension(&initial_session.id, ext.clone()).await {
            tracing::error!("Failed to add extension {}: {}", ext.name(), e);
        }
    }

    let config = TuiConfig::load()?;
    let mut state = AppState::new(
        initial_session.id.clone(),
        config,
        Some(provider.clone()),
        model.clone(),
    );

    if let Some(ref model_name) = model {
        state.model_context_limit = goose::model::ModelConfig::new(model_name)
            .map(|config| config.context_limit())
            .unwrap_or(128_000);
    } else {
        state.model_context_limit = 128_000;
    }

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

fn needs_redraw(event: &Event, state: &AppState) -> bool {
    match event {
        Event::Tick => state.is_working,
        _ => true,
    }
}

async fn run_event_loop(
    mut terminal: tui::Tui,
    mut app: App<'_>,
    mut event_handler: EventHandler,
    mut state: AppState,
    client: Client,
) -> Result<()> {
    let tx = event_handler.sender();
    let c_tools = client.clone();
    let s_id = state.session_id.clone();
    let tx_tools = tx.clone();

    tokio::spawn(async move {
        if let Ok(tools) = c_tools.get_tools(&s_id).await {
            let _ = tx_tools.send(Event::ToolsLoaded(tools));
        }
    });

    let mut reply_task: Option<tokio::task::JoinHandle<()>> = None;
    let mut should_redraw = true;

    loop {
        if state.needs_refresh {
            terminal.clear()?;
            state.needs_refresh = false;
            should_redraw = true;
        }

        if should_redraw {
            terminal.draw(|f| {
                app.render(f, f.area(), &state);
            })?;
            should_redraw = false;
        }

        let Some(event) = event_handler.next().await else {
            break;
        };

        if needs_redraw(&event, &state) {
            should_redraw = true;
        }

        if process_event(event, &mut app, &mut state, &client, &tx, &mut reply_task) {
            break;
        }

        let mut quit = false;
        while let Some(event) = event_handler.try_next() {
            if needs_redraw(&event, &state) {
                should_redraw = true;
            }
            if process_event(event, &mut app, &mut state, &client, &tx, &mut reply_task) {
                quit = true;
                break;
            }
        }
        if quit {
            break;
        }
    }

    Ok(())
}

async fn run_recipe_event_loop(
    mut terminal: tui::Tui,
    mut app: App<'_>,
    mut event_handler: EventHandler,
    mut state: AppState,
    client: Client,
    prompt: String,
) -> Result<()> {
    let tx = event_handler.sender();

    let user_message = goose::conversation::message::Message::user().with_text(&prompt);
    state.messages.push(user_message.clone());
    state.is_working = true;

    let client_clone = client.clone();
    let tx_clone = tx.clone();
    let session_id = state.session_id.clone();
    let messages_snapshot = state.messages.clone();

    tokio::spawn(async move {
        match client_clone.reply(messages_snapshot, session_id).await {
            Ok(mut stream) => {
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(msg) => {
                            let _ = tx_clone.send(Event::Server(std::sync::Arc::new(msg)));
                        }
                        Err(e) => {
                            let _ = tx_clone.send(Event::Error(e.to_string()));
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx_clone.send(Event::Error(e.to_string()));
            }
        }
    });

    let mut reply_task: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        if state.needs_refresh {
            terminal.clear()?;
            state.needs_refresh = false;
        }

        terminal.draw(|f| {
            app.render(f, f.area(), &state);
        })?;

        let Some(event) = event_handler.next().await else {
            break;
        };

        if let Event::Input(key) = &event {
            if key.code == crossterm::event::KeyCode::Char('c')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                break;
            }
        }

        if let Event::Server(msg) = &event {
            if let goose_server::routes::reply::MessageEvent::Finish { .. } = msg.as_ref() {
                state::reducer::update(&mut state, Action::ServerMessage(msg.clone()));
                terminal.draw(|f| {
                    app.render(f, f.area(), &state);
                })?;
                break;
            }
        }

        if process_event(event, &mut app, &mut state, &client, &tx, &mut reply_task) {
            break;
        }

        while let Some(event) = event_handler.try_next() {
            if let Event::Server(msg) = &event {
                if let goose_server::routes::reply::MessageEvent::Finish { .. } = msg.as_ref() {
                    state::reducer::update(&mut state, Action::ServerMessage(msg.clone()));
                    terminal.draw(|f| {
                        app.render(f, f.area(), &state);
                    })?;
                    return Ok(());
                }
            }

            if process_event(event, &mut app, &mut state, &client, &tx, &mut reply_task) {
                break;
            }
        }
    }

    Ok(())
}

fn process_event(
    event: Event,
    app: &mut App,
    state: &mut AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
    reply_task: &mut Option<tokio::task::JoinHandle<()>>,
) -> bool {
    if let Ok(Some(action)) = app.handle_event(&event, state) {
        if handle_action(&action, state, client, tx, reply_task) {
            state::reducer::update(state, action);
            return true;
        }
        let was_copy_mode = state.copy_mode;
        state::reducer::update(state, action);
        if state.copy_mode != was_copy_mode {
            let _ = tui::set_mouse_capture(!state.copy_mode);
        }
    }
    false
}

#[allow(clippy::too_many_lines)]
fn handle_action(
    action: &Action,
    state: &AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
    reply_task: &mut Option<tokio::task::JoinHandle<()>>,
) -> bool {
    match action {
        Action::SendMessage(message_to_send) => {
            let client = client.clone();
            let tx = tx.clone();
            let mut messages_snapshot = state.messages.clone();
            messages_snapshot.push(message_to_send.clone());
            let session_id = state.session_id.clone();

            let task = tokio::spawn(async move {
                match client.reply(messages_snapshot, session_id).await {
                    Ok(mut stream) => {
                        while let Some(result) = stream.next().await {
                            match result {
                                Ok(msg) => {
                                    let _ = tx.send(Event::Server(std::sync::Arc::new(msg)));
                                }
                                Err(e) => {
                                    let _ = tx.send(Event::Error(e.to_string()));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(e.to_string()));
                    }
                }
            });
            *reply_task = Some(task);
        }
        Action::ResumeSession(id) => {
            let client = client.clone();
            let tx = tx.clone();
            let id = id.clone();
            tokio::spawn(async move {
                match client.resume_agent(&id).await {
                    Ok(s) => {
                        let _ = tx.send(Event::SessionResumed(Box::new(s)));
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(e.to_string()));
                    }
                }
            });
        }
        Action::CreateNewSession => {
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let cwd = std::env::current_dir().unwrap_or_default();
                match client.start_agent(cwd.to_string_lossy().to_string()).await {
                    Ok(s) => {
                        // Configure the new session with global defaults
                        let global_config = goose::config::Config::global();
                        let provider = global_config
                            .get_goose_provider()
                            .unwrap_or_else(|_| "openai".to_string());
                        let model = global_config.get_goose_model().ok();

                        if let Err(e) = client.update_provider(&s.id, provider, model).await {
                            let _ =
                                tx.send(Event::Error(format!("Failed to update provider: {e}")));
                        }

                        for ext in goose::config::get_enabled_extensions() {
                            if let Err(e) = client.add_extension(&s.id, ext.clone()).await {
                                let _ = tx.send(Event::Error(format!(
                                    "Failed to add extension {}: {e}",
                                    ext.name()
                                )));
                            }
                        }

                        let _ = tx.send(Event::SessionResumed(Box::new(s)));
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(e.to_string()));
                    }
                }
            });
        }
        Action::OpenSessionPicker => {
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                match client.list_sessions().await {
                    Ok(sessions) => {
                        let _ = tx.send(Event::SessionsList(sessions));
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(e.to_string()));
                    }
                }
            });
        }
        Action::OpenConfig => {
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                // Fetch providers
                match client.get_providers().await {
                    Ok(providers) => {
                        let _ = tx.send(Event::ProvidersLoaded(providers));
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(e.to_string()));
                    }
                }
                // Fetch extensions
                match client.get_extensions().await {
                    Ok(extensions) => {
                        let _ = tx.send(Event::ExtensionsLoaded(extensions));
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(e.to_string()));
                    }
                }
                // Fetch config (for active provider)
                match client.read_config().await {
                    Ok(config) => {
                        let _ = tx.send(Event::ConfigLoaded(config));
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(e.to_string()));
                    }
                }
            });
        }
        Action::FetchModels(provider) => {
            let client = client.clone();
            let tx = tx.clone();
            let p = provider.clone();
            tokio::spawn(async move {
                match client.get_provider_models(&p).await {
                    Ok(models) => {
                        let _ = tx.send(Event::ModelsLoaded {
                            provider: p,
                            models,
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch models for {}: {}", p, e);
                    }
                }
            });
        }
        Action::UpdateProvider { provider, model } => {
            let client = client.clone();
            let tx = tx.clone();
            let session_id = state.session_id.clone();
            let p = provider.clone();
            let m = model.clone();
            tokio::spawn(async move {
                // 1. Update session
                if let Err(e) = client
                    .update_provider(&session_id, p.clone(), Some(m.clone()))
                    .await
                {
                    let _ = tx.send(Event::Error(format!(
                        "Failed to update session provider: {e}"
                    )));
                    return;
                }
                // 2. Update persistent config
                if let Err(e) = client
                    .upsert_config("GOOSE_PROVIDER", serde_json::json!(p), false)
                    .await
                {
                    let _ = tx.send(Event::Error(format!(
                        "Failed to update config provider: {e}"
                    )));
                }
                if let Err(e) = client
                    .upsert_config("GOOSE_MODEL", serde_json::json!(m), false)
                    .await
                {
                    let _ = tx.send(Event::Error(format!("Failed to update config model: {e}")));
                }
            });
        }
        Action::ToggleExtension { name, enabled } => {
            let client = client.clone();
            let tx = tx.clone();
            let session_id = state.session_id.clone();
            let ext_name = name.clone();
            let enabled = *enabled;

            let ext_config = state
                .extensions
                .iter()
                .find(|e| e.config.name() == ext_name)
                .map(|e| e.config.clone());

            if let Some(config) = ext_config {
                tokio::spawn(async move {
                    if enabled {
                        // Enable
                        if let Err(e) = client.add_extension(&session_id, config.clone()).await {
                            let _ = tx.send(Event::Error(format!(
                                "Failed to enable extension in session: {e}"
                            )));
                        }
                        if let Err(e) = client.add_config_extension(ext_name, config, true).await {
                            let _ = tx.send(Event::Error(format!(
                                "Failed to enable extension in config: {e}"
                            )));
                        }
                    } else {
                        // Disable
                        if let Err(e) = client.remove_extension(&session_id, &ext_name).await {
                            let _ = tx.send(Event::Error(format!(
                                "Failed to disable extension in session: {e}"
                            )));
                        }
                        if let Err(e) = client.add_config_extension(ext_name, config, false).await {
                            let _ = tx.send(Event::Error(format!(
                                "Failed to disable extension in config: {e}"
                            )));
                        }
                    }

                    // Refresh
                    match client.get_extensions().await {
                        Ok(extensions) => {
                            let _ = tx.send(Event::ExtensionsLoaded(extensions));
                        }
                        Err(e) => {
                            let _ = tx.send(Event::Error(e.to_string()));
                        }
                    }
                });
            }
        }
        Action::ForkFromMessage(msg_idx) => {
            let client = client.clone();
            let tx = tx.clone();
            let session_id = state.session_id.clone();
            let msg_idx = *msg_idx;

            tokio::spawn(async move {
                let exported = match client.export_session(&session_id).await {
                    Ok(json) => {
                        tracing::debug!(
                            "Fork: exported session (first 500 chars): {}",
                            &json[..json.len().min(500)]
                        );
                        json
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(format!("Export failed: {e}")));
                        return;
                    }
                };

                let mut session: serde_json::Value = match serde_json::from_str(&exported) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("Fork: failed to parse exported JSON: {e}");
                        let _ = tx.send(Event::Error(format!("Parse failed: {e}")));
                        return;
                    }
                };

                let original_count = session
                    .get("conversation")
                    .and_then(|c| c.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);

                if let Some(conv) = session.get_mut("conversation") {
                    if let Some(messages) = conv.as_array_mut() {
                        messages.truncate(msg_idx + 1);
                    }
                }

                let new_count = session
                    .get("conversation")
                    .and_then(|c| c.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);

                tracing::info!(
                    "Fork: truncating from {} to {} messages (up to index {})",
                    original_count,
                    new_count,
                    msg_idx
                );

                if let Some(name) = session.get("name").and_then(|n| n.as_str()) {
                    session["name"] = serde_json::json!(format!("Fork: {}", name));
                }

                let modified_json = serde_json::to_string(&session).unwrap_or_default();
                tracing::info!(
                    "Fork: sending import request with {} bytes",
                    modified_json.len()
                );
                let forked = match client.import_session(&modified_json).await {
                    Ok(s) => {
                        tracing::info!("Fork: import succeeded, new session id: {}", s.id);
                        s
                    }
                    Err(e) => {
                        tracing::error!("Fork: import failed: {e}");
                        let _ = tx.send(Event::Error(format!("Import failed: {e}")));
                        return;
                    }
                };

                tracing::info!(
                    "Fork: imported session {} with {} messages",
                    forked.id,
                    forked
                        .conversation
                        .as_ref()
                        .map(|c| c.messages().len())
                        .unwrap_or(0)
                );

                match client.resume_agent(&forked.id).await {
                    Ok(s) => {
                        let _ = tx.send(Event::SessionResumed(Box::new(s)));
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(format!("Resume failed: {e}")));
                    }
                }
            });
        }
        Action::Quit => {
            return true;
        }
        Action::Interrupt => {
            if let Some(task) = reply_task.take() {
                task.abort();
            }
        }
        _ => {}
    }
    false
}
