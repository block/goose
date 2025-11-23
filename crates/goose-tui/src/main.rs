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
use services::client::Client;
use services::config::TuiConfig;
use services::events::{Event, EventHandler};
use state::action::Action;
use state::AppState;
use std::env;
use std::fs;
use tokio::sync::mpsc;
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
        .update_provider(&initial_session.id, provider, model.clone())
        .await
    {
        tracing::error!("Failed to update provider: {}", e);
    }

    for ext in goose::config::get_enabled_extensions() {
        if let Err(e) = client.add_extension(&initial_session.id, ext.clone()).await {
            tracing::error!("Failed to add extension {}: {}", ext.name(), e);
        }
    }

    let config = TuiConfig::load()?;
    let mut state = AppState::new(initial_session.id.clone(), config);

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
        match &action {
            Action::SendMessage(message_to_send) => {
                let client = client.clone();
                let tx = tx.clone();
                let mut messages_snapshot = state.messages.clone();
                messages_snapshot.push(message_to_send.clone());
                let session_id = state.session_id.clone();

                let task = tokio::spawn(async move {
                    if let Err(e) = client
                        .reply(messages_snapshot, session_id, tx.clone())
                        .await
                    {
                        let _ = tx.send(Event::Error(e.to_string()));
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
            Action::Quit => {
                state::reducer::update(state, action);
                return true;
            }
            Action::Interrupt => {
                if let Some(task) = reply_task.take() {
                    task.abort();
                }
            }
            _ => {}
        }
        state::reducer::update(state, action);
    }
    false
}
