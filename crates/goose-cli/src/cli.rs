use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use goose::config::Config;

use crate::commands::agent_version::AgentCommand;
use crate::commands::bench::{list_selectors, run_benchmark};
use crate::commands::configure::handle_configure;
use crate::commands::info::handle_info;
use crate::commands::mcp::run_server;
use crate::commands::session::handle_session_list;
use crate::logging::setup_logging;
use crate::session;
use crate::session::build_session;
use std::io::Read;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, display_name = "", about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Args)]
#[group(required = false, multiple = false)]
struct Identifier {
    #[arg(
        short,
        long,
        value_name = "NAME",
        help = "Name for the chat session (e.g., 'project-x')",
        long_help = "Specify a name for your chat session. When used with --resume, will resume this specific session if it exists."
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
        extension: Vec<String>,

        /// Add remote extensions with a URL
        #[arg(
            long = "with-remote-extension",
            value_name = "URL",
            help = "Add remote extensions (can be specified multiple times)",
            long_help = "Add remote extensions from a URL. Can be specified multiple times. Format: 'url...'",
            action = clap::ArgAction::Append
        )]
        remote_extension: Vec<String>,

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
            help = "Path to instruction file containing commands. Use - for stdin.",
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

        /// Continue in interactive mode after processing input
        #[arg(
            short = 's',
            long = "interactive",
            help = "Continue in interactive mode after processing initial input"
        )]
        interactive: bool,

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
        extension: Vec<String>,

        /// Add remote extensions
        #[arg(
            long = "with-remote-extension",
            value_name = "URL",
            help = "Add remote extensions (can be specified multiple times)",
            long_help = "Add remote extensions. Can be specified multiple times. Format: 'url...'",
            action = clap::ArgAction::Append
        )]
        remote_extension: Vec<String>,

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

    Bench {
        #[arg(
            short = 's',
            long = "selectors",
            value_name = "EVALUATIONS_SELECTOR",
            help = "Run this list of bench-suites.",
            long_help = "Specify a comma-separated list of evaluation-suite names to be run.",
            value_delimiter = ','
        )]
        selectors: Vec<String>,

        #[arg(
            short = 'i',
            long = "include-dir",
            value_name = "DIR_NAME",
            action = clap::ArgAction::Append,
            long_help = "Make one or more dirs available to all bench suites. Specify either a single dir-name, a comma-separated list of dir-names, or use this multiple instances of this flag to specify multiple dirs.",
            value_delimiter = ','
        )]
        include_dirs: Vec<PathBuf>,

        #[arg(
            long = "repeat",
            value_name = "QUANTITY",
            long_help = "Number of times to repeat the benchmark run.",
            default_value = "1"
        )]
        repeat: usize,

        #[arg(
            long = "list",
            value_name = "LIST",
            help = "List all selectors and the number of evaluations they select."
        )]
        list: bool,

        #[arg(
            long = "output",
            short = 'o',
            value_name = "FILE",
            help = "Save benchmark results to a file"
        )]
        output: Option<PathBuf>,

        #[arg(
            long = "format",
            value_name = "FORMAT",
            help = "Output format (text, json)",
            default_value = "text"
        )]
        format: String,

        #[arg(
            long = "summary",
            help = "Show only summary results",
            action = clap::ArgAction::SetTrue
        )]
        summary: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum CliProviderVariant {
    OpenAi,
    Databricks,
    Ollama,
}

pub async fn cli() -> Result<()> {
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
            command,
            identifier,
            resume,
            debug,
            extension,
            remote_extension,
            builtin,
        }) => {
            match command {
                Some(SessionCommand::List { verbose, format }) => {
                    handle_session_list(verbose, format)?;
                    return Ok(());
                }
                None => {
                    // Run session command by default
                    let mut session = build_session(
                        identifier.map(extract_identifier),
                        resume,
                        extension,
                        remote_extension,
                        builtin,
                        debug,
                    )
                    .await;
                    setup_logging(
                        session.session_file().file_stem().and_then(|s| s.to_str()),
                        None,
                    )?;
                    let _ = session.interactive(None).await;
                    return Ok(());
                }
            }
        }
        Some(Command::Run {
            instructions,
            input_text,
            interactive,
            identifier,
            resume,
            debug,
            extension,
            remote_extension,
            builtin,
        }) => {
            let contents = match (instructions, input_text) {
                (Some(file), _) if file == "-" => {
                    let mut stdin = String::new();
                    std::io::stdin()
                        .read_to_string(&mut stdin)
                        .expect("Failed to read from stdin");
                    stdin
                }
                (Some(file), _) => std::fs::read_to_string(&file).unwrap_or_else(|err| {
                    eprintln!(
                        "Instruction file not found — did you mean to use goose run --text?\n{}",
                        err
                    );
                    std::process::exit(1);
                }),
                (None, Some(text)) => text,
                (None, None) => {
                    eprintln!("Error: Must provide either --instructions (-i) or --text (-t). Use -i - for stdin.");
                    std::process::exit(1);
                }
            };

            let mut session = build_session(
                identifier.map(extract_identifier),
                resume,
                extension,
                remote_extension,
                builtin,
                debug,
            )
            .await;

            setup_logging(
                session.session_file().file_stem().and_then(|s| s.to_str()),
                None,
            )?;

            if interactive {
                session.interactive(Some(contents)).await?;
            } else {
                session.headless(contents).await?;
            }

            return Ok(());
        }
        Some(Command::Agents(cmd)) => {
            cmd.run()?;
            return Ok(());
        }
        Some(Command::Update {
            canary,
            reconfigure,
        }) => {
            crate::commands::update::update(canary, reconfigure)?;
            return Ok(());
        }
        Some(Command::Bench {
            selectors,
            include_dirs,
            repeat,
            list,
            output,
            format,
            summary,
        }) => {
            if list {
                return list_selectors().await;
            }

            let selectors = if selectors.is_empty() {
                vec!["core".to_string()]
            } else {
                selectors
            };

            let current_dir = std::env::current_dir()?;

            for i in 0..repeat {
                if repeat > 1 {
                    println!("\nRun {} of {}:", i + 1, repeat);
                }
                let results = run_benchmark(selectors.clone(), include_dirs.clone()).await?;

                // Handle output based on format
                let output_str = match format.as_str() {
                    "json" => serde_json::to_string_pretty(&results)?,
                    _ => results.to_string(), // Uses Display impl
                };

                // Save to file if specified
                if let Some(path) = &output {
                    std::fs::write(current_dir.join(path), &output_str)?;
                    println!("Results saved to: {}", path.display());
                } else {
                    // Print to console
                    if summary {
                        println!("{}", results.summary());
                    } else {
                        println!("{}", output_str);
                    }
                }
            }
            return Ok(());
        }
        None => {
            if !Config::global().exists() {
                let _ = handle_configure().await;
                return Ok(());
            } else {
                // Run session command by default
                let mut session = build_session(None, false, vec![], vec![], vec![], false).await;
                setup_logging(
                    session.session_file().file_stem().and_then(|s| s.to_str()),
                    None,
                )?;
                let _ = session.interactive(None).await;
                return Ok(());
            }
        }
    }
    Ok(())
}
