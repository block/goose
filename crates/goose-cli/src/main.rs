use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

mod commands;
mod log_usage;
mod logging;
mod prompt;
mod session;

use commands::agent_version::AgentCommand;
use commands::configure::handle_configure;
use commands::mcp::run_server;
use commands::session::build_session;
use commands::version::print_version;
use console::style;
use goose::config::Config;
use logging::setup_logging;
use std::io::{self, Read};

#[cfg(test)]
mod test_helpers;

#[derive(Parser)]
#[command(author, about, long_about = None)]
struct Cli {
    #[arg(short = 'v', long = "version")]
    version: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Configure Goose settings
    #[command(about = "Configure Goose settings")]
    Configure {},

    /// Manage system prompts and behaviors
    #[command(about = "Run one of the mcp servers bundled with goose")]
    Mcp { name: String },

    /// Start or resume interactive chat sessions
    #[command(about = "Start or resume interactive chat sessions", alias = "s")]
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
            help = "Resume a previous session (last used or specified by --session)",
            long_help = "Continue from a previous chat session. If --session is provided, resumes that specific session. Otherwise resumes the last used session."
        )]
        resume: bool,

        /// Add a stdio extension with environment variables and command
        #[arg(
            long = "with-extension",
            value_name = "COMMAND",
            help = "Add a stdio extension (e.g., 'GITHUB_TOKEN=xyz npx -y @modelcontextprotocol/server-github')",
            long_help = "Add a stdio extension from a full command with environment variables. Format: 'ENV1=val1 ENV2=val2 command args...'"
        )]
        extension: Option<String>,

        /// Add a builtin extension by name
        #[arg(
            long = "with-builtin",
            value_name = "NAME",
            help = "Add a builtin extension by name (e.g., 'developer')",
            long_help = "Add a builtin extension that is bundled with goose by specifying its name"
        )]
        builtin: Option<String>,
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

        /// Add a stdio extension with environment variables and command
        #[arg(
            long = "with-extension",
            value_name = "COMMAND",
            help = "Add a stdio extension with environment variables and command (e.g., 'GITHUB_TOKEN=xyz npx -y @modelcontextprotocol/server-github')",
            long_help = "Add a stdio extension with environment variables and command. Format: 'ENV1=val1 ENV2=val2 command args...'"
        )]
        extension: Option<String>,

        /// Add a builtin extension by name
        #[arg(
            long = "with-builtin",
            value_name = "NAME",
            help = "Add a builtin extension by name (e.g., 'developer')",
            long_help = "Add a builtin extension that is compiled into goose by specifying its name"
        )]
        builtin: Option<String>,
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

    if cli.version {
        print_version();
        return Ok(());
    }

    match cli.command {
        Some(Command::Configure {}) => {
            let _ = handle_configure().await;
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
