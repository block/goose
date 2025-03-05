use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use goose::config::Config;

use goose_cli::commands::agent_version::AgentCommand;
use goose_cli::commands::bench::{list_suites, run_benchmark};
use goose_cli::commands::configure::handle_configure;
use goose_cli::commands::info::handle_info;
use goose_cli::commands::mcp::run_server;
use goose_cli::logging::setup_logging;
use goose_cli::session;
use goose_cli::session::build_session;
use std::io::{self, Read};
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
            long = "suites",
            value_name = "BENCH_SUITE_NAME",
            help = "Run this list of bench-suites.",
            long_help = "Specify a comma-separated list of evaluation-suite names to be run.",
            value_delimiter = ','
        )]
        suites: Vec<String>,

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
            help = "List all available bench suites."
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
            identifier,
            resume,
            extension,
            builtin,
        }) => {
            let mut session = build_session(
                identifier.map(extract_identifier),
                resume,
                extension,
                builtin,
            )
            .await;

            setup_logging(session.session_file().file_stem().and_then(|s| s.to_str()))?;
            let _ = session.interactive(None).await;
            return Ok(());
        }
        Some(Command::Run {
            instructions,
            input_text,
            interactive,
            identifier,
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
            let mut session = build_session(
                identifier.map(extract_identifier),
                resume,
                extension,
                builtin,
            )
            .await;
            setup_logging(session.session_file().file_stem().and_then(|s| s.to_str()))?;

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
            goose_cli::commands::update::update(canary, reconfigure)?;
            return Ok(());
        }
        Some(Command::Bench {
            suites,
            include_dirs,
            repeat,
            list,
            output,
            format,
            summary,
        }) => {
            if list {
                let suites = list_suites().await?;
                for suite in suites.keys() {
                    println!("{}: {}", suite, suites.get(suite).unwrap());
                }
                return Ok(());
            }
            let suites = if suites.is_empty() {
                vec!["core".to_string()]
            } else {
                suites
            };
            let current_dir = std::env::current_dir()?;

            for i in 0..repeat {
                if repeat > 1 {
                    println!("\nRun {} of {}:", i + 1, repeat);
                }
                let results = run_benchmark(suites.clone(), include_dirs.clone()).await?;

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
                let mut session = build_session(None, false, vec![], vec![]).await;
                setup_logging(session.session_file().file_stem().and_then(|s| s.to_str()))?;
                let _ = session.interactive(None).await;
                return Ok(());
            }
        }
    }
    Ok(())
}
