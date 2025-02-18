use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

use console::style;
use goose::config::Config;
use goose_cli::commands::agent_version::AgentCommand;
use goose_cli::commands::configure::handle_configure;
use goose_cli::commands::info::handle_info;
use goose_cli::commands::mcp::run_server;
use goose_cli::logging::setup_logging;
use goose_cli::session::build_session;
use std::io::{self, Read};

#[derive(Parser)]
#[command(author, version, display_name = "", about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
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
        /// Name for the chat session
        #[arg(
            short,
            long,
            value_name = "NAME",
            help = "Name for the chat session (e.g., 'project-x')",
            long_help = "Specify a name for your chat session. When used with --resume, will resume this specific session if it exists."
        )]
        name: Option<String>,

        /// Resume a previous session
        #[arg(
            short,
            long,
            help = "Resume a previous session (last used or specified by --name)",
            long_help = "Continue from a previous chat session. If --name is provided, resumes that specific session. Otherwise resumes the last used session."
        )]
        resume: bool,

        /// Add stdio extensions with environment variables and commands
        #[arg(
            long = "with-extension",
            value_name = "COMMAND",
            help = "Add stdio extensions (can be specified multiple times)",
            long_help = "Add stdio extensions from full commands with environment variables. Can be specified multiple times. Format: 'ENV1=val1 ENV2=val2 command args...'",
            action = clap::ArgAction::Append
        )]
        extension: Vec<String>,

        /// Add builtin extensions by name
        #[arg(
            long = "with-builtin",
            value_name = "NAME",
            help = "Add builtin extensions by name (e.g., 'developer' or multiple: 'developer,github')",
            long_help = "Add one or more builtin extensions that are bundled with goose by specifying their names, comma-separated",
            value_delimiter = ','
        )]
        builtin: Vec<String>,
    },

    /// Execute commands from an instruction file
    #[command(about = "Execute commands from an instruction file or stdin")]
    Run {
        /// Path to instruction file containing commands
        #[arg(
            short,
            long,
            value_name = "FILE",
            help = "Path to instruction file containing commands",
            conflicts_with = "input_text"
        )]
        instructions: Option<String>,

        /// Input text containing commands
        #[arg(
            short = 't',
            long = "text",
            value_name = "TEXT",
            help = "Input text to provide to Goose directly",
            long_help = "Input text containing commands for Goose. Use this in lieu of the instructions argument.",
            conflicts_with = "instructions"
        )]
        input_text: Option<String>,

        /// Name for this run session
        #[arg(
            short,
            long,
            value_name = "NAME",
            help = "Name for this run session (e.g., 'daily-tasks')",
            long_help = "Specify a name for this run session. This helps identify and resume specific runs later."
        )]
        name: Option<String>,

        /// Resume a previous run
        #[arg(
            short,
            long,
            action = clap::ArgAction::SetTrue,
            help = "Resume from a previous run",
            long_help = "Continue from a previous run, maintaining the execution state and context."
        )]
        resume: bool,

        /// Add stdio extensions with environment variables and commands
        #[arg(
            long = "with-extension",
            value_name = "COMMAND",
            help = "Add stdio extensions (can be specified multiple times)",
            long_help = "Add stdio extensions from full commands with environment variables. Can be specified multiple times. Format: 'ENV1=val1 ENV2=val2 command args...'",
            action = clap::ArgAction::Append
        )]
        extension: Vec<String>,

        /// Add builtin extensions by name
        #[arg(
            long = "with-builtin",
            value_name = "NAME",
            help = "Add builtin extensions by name (e.g., 'developer' or multiple: 'developer,github')",
            long_help = "Add one or more builtin extensions that are bundled with goose by specifying their names, comma-separated",
            value_delimiter = ','
        )]
        builtin: Vec<String>,
    },

    /// List available agent versions
    Agents(AgentCommand),
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum CliProviderVariant {
    OpenAi,
    Databricks,
    Ollama,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

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
            name,
            resume,
            extension,
            builtin,
        }) => {
            let mut session = build_session(name, resume, extension, builtin).await;
            setup_logging(session.session_file().file_stem().and_then(|s| s.to_str()))?;
            let _ = session.start().await;
            return Ok(());
        }
        Some(Command::Run {
            instructions,
            input_text,
            name,
            resume,
            extension,
            builtin,
        }) => {
            // Validate that we have some input source
            if instructions.is_none() && input_text.is_none() {
                eprintln!("Error: Must provide either --instructions or --text");
                std::process::exit(1);
            }

            let contents = if let Some(file_name) = instructions {
                let file_path = std::path::Path::new(&file_name);
                std::fs::read_to_string(file_path).expect("Failed to read the instruction file")
            } else if let Some(input_text) = input_text {
                input_text
            } else {
                let mut stdin = String::new();
                io::stdin()
                    .read_to_string(&mut stdin)
                    .expect("Failed to read from stdin");
                stdin
            };
            let mut session = build_session(name, resume, extension, builtin).await;
            setup_logging(session.session_file().file_stem().and_then(|s| s.to_str()))?;
            let _ = session.headless_start(contents.clone()).await;
            return Ok(());
        }
        Some(Command::Agents(cmd)) => {
            cmd.run()?;
            return Ok(());
        }
        None => {
            Cli::command().print_help()?;
            println!();
            if !Config::global().exists() {
                println!(
                    "\n  {}: Run '{}' to setup goose for the first time",
                    style("Tip").green().italic(),
                    style("goose configure").cyan()
                );
            }
        }
    }
    Ok(())
}
