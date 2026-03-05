mod action_handler;
mod app;
mod at_mention;
mod cli;
mod commands;
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
use clap::{CommandFactory, Parser, Subcommand};
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

// ---------------------------------------------------------------------------
// Configure session helper
// ---------------------------------------------------------------------------

pub async fn configure_session_from_global(
    client: &Client,
    session_id: &str,
    provider_override: Option<&str>,
    model_override: Option<&str>,
) -> (String, Option<String>) {
    let global_config = goose::config::Config::global();
    let provider = provider_override
        .map(|s| s.to_string())
        .or_else(|| global_config.get_goose_provider().ok())
        .unwrap_or_else(|| "openai".to_string());
    let model = model_override
        .map(|s| s.to_string())
        .or_else(|| global_config.get_goose_model().ok());

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

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=VALUE: no `=` found in `{s}`"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

#[derive(Parser)]
#[command(name = "goose", author, version, about = "An AI agent")]
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

    #[arg(long, value_name = "PROVIDER", help = "Override the LLM provider")]
    provider: Option<String>,

    #[arg(long, value_name = "MODEL", help = "Override the model to use")]
    model: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure goose settings
    Configure {},

    /// Display goose information
    Info {
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run one of the mcp servers bundled with goose
    #[command(hide = true)]
    Mcp { name: String },

    /// Start or manage sessions
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },

    /// Execute commands from an instruction file, text, or recipe
    Run {
        #[arg(
            short,
            long,
            value_name = "FILE",
            help = "Path to instruction file. Use - for stdin."
        )]
        instructions: Option<String>,

        #[arg(short = 't', long = "text", value_name = "TEXT")]
        text: Option<String>,

        #[arg(long, value_name = "RECIPE")]
        recipe: Option<String>,

        #[arg(long, value_name = "TEXT", help = "Additional system prompt")]
        system: Option<String>,

        #[arg(long, value_name = "KEY=VALUE", action = clap::ArgAction::Append, value_parser = parse_key_val)]
        params: Vec<(String, String)>,

        #[arg(short = 'q', long, help = "Quiet mode")]
        quiet: bool,

        #[arg(long, value_name = "PROVIDER")]
        provider: Option<String>,

        #[arg(long, value_name = "MODEL")]
        model: Option<String>,

        #[arg(long, help = "Run in headless mode")]
        headless: bool,
    },

    /// Recipe utilities
    Recipe {
        #[command(subcommand)]
        command: RecipeCommand,
    },

    /// Manage scheduled jobs
    #[command(visible_alias = "sched")]
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommand,
    },

    /// Manage gateways for external platform integrations
    #[command(visible_alias = "gw")]
    Gateway {
        #[command(subcommand)]
        command: GatewayCommand,
    },

    /// Update the goose CLI version
    Update {
        #[arg(short, long, help = "Update to canary version")]
        canary: bool,
        #[arg(short, long, help = "Re-configure goose during update")]
        reconfigure: bool,
    },

    /// Terminal-integrated goose session
    Term {
        #[command(subcommand)]
        command: TermCommand,
    },

    /// Manage local inference models
    #[command(visible_alias = "lm")]
    LocalModels {
        #[command(subcommand)]
        command: LocalModelsCommand,
    },

    /// Generate shell completion scripts
    Completion {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
        #[arg(long, default_value = "goose")]
        bin_name: String,
    },

    /// Start standalone server (powers the desktop app)
    Server {
        #[arg(long, short, default_value = "3000")]
        port: u16,
    },
}

