use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use goose::config::{Config, ExtensionConfig};

use crate::commands::bench::agent_generator;
use crate::commands::configure::handle_configure;
use crate::commands::info::handle_info;
use crate::commands::mcp::run_server;
use crate::commands::project::{handle_project_default, handle_projects_interactive};
use crate::commands::recipe::{handle_deeplink, handle_list, handle_validate};
// Import the new handlers from commands::schedule
use crate::commands::schedule::{
    handle_schedule_add, handle_schedule_cron_help, handle_schedule_list, handle_schedule_remove,
    handle_schedule_run_now, handle_schedule_services_status, handle_schedule_services_stop,
    handle_schedule_sessions,
};
use crate::commands::session::{handle_session_list, handle_session_remove};
use crate::recipes::extract_from_cli::extract_recipe_info_from_cli;
use crate::recipes::recipe::{explain_recipe, render_recipe_as_yaml};
use crate::session;
use crate::session::{build_session, SessionBuilderConfig, SessionSettings};
use goose_bench::bench_config::BenchRunConfig;
use goose_bench::runners::bench_runner::BenchRunner;
use goose_bench::runners::eval_runner::EvalRunner;
use goose_bench::runners::metric_aggregator::MetricAggregator;
use goose_bench::runners::model_runner::ModelRunner;
use std::io::Read;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, display_name = "", about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Args, Debug)]
#[group(required = false, multiple = false)]
struct Identifier {
    #[arg(
        short,
        long,
        value_name = "NAME",
        help = "Name for the chat session (e.g., 'project-x')",
        long_help = "Specify a name for your chat session. When used with --resume, will resume this specific session if it exists.",
        alias = "id"
    )]
    name: Option<String>,

    #[arg(
        short,
        long,
        value_name = "PATH",
        help = "Path for the chat session (e.g., './playground.jsonl')",
        long_help = "Specify a path for your chat session. When used with --resume, will resume this specific session if it exists."
    )]
    path: Option<PathBuf>,
}

fn extract_identifier(identifier: Identifier) -> session::Identifier {
    if let Some(name) = identifier.name {
        session::Identifier::Name(name)
    } else if let Some(path) = identifier.path {
        session::Identifier::Path(path)
    } else {
        unreachable!()
    }
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    match s.split_once('=') {
        Some((key, value)) => Ok((key.to_string(), value.to_string())),
        None => Err(format!("invalid KEY=VALUE: {}", s)),
    }
}

#[derive(Subcommand)]
enum SessionCommand {
    #[command(about = "List all available sessions")]
    List {
        #[arg(short, long, help = "List all available sessions")]
        verbose: bool,

        #[arg(
            short,
            long,
            help = "Output format (text, json)",
            default_value = "text"
        )]
        format: String,

        #[arg(
            long = "ascending",
            help = "Sort by date in ascending order (oldest first)",
            long_help = "Sort sessions by date in ascending order (oldest first). Default is descending order (newest first)."
        )]
        ascending: bool,
    },
    #[command(about = "Remove sessions. Runs interactively if no ID or regex is provided.")]
    Remove {
        #[arg(short, long, help = "Session ID to be removed (optional)")]
        id: Option<String>,
        #[arg(short, long, help = "Regex for removing matched sessions (optional)")]
        regex: Option<String>,
    },
    #[command(about = "Export a session to Markdown format")]
    Export {
        #[command(flatten)]
        identifier: Option<Identifier>,

        #[arg(
            short,
            long,
            help = "Output file path (default: stdout)",
            long_help = "Path to save the exported Markdown. If not provided, output will be sent to stdout"
        )]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum SchedulerCommand {
    #[command(about = "Add a new scheduled job")]
    Add {
        #[arg(long, help = "Unique ID for the job")]
        id: String,
        #[arg(
            long,
            help = "Cron expression for the schedule",
            long_help = "Cron expression for when to run the job. Examples:\n  '0 * * * *'     - Every hour at minute 0\n  '0 */2 * * *'   - Every 2 hours\n  '@hourly'       - Every hour (shorthand)\n  '0 9 * * *'     - Every day at 9:00 AM\n  '0 9 * * 1'     - Every Monday at 9:00 AM\n  '0 0 1 * *'     - First day of every month at midnight"
        )]
        cron: String,
        #[arg(
            long,
            help = "Recipe source (path to file, or base64 encoded recipe string)"
        )]
        recipe_source: String,
    },
    #[command(about = "List all scheduled jobs")]
    List {},
    #[command(about = "Remove a scheduled job by ID")]
    Remove {
        #[arg(long, help = "ID of the job to remove")] // Changed from positional to named --id
        id: String,
    },
    /// List sessions created by a specific schedule
    #[command(about = "List sessions created by a specific schedule")]
    Sessions {
        /// ID of the schedule
        #[arg(long, help = "ID of the schedule")] // Explicitly make it --id
        id: String,
        /// Maximum number of sessions to return
        #[arg(long, help = "Maximum number of sessions to return")]
        limit: Option<u32>,
    },
    /// Run a scheduled job immediately
    #[command(about = "Run a scheduled job immediately")]
    RunNow {
        /// ID of the schedule to run
        #[arg(long, help = "ID of the schedule to run")] // Explicitly make it --id
        id: String,
    },
    /// Check status of Temporal services (temporal scheduler only)
    #[command(about = "Check status of Temporal services")]
    ServicesStatus {},
    /// Stop Temporal services (temporal scheduler only)
    #[command(about = "Stop Temporal services")]
    ServicesStop {},
    /// Show cron expression examples and help
    #[command(about = "Show cron expression examples and help")]
    CronHelp {},
}

