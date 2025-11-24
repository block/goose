mod app;
mod components;
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
use std::env;
use std::fs;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
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
    Tui {
        #[arg(long)]
        session: Option<String>,
    },
    Mcp {
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
        .with_ansi(false)
        .init();

    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Tui { session: None }) {
        Commands::Tui { session } => run_tui(session).await,
        Commands::Mcp { name } => {
            goose_server::logging::setup_logging(Some(&format!("mcp-{name}")))?;
            goose_mcp::mcp_server_runner::run_mcp_server(&name).await?;
            Ok(())
        }
    }
}

async fn run_tui(session: Option<String>) -> Result<()> {
    let _guard = setup_tui_logging()?;
    info!("Starting Goose TUI...");

    let secret_key = Uuid::new_v4().to_string();
    env::set_var("GOOSE_SERVER__SECRET_KEY", &secret_key);
    env::set_var("GOOSE_PORT", "0");

    info!("Initializing embedded server...");

    let (server_app, listener) = goose_server::commands::agent::build_app().await?;
    let port = listener.local_addr()?.port();
    info!("Embedded server bound to port: {}", port);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, server_app).await {
            tracing::error!("Server error: {}", e);
        }
    });

    let client = Client::new(port, secret_key.clone());
    let cwd = std::env::current_dir()?;

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
        Some(provider.clone()), // Pass the active provider
        model.clone(),          // Pass the active model
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

    run_event_loop(terminal, app, event_handler, state, client).await
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

    loop {
        terminal.draw(|f| {
            app.render(f, f.area(), &state);
        })?;

        let event = event_handler.next().await.unwrap();
        if process_event(event, &mut app, &mut state, &client, &tx, &mut reply_task) {
            break;
        }

        let mut quit = false;
        while let Some(event) = event_handler.try_next() {
            if process_event(event, &mut app, &mut state, &client, &tx, &mut reply_task) {
                quit = true;
                break;
            }
        }
        if quit {
            break;
        }
    }

    tui::restore()?;
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
        state::reducer::update(state, action);
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