#[derive(Subcommand)]
enum SessionCommand {
    /// List all available sessions
    List,
    /// Remove sessions
    Remove {
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        regex: Option<String>,
    },
    /// Export a session
    Export {
        #[arg(long)]
        id: Option<String>,
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Generate diagnostics for a session
    Diagnostics {
        #[arg(long)]
        id: Option<String>,
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum RecipeCommand {
    /// Validate a recipe file
    Validate { recipe_name: String },
    /// List available recipes
    List,
}

#[derive(Subcommand)]
enum ScheduleCommand {
    /// Add a new scheduled job
    Add {
        #[arg(long)]
        schedule_id: Option<String>,
        #[arg(long)]
        cron: String,
        #[arg(long = "recipe")]
        recipe_source: String,
    },
    /// List all scheduled jobs
    List,
    /// Remove a scheduled job by ID
    Remove { schedule_id: String },
    /// List sessions created by a schedule
    Sessions {
        schedule_id: String,
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Run a scheduled job immediately
    RunNow { schedule_id: String },
    /// Show cron expression help
    CronHelp,
}

#[derive(Subcommand)]
enum GatewayCommand {
    /// Show gateway status
    Status,
    /// Start a gateway
    Start {
        gateway_type: String,
        #[arg(long)]
        bot_token: Option<String>,
    },
    /// Stop a running gateway
    Stop { gateway_type: String },
    /// Generate a pairing code
    Pair { gateway_type: String },
}

#[derive(Subcommand)]
enum TermCommand {
    /// Print shell initialization script
    Init {
        #[arg(value_enum)]
        shell: commands::term::Shell,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        with_command_not_found: bool,
    },
    /// Run a prompt in the terminal session
    Run { prompt: Vec<String> },
    /// Print session info for prompt integration
    Info,
    /// Log a shell command to the session
    #[command(hide = true)]
    Log { command: String },
}

#[derive(Subcommand)]
enum LocalModelsCommand {
    /// Search HuggingFace for GGUF models
    Search {
        query: String,
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Download a GGUF model
    Download { spec: String },
    /// List downloaded local models
    List,
    /// Delete a downloaded local model
    Delete { id: String },
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Stdin helper
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli_args = Cli::parse();

    // Subcommands that don't need the embedded server
    match &cli_args.command {
        Some(Commands::Mcp { name }) => return run_mcp_server(name.clone()),
        Some(Commands::Server { port }) => return run_server(*port),
        Some(Commands::Configure {}) => {
            return tokio::runtime::Runtime::new()?
                .block_on(commands::configure::handle_configure());
        }
        Some(Commands::Info { verbose }) => {
            return commands::info::handle_info(*verbose);
        }
        Some(Commands::Update {
            canary,
            reconfigure,
        }) => {
            return tokio::runtime::Runtime::new()?
                .block_on(commands::update::update(*canary, *reconfigure));
        }
        Some(Commands::Session { command }) => {
            return tokio::runtime::Runtime::new()?.block_on(dispatch_session_command(command));
        }
        Some(Commands::Term { command }) => {
            return tokio::runtime::Runtime::new()?.block_on(dispatch_term_command(command));
        }
        Some(Commands::Recipe { command }) => {
            return dispatch_recipe_command(command);
        }
        Some(Commands::Schedule { command }) => {
            return tokio::runtime::Runtime::new()?.block_on(dispatch_schedule_command(command));
        }
        Some(Commands::Gateway { command }) => {
            return tokio::runtime::Runtime::new()?.block_on(dispatch_gateway_command(command));
        }
        Some(Commands::LocalModels { command }) => {
            return tokio::runtime::Runtime::new()?
                .block_on(dispatch_local_models_command(command));
        }
        Some(Commands::Completion { shell, bin_name }) => {
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, bin_name, &mut std::io::stdout());
            return Ok(());
        }
        Some(Commands::Run { .. }) | None => {
            // Fall through to TUI / embedded-server path
        }
    }

    // Handle `goose run` subcommand — extract inputs, then use TUI infrastructure
    let (text_input, recipe_path, headless, provider, model) = if let Some(Commands::Run {
        instructions,
        text,
        recipe,
        system: _system, // TODO: wire system prompt
        params: _params, // TODO: wire recipe params
        quiet,
        provider,
        model,
        headless,
    }) = cli_args.command
    {
        let extracted_text = match (instructions.as_deref(), &text) {
            (Some("-"), _) => {
                let mut contents = String::new();
                std::io::stdin()
                    .read_to_string(&mut contents)
                    .expect("Failed to read from stdin");
                Some(contents)
            }
            (Some(file), _) => Some(fs::read_to_string(file)?),
            (_, Some(t)) => Some(t.clone()),
            (None, None) => None,
        };
        let recipe_pb = recipe.map(PathBuf::from);
        let is_headless = headless || quiet || extracted_text.is_some();
        (extracted_text, recipe_pb, is_headless, provider, model)
    } else {
        // No subcommand — use top-level args
        let stdin_input = read_stdin_if_piped();
        let text_input = match (stdin_input, cli_args.text) {
            (Some(stdin), Some(text)) => Some(format!("{stdin}\n\n{text}")),
            (Some(stdin), None) => Some(stdin),
            (None, Some(text)) => Some(text),
            (None, None) => None,
        };
        (
            text_input,
            cli_args.recipe,
            cli_args.headless,
            cli_args.provider,
            cli_args.model,
        )
    };

    let secret_key = Uuid::new_v4().to_string();
    std::env::set_var("GOOSE_SERVER__SECRET_KEY", &secret_key);
    std::env::set_var("GOOSE_PORT", "0");

    tokio::runtime::Runtime::new()?.block_on(run_tui(
        cli_args.session,
        cli_args.name,
        cli_args.resume,
        recipe_path,
        headless,
        cli_args.cli,
        text_input,
        secret_key,
        provider,
        model,
    ))
}

// ---------------------------------------------------------------------------
// Subcommand dispatchers
// ---------------------------------------------------------------------------

async fn dispatch_session_command(command: &SessionCommand) -> Result<()> {
    match command {
        SessionCommand::List => {
            commands::session::handle_session_list("text".to_string(), false, None, None).await
        }
        SessionCommand::Remove { id, name, regex } => {
            commands::session::handle_session_remove(id.clone(), name.clone(), regex.clone()).await
        }
        SessionCommand::Export { id, output, format } => {
            let session_id = match id {
                Some(id) => id.clone(),
                None => {
                    let sm = goose::session::SessionManager::instance();
                    commands::session::prompt_interactive_session_selection(&sm).await?
                }
            };
            commands::session::handle_session_export(session_id, output.clone(), format.clone())
                .await
        }
        SessionCommand::Diagnostics { id, output } => {
            let session_id = match id {
                Some(id) => id.clone(),
                None => {
                    let sm = goose::session::SessionManager::instance();
                    commands::session::prompt_interactive_session_selection(&sm).await?
                }
            };
            commands::session::handle_diagnostics(&session_id, output.clone()).await
        }
    }
}

async fn dispatch_term_command(command: &TermCommand) -> Result<()> {
    match command {
        TermCommand::Init {
            shell,
            name,
            with_command_not_found,
        } => {
            commands::term::handle_term_init(shell.clone(), name.clone(), *with_command_not_found)
                .await
        }
        TermCommand::Run { prompt } => commands::term::handle_term_run(prompt.clone()).await,
        TermCommand::Info => commands::term::handle_term_info().await,
        TermCommand::Log { command } => commands::term::handle_term_log(command.clone()).await,
    }
}

fn dispatch_recipe_command(command: &RecipeCommand) -> Result<()> {
    match command {
        RecipeCommand::Validate { recipe_name } => commands::recipe::handle_validate(recipe_name),
        RecipeCommand::List => commands::recipe::handle_list("text", false),
    }
}

async fn dispatch_schedule_command(command: &ScheduleCommand) -> Result<()> {
    match command {
        ScheduleCommand::Add {
            schedule_id,
            cron,
            recipe_source,
        } => {
            let id = schedule_id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            commands::schedule::handle_schedule_add(id, cron.clone(), recipe_source.clone()).await
        }
        ScheduleCommand::List => commands::schedule::handle_schedule_list().await,
        ScheduleCommand::Remove { schedule_id } => {
            commands::schedule::handle_schedule_remove(schedule_id.clone()).await
        }
        ScheduleCommand::Sessions { schedule_id, limit } => {
            commands::schedule::handle_schedule_sessions(schedule_id.clone(), Some(*limit)).await
        }
        ScheduleCommand::RunNow { schedule_id } => {
            commands::schedule::handle_schedule_run_now(schedule_id.clone()).await
        }
        ScheduleCommand::CronHelp => commands::schedule::handle_schedule_cron_help().await,
    }
}

async fn dispatch_gateway_command(command: &GatewayCommand) -> Result<()> {
    match command {
        GatewayCommand::Status => commands::gateway::handle_gateway_status().await,
        GatewayCommand::Start {
            gateway_type,
            bot_token,
        } => {
            let platform_config = serde_json::json!({ "bot_token": bot_token });
            commands::gateway::handle_gateway_start(gateway_type.clone(), platform_config).await
        }
        GatewayCommand::Stop { gateway_type } => {
            commands::gateway::handle_gateway_stop(gateway_type.clone()).await
        }
        GatewayCommand::Pair { gateway_type } => {
            commands::gateway::handle_gateway_pair(gateway_type.clone()).await
        }
    }
}

async fn dispatch_local_models_command(command: &LocalModelsCommand) -> Result<()> {
    use goose::providers::local_inference::hf_models;
    use goose::providers::local_inference::local_model_registry::{
        get_registry, model_id_from_repo, LocalModelEntry,
    };

    match command {
        LocalModelsCommand::Search { query, limit } => {
            println!("Searching HuggingFace for '{}'...", query);
            let results = hf_models::search_gguf_models(query, *limit).await?;
            if results.is_empty() {
                println!("No GGUF models found.");
                return Ok(());
            }
            for model in &results {
                println!(
                    "\n{} (by {}) — {} downloads",
                    model.model_name, model.author, model.downloads
                );
                for file in &model.gguf_files {
                    let size = if file.size_bytes > 0 {
                        format!(
                            "{:.1}GB",
                            file.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
                        )
                    } else {
                        "unknown".to_string()
                    };
                    println!("  {} — {}", file.quantization, size);
                }
                println!(
                    "  Download: goose local-models download {}:<quantization>",
                    model.repo_id
                );
            }
            Ok(())
        }
        LocalModelsCommand::Download { spec } => {
            println!("Resolving {}...", spec);
            let (repo_id, file) = hf_models::resolve_model_spec(spec).await?;
            let model_id = model_id_from_repo(&repo_id, &file.quantization);
            let local_path =
                goose::config::paths::Paths::in_data_dir("models").join(&file.filename);

            println!("Downloading {} ...", model_id);

            let entry = LocalModelEntry {
                id: model_id.clone(),
                repo_id: repo_id.clone(),
                filename: file.filename.clone(),
                quantization: file.quantization.clone(),
                local_path: local_path.clone(),
                source_url: file.download_url.clone(),
                settings: Default::default(),
                size_bytes: file.size_bytes,
            };

            {
                let mut registry = get_registry()
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Failed to acquire registry lock"))?;
                registry.add_model(entry)?;
            }

            let manager = goose::download_manager::get_download_manager();
            manager
                .download_model(
                    format!("{}-model", model_id),
                    file.download_url,
                    local_path,
                    None,
                )
                .await?;

            loop {
                if let Some(progress) = manager.get_progress(&format!("{}-model", model_id)) {
                    match progress.status {
                        goose::download_manager::DownloadStatus::Downloading => {
                            print!(
                                "\r  {:.1}% ({:.0}MB / {:.0}MB)",
                                progress.progress_percent,
                                progress.bytes_downloaded as f64 / (1024.0 * 1024.0),
                                progress.total_bytes as f64 / (1024.0 * 1024.0),
                            );
                            use std::io::Write;
                            std::io::stdout().flush().ok();
                        }
                        goose::download_manager::DownloadStatus::Completed => {
                            println!("\nDownloaded: {}", model_id);
                            break;
                        }
                        goose::download_manager::DownloadStatus::Failed => {
                            let err = progress.error.unwrap_or_default();
                            anyhow::bail!("Download failed: {}", err);
                        }
                        goose::download_manager::DownloadStatus::Cancelled => {
                            println!("\nDownload cancelled.");
                            break;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Ok(())
        }
        LocalModelsCommand::List => {
            let registry = get_registry()
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to acquire registry lock"))?;
            let models = registry.list_models();
            if models.is_empty() {
                println!("No local models downloaded.");
                return Ok(());
            }
            println!("{:<50} {:<10} Downloaded", "ID", "Quant");
            println!("{}", "-".repeat(70));
            for m in models {
                println!(
                    "{:<50} {:<10} {}",
                    m.id,
                    m.quantization,
                    if m.is_downloaded() { "✓" } else { "✗" }
                );
            }
            Ok(())
        }
        LocalModelsCommand::Delete { id } => {
            let mut registry = get_registry()
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to acquire registry lock"))?;
            if let Some(entry) = registry.get_model(id) {
                if entry.local_path.exists() {
                    std::fs::remove_file(&entry.local_path)?;
                }
                registry.remove_model(id)?;
                println!("Deleted model: {}", id);
            } else {
                println!("Model not found: {}", id);
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// MCP / Server
// ---------------------------------------------------------------------------

fn run_mcp_server(name: String) -> Result<()> {
    use goose_mcp::mcp_server_runner::{serve, McpCommand};
    use goose_mcp::{AutoVisualiserRouter, ComputerControllerServer, MemoryServer, TutorialServer};
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
        }
        Ok(())
    })
}

fn run_server(port: u16) -> Result<()> {
    std::env::set_var("GOOSE_PORT", port.to_string());
    tokio::runtime::Runtime::new()?.block_on(goose_server::commands::agent::run())
}

// ---------------------------------------------------------------------------
// TUI runtime (embedded server + interactive/headless modes)
// ---------------------------------------------------------------------------

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
    provider_override: Option<String>,
    model_override: Option<String>,
) -> Result<()> {
    let _guard = setup_tui_logging()?;
    info!("Starting Goose...");

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

    let prov_ref = provider_override.as_deref();
    let model_ref = model_override.as_deref();

    let result = if let Some(recipe_path) = recipe {
        run_recipe_mode(client, cwd, recipe_path, headless, prov_ref, model_ref).await
    } else if let Some(prompt) = text_input {
        run_text_mode(
            client,
            cwd,
            resolved_session,
            name,
            prompt,
            prov_ref,
            model_ref,
        )
        .await
    } else if headless {
        anyhow::bail!("--headless requires either --recipe or --text (or piped stdin)")
    } else if cli_mode {
        run_cli_mode(client, cwd, resolved_session, name, prov_ref, model_ref).await
    } else {
        run_interactive_mode(client, cwd, resolved_session, name, prov_ref, model_ref).await
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
    provider_override: Option<&str>,
    model_override: Option<&str>,
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

    configure_session_from_global(
        &client,
        &initial_session.id,
        provider_override,
        model_override,
    )
    .await;

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
    provider_override: Option<&str>,
    model_override: Option<&str>,
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

    configure_session_from_global(
        &client,
        &initial_session.id,
        provider_override,
        model_override,
    )
    .await;

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
    provider_override: Option<&str>,
    model_override: Option<&str>,
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

    let (provider, model) = configure_session_from_global(
        &client,
        &initial_session.id,
        provider_override,
        model_override,
    )
    .await;

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
    provider_override: Option<&str>,
    model_override: Option<&str>,
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

    let (provider, model) = configure_session_from_global(
        &client,
        &initial_session.id,
        provider_override,
        model_override,
    )
    .await;

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
