use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use goose::agents::AgentFactory;

mod commands;
mod config;
mod log_usage;
mod logging;
mod prompt;
mod session;

use commands::agent_version::AgentCommand;
use commands::configure::handle_configure;
use commands::mcp::run_server;
use commands::session::build_session;
use commands::version::print_version;
use config::Config;
use console::style;
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
    Configure {
        /// AI Provider to use
        #[arg(
            short,
            long,
            help = "AI Provider to use (e.g., 'openai', 'databricks', 'ollama')",
            long_help = "Specify AI Provider to use (e.g., 'openai', 'databricks', 'ollama')."
        )]
        provider: Option<String>,

        /// Model to use
        #[arg(
            short,
            long,
            help = "Model to use (e.g., 'gpt-4', 'llama2')",
            long_help = "Specify which model to use."
        )]
        model: Option<String>,
    },

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

        /// Provider to use (overrides config)
        #[arg(
            short,
            long,
            help = "Provider to use (e.g., 'openai', 'anthropic')",
            long_help = "Override the default provider from config"
        )]
        provider: Option<String>,

        /// Model to use (overrides config)
        #[arg(
            short,
            long,
            help = "Model to use (e.g., 'gpt-4', 'claude-3')",
            long_help = "Override the default model from config"
        )]
        model: Option<String>,

        /// Agent version to use (e.g., 'default', 'v1')
        #[arg(
            short,
            long,
            help = "Agent version to use (e.g., 'default', 'v1'), defaults to 'default'",
            long_help = "Specify which agent version to use for this session."
        )]
        agent: Option<String>,

        /// Resume a previous session
        #[arg(
            short,
            long,
            help = "Resume a previous session (last used or specified by --session)",
            long_help = "Continue from a previous chat session. If --session is provided, resumes that specific session. Otherwise resumes the last used session."
        )]
        resume: bool,
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

        /// Provider to use (overrides config)
        #[arg(
            short,
            long,
            help = "Provider to use (e.g., 'openai', 'anthropic')",
            long_help = "Override the default provider from config"
        )]
        provider: Option<String>,

        /// Model to use (overrides config)
        #[arg(
            short,
            long,
            help = "Model to use (e.g., 'gpt-4', 'claude-3')",
            long_help = "Override the default model from config"
        )]
        model: Option<String>,

        /// Name for this run session
        #[arg(
            short,
            long,
            value_name = "NAME",
            help = "Name for this run session (e.g., 'daily-tasks')",
            long_help = "Specify a name for this run session. This helps identify and resume specific runs later."
        )]
        name: Option<String>,

        /// Agent version to use (e.g., 'default', 'v1')
        #[arg(
            short,
            long,
            help = "Agent version to use (e.g., 'default', 'v1')",
            long_help = "Specify which agent version to use for this session."
        )]
        agent: Option<String>,

        /// Resume a previous run
        #[arg(
            short,
            long,
            action = clap::ArgAction::SetTrue,
            help = "Resume from a previous run",
            long_help = "Continue from a previous run, maintaining the execution state and context."
        )]
        resume: bool,
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
        Some(Command::Configure { provider, model }) => {
            let _ = handle_configure(provider, model).await;
            return Ok(());
        }
        Some(Command::Mcp { name }) => {
            let _ = run_server(&name).await;
        }
        Some(Command::Session {
            name,
            provider,
            model,
            agent,
            resume,
        }) => {
            if let Some(agent_version) = agent.clone() {
                if !AgentFactory::available_versions().contains(&agent_version.as_str()) {
                    eprintln!("Error: Invalid agent version '{}'", agent_version);
                    eprintln!("Available versions:");
                    for version in AgentFactory::available_versions() {
                        if version == AgentFactory::default_version() {
                            eprintln!("* {} (default)", version);
                        } else {
                            eprintln!("  {}", version);
                        }
                    }
                    std::process::exit(1);
                }
            }

            let mut session = build_session(name, provider, model, agent, resume).await;
            setup_logging(session.session_file().file_stem().and_then(|s| s.to_str()))?;

            let _ = session.start().await;
            return Ok(());
        }
        Some(Command::Run {
            instructions,
            input_text,
            provider,
            model,
            name,
            agent,
            resume,
        }) => {
            // Validate that we have some input source
            if instructions.is_none() && input_text.is_none() {
                eprintln!("Error: Must provide either --instructions or --text");
                std::process::exit(1);
            }

            if let Some(agent_version) = agent.clone() {
                if !AgentFactory::available_versions().contains(&agent_version.as_str()) {
                    eprintln!("Error: Invalid agent version '{}'", agent_version);
                    eprintln!("Available versions:");
                    for version in AgentFactory::available_versions() {
                        if version == AgentFactory::default_version() {
                            eprintln!("* {} (default)", version);
                        } else {
                            eprintln!("  {}", version);
                        }
                    }
                    std::process::exit(1);
                }
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
            let mut session = build_session(name, provider, model, agent, resume).await;
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
            if Config::load().is_err() {
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