#[derive(Subcommand)]
pub enum BenchCommand {
    #[command(name = "init-config", about = "Create a new starter-config")]
    InitConfig {
        #[arg(short, long, help = "filename with extension for generated config")]
        name: String,
    },

    #[command(about = "Run all benchmarks from a config")]
    Run {
        #[arg(
            short,
            long,
            help = "A config file generated by the config-init command"
        )]
        config: PathBuf,
    },

    #[command(about = "List all available selectors")]
    Selectors {
        #[arg(
            short,
            long,
            help = "A config file generated by the config-init command"
        )]
        config: Option<PathBuf>,
    },

    #[command(name = "eval-model", about = "Run an eval of model")]
    EvalModel {
        #[arg(short, long, help = "A serialized config file for the model only.")]
        config: String,
    },

    #[command(name = "exec-eval", about = "run a single eval")]
    ExecEval {
        #[arg(short, long, help = "A serialized config file for the eval only.")]
        config: String,
    },

    #[command(
        name = "generate-leaderboard",
        about = "Generate a leaderboard CSV from benchmark results"
    )]
    GenerateLeaderboard {
        #[arg(
            short,
            long,
            help = "Path to the benchmark directory containing model evaluation results"
        )]
        benchmark_dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum RecipeCommand {
    /// Validate a recipe file
    #[command(about = "Validate a recipe")]
    Validate {
        /// Recipe name to get recipe file to validate
        #[arg(help = "recipe name to get recipe file or full path to the recipe file to validate")]
        recipe_name: String,
    },

    /// Generate a deeplink for a recipe file
    #[command(about = "Generate a deeplink for a recipe")]
    Deeplink {
        /// Recipe name to get recipe file to generate deeplink
        #[arg(
            help = "recipe name to get recipe file or full path to the recipe file to generate deeplink"
        )]
        recipe_name: String,
    },

    /// List available recipes
    #[command(about = "List available recipes")]
    List {
        /// Output format (text, json)
        #[arg(
            long = "format",
            value_name = "FORMAT",
            help = "Output format (text, json)",
            default_value = "text"
        )]
        format: String,

        /// Show verbose information including recipe descriptions
        #[arg(
            short,
            long,
            help = "Show verbose information including recipe descriptions"
        )]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum Command {
    /// Configure Goose settings
    #[command(about = "Configure Goose settings")]
    Configure {},

    /// Display Goose configuration information
    #[command(about = "Display Goose information")]
    Info {
        /// Show verbose information including current configuration
        #[arg(short, long, help = "Show verbose information including config.yaml")]
        verbose: bool,
    },

    /// Manage system prompts and behaviors
    #[command(about = "Run one of the mcp servers bundled with goose")]
    Mcp { name: String },

    /// Start or resume interactive chat sessions
    #[command(
        about = "Start or resume interactive chat sessions",
        visible_alias = "s"
    )]
    Session {
        #[command(subcommand)]
        command: Option<SessionCommand>,
        /// Identifier for the chat session
        #[command(flatten)]
        identifier: Option<Identifier>,

        /// Resume a previous session
        #[arg(
            short,
            long,
            help = "Resume a previous session (last used or specified by --name)",
            long_help = "Continue from a previous chat session. If --name or --path is provided, resumes that specific session. Otherwise resumes the last used session."
        )]
        resume: bool,

        /// Show message history when resuming
        #[arg(
            long,
            help = "Show previous messages when resuming a session",
            requires = "resume"
        )]
        history: bool,

        /// Enable debug output mode
        #[arg(
            long,
            help = "Enable debug output mode with full content and no truncation",
            long_help = "When enabled, shows complete tool responses without truncation and full paths."
        )]
        debug: bool,

        /// Maximum number of consecutive identical tool calls allowed
        #[arg(
            long = "max-tool-repetitions",
            value_name = "NUMBER",
            help = "Maximum number of consecutive identical tool calls allowed",
            long_help = "Set a limit on how many times the same tool can be called consecutively with identical parameters. Helps prevent infinite loops."
        )]
        max_tool_repetitions: Option<u32>,

        /// Maximum number of turns (iterations) allowed in a single response
        #[arg(
            long = "max-turns",
            value_name = "NUMBER",
            help = "Maximum number of turns allowed without user input (default: 1000)",
            long_help = "Set a limit on how many turns (iterations) the agent can take without asking for user input to continue."
        )]
        max_turns: Option<u32>,

        /// Add stdio extensions with environment variables and commands
        #[arg(
            long = "with-extension",
            value_name = "COMMAND",
            help = "Add stdio extensions (can be specified multiple times)",
            long_help = "Add stdio extensions from full commands with environment variables. Can be specified multiple times. Format: 'ENV1=val1 ENV2=val2 command args...'",
            action = clap::ArgAction::Append
        )]
        extensions: Vec<String>,

        /// Add remote extensions with a URL
        #[arg(
            long = "with-remote-extension",
            value_name = "URL",
            help = "Add remote extensions (can be specified multiple times)",
            long_help = "Add remote extensions from a URL. Can be specified multiple times. Format: 'url...'",
            action = clap::ArgAction::Append
        )]
        remote_extensions: Vec<String>,

        /// Add streamable HTTP extensions with a URL
        #[arg(
            long = "with-streamable-http-extension",
            value_name = "URL",
            help = "Add streamable HTTP extensions (can be specified multiple times)",
            long_help = "Add streamable HTTP extensions from a URL. Can be specified multiple times. Format: 'url...'",
            action = clap::ArgAction::Append
        )]
        streamable_http_extensions: Vec<String>,

        /// Add builtin extensions by name
        #[arg(
            long = "with-builtin",
            value_name = "NAME",
            help = "Add builtin extensions by name (e.g., 'developer' or multiple: 'developer,github')",
            long_help = "Add one or more builtin extensions that are bundled with goose by specifying their names, comma-separated",
            value_delimiter = ','
        )]
        builtins: Vec<String>,
    },

    /// Open the last project directory
    #[command(about = "Open the last project directory", visible_alias = "p")]
    Project {},

    /// List recent project directories
    #[command(about = "List recent project directories", visible_alias = "ps")]
    Projects,

    /// Execute commands from an instruction file
    #[command(about = "Execute commands from an instruction file or stdin")]
    Run {
        /// Path to instruction file containing commands
        #[arg(
            short,
            long,
            value_name = "FILE",
            help = "Path to instruction file containing commands. Use - for stdin.",
            conflicts_with = "input_text",
            conflicts_with = "recipe"
        )]
        instructions: Option<String>,

        /// Input text containing commands
        #[arg(
            short = 't',
            long = "text",
            value_name = "TEXT",
            help = "Input text to provide to Goose directly",
            long_help = "Input text containing commands for Goose. Use this in lieu of the instructions argument.",
            conflicts_with = "instructions",
            conflicts_with = "recipe"
        )]
        input_text: Option<String>,

        /// Additional system prompt to customize agent behavior
        #[arg(
            long = "system",
            value_name = "TEXT",
            help = "Additional system prompt to customize agent behavior",
            long_help = "Provide additional system instructions to customize the agent's behavior",
            conflicts_with = "recipe"
        )]
        system: Option<String>,

        /// Recipe name or full path to the recipe file
        #[arg(
            short = None,
            long = "recipe",
            value_name = "RECIPE_NAME or FULL_PATH_TO_RECIPE_FILE",
            help = "Recipe name to get recipe file or the full path of the recipe file (use --explain to see recipe details)",
            long_help = "Recipe name to get recipe file or the full path of the recipe file that defines a custom agent configuration. Use --explain to see the recipe's title, description, and parameters.",
            conflicts_with = "instructions",
            conflicts_with = "input_text"
        )]
        recipe: Option<String>,

        #[arg(
            long,
            value_name = "KEY=VALUE",
            help = "Dynamic parameters (e.g., --params username=alice --params channel_name=goose-channel)",
            long_help = "Key-value parameters to pass to the recipe file. Can be specified multiple times.",
            action = clap::ArgAction::Append,
            value_parser = parse_key_val,
        )]
        params: Vec<(String, String)>,

        /// Continue in interactive mode after processing input
        #[arg(
            short = 's',
            long = "interactive",
            help = "Continue in interactive mode after processing initial input"
        )]
        interactive: bool,

        /// Run without storing a session file
        #[arg(
            long = "no-session",
            help = "Run without storing a session file",
            long_help = "Execute commands without creating or using a session file. Useful for automated runs.",
            conflicts_with_all = ["resume", "name", "path"]
        )]
        no_session: bool,

        /// Show the recipe title, description, and parameters
        #[arg(
            long = "explain",
            help = "Show the recipe title, description, and parameters"
        )]
        explain: bool,

        /// Print the rendered recipe instead of running it
        #[arg(
            long = "render-recipe",
            help = "Print the rendered recipe instead of running it."
        )]
        render_recipe: bool,

        /// Maximum number of consecutive identical tool calls allowed
        #[arg(
            long = "max-tool-repetitions",
            value_name = "NUMBER",
            help = "Maximum number of consecutive identical tool calls allowed",
            long_help = "Set a limit on how many times the same tool can be called consecutively with identical parameters. Helps prevent infinite loops."
        )]
        max_tool_repetitions: Option<u32>,

        /// Maximum number of turns (iterations) allowed in a single response
        #[arg(
            long = "max-turns",
            value_name = "NUMBER",
            help = "Maximum number of turns allowed without user input (default: 1000)",
            long_help = "Set a limit on how many turns (iterations) the agent can take without asking for user input to continue."
        )]
        max_turns: Option<u32>,

        /// Identifier for this run session
        #[command(flatten)]
        identifier: Option<Identifier>,

        /// Resume a previous run
        #[arg(
            short,
            long,
            action = clap::ArgAction::SetTrue,
            help = "Resume from a previous run",
            long_help = "Continue from a previous run, maintaining the execution state and context."
        )]
        resume: bool,

        /// Enable debug output mode
        #[arg(
            long,
            help = "Enable debug output mode with full content and no truncation",
            long_help = "When enabled, shows complete tool responses without truncation and full paths."
        )]
        debug: bool,

        /// Add stdio extensions with environment variables and commands
        #[arg(
            long = "with-extension",
            value_name = "COMMAND",
            help = "Add stdio extensions (can be specified multiple times)",
            long_help = "Add stdio extensions from full commands with environment variables. Can be specified multiple times. Format: 'ENV1=val1 ENV2=val2 command args...'",
            action = clap::ArgAction::Append
        )]
        extensions: Vec<String>,

        /// Add remote extensions
        #[arg(
            long = "with-remote-extension",
            value_name = "URL",
            help = "Add remote extensions (can be specified multiple times)",
            long_help = "Add remote extensions. Can be specified multiple times. Format: 'url...'",
            action = clap::ArgAction::Append
        )]
        remote_extensions: Vec<String>,

        /// Add streamable HTTP extensions
        #[arg(
            long = "with-streamable-http-extension",
            value_name = "URL",
            help = "Add streamable HTTP extensions (can be specified multiple times)",
            long_help = "Add streamable HTTP extensions. Can be specified multiple times. Format: 'url...'",
            action = clap::ArgAction::Append
        )]
        streamable_http_extensions: Vec<String>,

        /// Add builtin extensions by name
        #[arg(
            long = "with-builtin",
            value_name = "NAME",
            help = "Add builtin extensions by name (e.g., 'developer' or multiple: 'developer,github')",
            long_help = "Add one or more builtin extensions that are bundled with goose by specifying their names, comma-separated",
            value_delimiter = ','
        )]
        builtins: Vec<String>,

        /// Quiet mode - suppress non-response output
        #[arg(
            short = 'q',
            long = "quiet",
            help = "Quiet mode. Suppress non-response output, printing only the model response to stdout"
        )]
        quiet: bool,

        /// Scheduled job ID (used internally for scheduled executions)
        #[arg(
            long = "scheduled-job-id",
            value_name = "ID",
            help = "ID of the scheduled job that triggered this execution (internal use)",
            long_help = "Internal parameter used when this run command is executed by a scheduled job. This associates the session with the schedule for tracking purposes.",
            hide = true
        )]
        scheduled_job_id: Option<String>,

        /// Additional sub-recipe file paths
        #[arg(
            long = "sub-recipe",
            value_name = "RECIPE",
            help = "Sub-recipe name or file path (can be specified multiple times)",
            long_help = "Specify sub-recipes to include alongside the main recipe. Can be:\n  - Recipe names from GitHub (if GOOSE_RECIPE_GITHUB_REPO is configured)\n  - Local file paths to YAML files\nCan be specified multiple times to include multiple sub-recipes.",
            action = clap::ArgAction::Append
        )]
        additional_sub_recipes: Vec<String>,

        /// Provider to use for this run (overrides environment variable)
        #[arg(
            long = "provider",
            value_name = "PROVIDER",
            help = "Specify the LLM provider to use (e.g., 'openai', 'anthropic')",
            long_help = "Override the GOOSE_PROVIDER environment variable for this run. Available providers include openai, anthropic, ollama, databricks, gemini-cli, claude-code, and others."
        )]
        provider: Option<String>,

        /// Model to use for this run (overrides environment variable)
        #[arg(
            long = "model",
            value_name = "MODEL",
            help = "Specify the model to use (e.g., 'gpt-4o', 'claude-3.5-sonnet')",
            long_help = "Override the GOOSE_MODEL environment variable for this run. The model must be supported by the specified provider."
        )]
        model: Option<String>,
    },

    /// Recipe utilities for validation and deeplinking
    #[command(about = "Recipe utilities for validation and deeplinking")]
    Recipe {
        #[command(subcommand)]
        command: RecipeCommand,
    },

    /// Manage scheduled jobs
    #[command(about = "Manage scheduled jobs", visible_alias = "sched")]
    Schedule {
        #[command(subcommand)]
        command: SchedulerCommand,
    },

    /// Update the Goose CLI version
    #[command(about = "Update the goose CLI version")]
    Update {
        /// Update to canary version
        #[arg(
            short,
            long,
            help = "Update to canary version",
            long_help = "Update to the latest canary version of the goose CLI, otherwise updates to the latest stable version."
        )]
        canary: bool,

        /// Enforce to re-configure Goose during update
        #[arg(short, long, help = "Enforce to re-configure goose during update")]
        reconfigure: bool,
    },

    /// Evaluate system configuration across a range of practical tasks
    #[command(about = "Evaluate system configuration across a range of practical tasks")]
    Bench {
        #[command(subcommand)]
        cmd: BenchCommand,
    },

    /// Start a web server with a chat interface
    #[command(about = "Experimental: Start a web server with a chat interface")]
    Web {
        /// Port to run the web server on
        #[arg(
            short,
            long,
            default_value = "3000",
            help = "Port to run the web server on"
        )]
        port: u16,

        /// Host to bind the web server to
        #[arg(
            long,
            default_value = "127.0.0.1",
            help = "Host to bind the web server to"
        )]
        host: String,

        /// Open browser automatically
        #[arg(long, help = "Open browser automatically when server starts")]
        open: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum CliProviderVariant {
    OpenAi,
    Databricks,
    Ollama,
}

#[derive(Debug)]
pub struct InputConfig {
    pub contents: Option<String>,
    pub extensions_override: Option<Vec<ExtensionConfig>>,
    pub additional_system_prompt: Option<String>,
}

#[derive(Debug)]
pub struct RecipeInfo {
    pub session_settings: Option<SessionSettings>,
    pub sub_recipes: Option<Vec<goose::recipe::SubRecipe>>,
    pub final_output_response: Option<goose::recipe::Response>,
    pub retry_config: Option<goose::agents::types::RetryConfig>,
}

pub async fn cli() -> Result<()> {
    let cli = Cli::parse();

    // Track the current directory in projects.json
    if let Err(e) = crate::project_tracker::update_project_tracker(None, None) {
        eprintln!("Warning: Failed to update project tracker: {}", e);
    }

    let command_name = match &cli.command {
        Some(Command::Configure {}) => "configure",
        Some(Command::Info { .. }) => "info",
        Some(Command::Mcp { .. }) => "mcp",
        Some(Command::Session { .. }) => "session",
        Some(Command::Project {}) => "project",
        Some(Command::Projects) => "projects",
        Some(Command::Run { .. }) => "run",
        Some(Command::Schedule { .. }) => "schedule",
        Some(Command::Update { .. }) => "update",
        Some(Command::Bench { .. }) => "bench",
        Some(Command::Recipe { .. }) => "recipe",
        Some(Command::Web { .. }) => "web",
        None => "default_session",
    };

    tracing::info!(
        counter.goose.cli_commands = 1,
        command = command_name,
        "CLI command executed"
    );

    match cli.command {
        Some(Command::Configure {}) => {
            let _ = handle_configure().await;
            return Ok(());
        }
        Some(Command::Info { verbose }) => {
            handle_info(verbose)?;
            return Ok(());
        }
        Some(Command::Mcp { name }) => {
            let _ = run_server(&name).await;
        }
        Some(Command::Session {
            command,
            identifier,
            resume,
            history,
            debug,
            max_tool_repetitions,
            max_turns,
            extensions,
            remote_extensions,
            streamable_http_extensions,
            builtins,
        }) => {
            return match command {
                Some(SessionCommand::List {
                    verbose,
                    format,
                    ascending,
                }) => {
                    handle_session_list(verbose, format, ascending)?;
                    Ok(())
                }
                Some(SessionCommand::Remove { id, regex }) => {
                    handle_session_remove(id, regex)?;
                    return Ok(());
                }
                Some(SessionCommand::Export { identifier, output }) => {
                    let session_identifier = if let Some(id) = identifier {
                        extract_identifier(id)
                    } else {
                        // If no identifier is provided, prompt for interactive selection
                        match crate::commands::session::prompt_interactive_session_selection() {
                            Ok(id) => id,
                            Err(e) => {
                                eprintln!("Error: {}", e);
                                return Ok(());
                            }
                        }
                    };

                    crate::commands::session::handle_session_export(session_identifier, output)?;
                    Ok(())
                }
                None => {
                    let session_start = std::time::Instant::now();
                    let session_type = if resume { "resumed" } else { "new" };

                    tracing::info!(
                        counter.goose.session_starts = 1,
                        session_type,
                        interactive = true,
                        "Session started"
                    );

                    // Run session command by default
                    let mut session: crate::Session = build_session(SessionBuilderConfig {
                        identifier: identifier.map(extract_identifier),
                        resume,
                        no_session: false,
                        extensions,
                        remote_extensions,
                        streamable_http_extensions,
                        builtins,
                        extensions_override: None,
                        additional_system_prompt: None,
                        settings: None,
                        provider: None,
                        model: None,
                        debug,
                        max_tool_repetitions,
                        max_turns,
                        scheduled_job_id: None,
                        interactive: true,
                        quiet: false,
                        sub_recipes: None,
                        final_output_response: None,
                        retry_config: None,
                    })
                    .await;

                    // Render previous messages if resuming a session and history flag is set
                    if resume && history {
                        session.render_message_history();
                    }

                    let result = session.interactive(None).await;

                    let session_duration = session_start.elapsed();
                    let exit_type = if result.is_ok() { "normal" } else { "error" };

                    let (total_tokens, message_count) = session
                        .get_metadata()
                        .map(|m| (m.total_tokens.unwrap_or(0), m.message_count))
                        .unwrap_or((0, 0));

                    tracing::info!(
                        counter.goose.session_completions = 1,
                        session_type,
                        exit_type,
                        duration_ms = session_duration.as_millis() as u64,
                        total_tokens,
                        message_count,
                        "Session completed"
                    );

                    tracing::info!(
                        counter.goose.session_duration_ms = session_duration.as_millis() as u64,
                        session_type,
                        "Session duration"
                    );

                    if total_tokens > 0 {
                        tracing::info!(
                            counter.goose.session_tokens = total_tokens,
                            session_type,
                            "Session tokens"
                        );
                    }

                    Ok(())
                }
            };
        }
        Some(Command::Project {}) => {
            // Default behavior: offer to resume the last project
            handle_project_default()?;
            return Ok(());
        }
        Some(Command::Projects) => {
            // Interactive project selection
            handle_projects_interactive()?;
            return Ok(());
        }

        Some(Command::Run {
            instructions,
            input_text,
            recipe,
            system,
            interactive,
            identifier,
            resume,
            no_session,
            debug,
            max_tool_repetitions,
            max_turns,
            extensions,
            remote_extensions,
            streamable_http_extensions,
            builtins,
            params,
            explain,
            render_recipe,
            scheduled_job_id,
            quiet,
            additional_sub_recipes,
            provider,
            model,
        }) => {
            let (input_config, recipe_info) = match (instructions, input_text, recipe) {
                (Some(file), _, _) if file == "-" => {
                    let mut input = String::new();
                    std::io::stdin()
                        .read_to_string(&mut input)
                        .expect("Failed to read from stdin");

                    let input_config = InputConfig {
                        contents: Some(input),
                        extensions_override: None,
                        additional_system_prompt: system,
                    };
                    (input_config, None)
                }
                (Some(file), _, _) => {
                    let contents = std::fs::read_to_string(&file).unwrap_or_else(|err| {
                        eprintln!(
                            "Instruction file not found — did you mean to use goose run --text?\n{}",
                            err
                        );
                        std::process::exit(1);
                    });
                    let input_config = InputConfig {
                        contents: Some(contents),
                        extensions_override: None,
                        additional_system_prompt: None,
                    };
                    (input_config, None)
                }
                (_, Some(text), _) => {
                    let input_config = InputConfig {
                        contents: Some(text),
                        extensions_override: None,
                        additional_system_prompt: system,
                    };
                    (input_config, None)
                }
                (_, _, Some(recipe_name)) => {
                    let recipe_display_name = std::path::Path::new(&recipe_name)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(&recipe_name);

                    tracing::info!(counter.goose.recipe_runs = 1,
                        recipe_name = %recipe_display_name,
                        "Recipe execution started"
                    );

                    if explain {
                        explain_recipe(&recipe_name, params)?;
                        return Ok(());
                    }
                    if render_recipe {
                        if let Err(err) = render_recipe_as_yaml(&recipe_name, params) {
                            eprintln!("{}: {}", console::style("Error").red().bold(), err);
                            std::process::exit(1);
                        }
                        return Ok(());
                    }
                    let (input_config, recipe_info) =
                        extract_recipe_info_from_cli(recipe_name, params, additional_sub_recipes)?;
                    (input_config, Some(recipe_info))
                }
                (None, None, None) => {
                    eprintln!("Error: Must provide either --instructions (-i), --text (-t), or --recipe. Use -i - for stdin.");
                    std::process::exit(1);
                }
            };

            let mut session = build_session(SessionBuilderConfig {
                identifier: identifier.map(extract_identifier),
                resume,
                no_session,
                extensions,
                remote_extensions,
                streamable_http_extensions,
                builtins,
                extensions_override: input_config.extensions_override,
                additional_system_prompt: input_config.additional_system_prompt,
                settings: recipe_info
                    .as_ref()
                    .and_then(|r| r.session_settings.clone()),
                provider,
                model,
                debug,
                max_tool_repetitions,
                max_turns,
                scheduled_job_id,
                interactive, // Use the interactive flag from the Run command
                quiet,
                sub_recipes: recipe_info.as_ref().and_then(|r| r.sub_recipes.clone()),
                final_output_response: recipe_info
                    .as_ref()
                    .and_then(|r| r.final_output_response.clone()),
                retry_config: recipe_info.as_ref().and_then(|r| r.retry_config.clone()),
            })
            .await;

            if interactive {
                let _ = session.interactive(input_config.contents).await;
            } else if let Some(contents) = input_config.contents {
                let session_start = std::time::Instant::now();
                let session_type = if recipe_info.is_some() {
                    "recipe"
                } else {
                    "run"
                };

                tracing::info!(
                    counter.goose.session_starts = 1,
                    session_type,
                    interactive = false,
                    "Headless session started"
                );

                let result = session.headless(contents).await;

                let session_duration = session_start.elapsed();
                let exit_type = if result.is_ok() { "normal" } else { "error" };

                let (total_tokens, message_count) = session
                    .get_metadata()
                    .map(|m| (m.total_tokens.unwrap_or(0), m.message_count))
                    .unwrap_or((0, 0));

                tracing::info!(
                    counter.goose.session_completions = 1,
                    session_type,
                    exit_type,
                    duration_ms = session_duration.as_millis() as u64,
                    total_tokens,
                    message_count,
                    interactive = false,
                    "Headless session completed"
                );

                tracing::info!(
                    counter.goose.session_duration_ms = session_duration.as_millis() as u64,
                    session_type,
                    "Headless session duration"
                );

                if total_tokens > 0 {
                    tracing::info!(
                        counter.goose.session_tokens = total_tokens,
                        session_type,
                        "Headless session tokens"
                    );
                }

                result?;
            } else {
                eprintln!("Error: no text provided for prompt in headless mode");
                std::process::exit(1);
            }

            return Ok(());
        }
        Some(Command::Schedule { command }) => {
            match command {
                SchedulerCommand::Add {
                    id,
                    cron,
                    recipe_source,
                } => {
                    handle_schedule_add(id, cron, recipe_source).await?;
                }
                SchedulerCommand::List {} => {
                    handle_schedule_list().await?;
                }
                SchedulerCommand::Remove { id } => {
                    handle_schedule_remove(id).await?;
                }
                SchedulerCommand::Sessions { id, limit } => {
                    // New arm
                    handle_schedule_sessions(id, limit).await?;
                }
                SchedulerCommand::RunNow { id } => {
                    // New arm
                    handle_schedule_run_now(id).await?;
                }
                SchedulerCommand::ServicesStatus {} => {
                    handle_schedule_services_status().await?;
                }
                SchedulerCommand::ServicesStop {} => {
                    handle_schedule_services_stop().await?;
                }
                SchedulerCommand::CronHelp {} => {
                    handle_schedule_cron_help().await?;
                }
            }
            return Ok(());
        }
        Some(Command::Update {
            canary,
            reconfigure,
        }) => {
            crate::commands::update::update(canary, reconfigure)?;
            return Ok(());
        }
        Some(Command::Bench { cmd }) => {
            match cmd {
                BenchCommand::Selectors { config } => BenchRunner::list_selectors(config)?,
                BenchCommand::InitConfig { name } => {
                    let mut config = BenchRunConfig::default();
                    let cwd =
                        std::env::current_dir().expect("Failed to get current working directory");
                    config.output_dir = Some(cwd);
                    config.save(name);
                }
                BenchCommand::Run { config } => BenchRunner::new(config)?.run()?,
                BenchCommand::EvalModel { config } => ModelRunner::from(config)?.run()?,
                BenchCommand::ExecEval { config } => {
                    EvalRunner::from(config)?.run(agent_generator).await?
                }
                BenchCommand::GenerateLeaderboard { benchmark_dir } => {
                    MetricAggregator::generate_csv_from_benchmark_dir(&benchmark_dir)?
                }
            }
            return Ok(());
        }
        Some(Command::Recipe { command }) => {
            match command {
                RecipeCommand::Validate { recipe_name } => {
                    handle_validate(&recipe_name)?;
                }
                RecipeCommand::Deeplink { recipe_name } => {
                    handle_deeplink(&recipe_name)?;
                }
                RecipeCommand::List { format, verbose } => {
                    handle_list(&format, verbose)?;
                }
            }
            return Ok(());
        }
        Some(Command::Web { port, host, open }) => {
            crate::commands::web::handle_web(port, host, open).await?;
            return Ok(());
        }
        None => {
            return if !Config::global().exists() {
                let _ = handle_configure().await;
                Ok(())
            } else {
                // Run session command by default
                let mut session = build_session(SessionBuilderConfig {
                    identifier: None,
                    resume: false,
                    no_session: false,
                    extensions: Vec::new(),
                    remote_extensions: Vec::new(),
                    streamable_http_extensions: Vec::new(),
                    builtins: Vec::new(),
                    extensions_override: None,
                    additional_system_prompt: None,
                    settings: None::<SessionSettings>,
                    provider: None,
                    model: None,
                    debug: false,
                    max_tool_repetitions: None,
                    max_turns: None,
                    scheduled_job_id: None,
                    interactive: true, // Default case is always interactive
                    quiet: false,
                    sub_recipes: None,
                    final_output_response: None,
                    retry_config: None,
                })
                .await;
                if let Err(e) = session.interactive(None).await {
                    eprintln!("Session ended with error: {}", e);
                }
                Ok(())
            };
        }
    }
    Ok(())
}
