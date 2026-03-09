use std::io::IsTerminal;
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "goose-acp", about = "Goose AI agent — ACP edition")]
struct Cli {
    /// Run as ACP server instead of interactive CLI
    #[arg(long)]
    server: bool,

    /// Server port (default: 3284)
    #[arg(long, default_value = "3284")]
    port: u16,

    /// Server bind address (default: 127.0.0.1)
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Resume a previous session by ID
    #[arg(long, value_name = "ID", conflicts_with = "prompt")]
    session: Option<String>,

    /// One-shot prompt (non-interactive)
    #[arg(long)]
    prompt: Option<String>,

    /// Run a recipe from a YAML file
    #[arg(long, value_name = "FILE")]
    recipe: Option<PathBuf>,

    /// Auto-approve all tool calls
    #[arg(long, alias = "yolo")]
    auto_approve: bool,

    /// Stream tokens directly without viewport rewrite (also auto-enabled for TERM=dumb / NO_COLOR)
    #[arg(long)]
    plain_stream: bool,
}

fn default_filter(level: &str) -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        // Don't re-panic on BrokenPipe — standard CLI behavior (e.g. `goose 2>&1 | head`)
        let is_broken_pipe = info
            .payload()
            .downcast_ref::<String>()
            .map(|s| s.contains("Broken pipe"))
            .unwrap_or_else(|| {
                info.payload()
                    .downcast_ref::<&str>()
                    .map(|s| s.contains("Broken pipe"))
                    .unwrap_or(false)
            });
        if is_broken_pipe {
            std::process::exit(0);
        }
        default_hook(info);
    }));

    let cli = Cli::parse();

    // Server mode: stderr logging is fine (no TUI). Interactive: file-only to avoid interleaving.
    if cli.server {
        tracing_subscriber::fmt()
            .with_env_filter(default_filter("info"))
            .init();
        return goose_cli_acp::transport::run_server(cli.host, cli.port).await;
    }

    if let Ok(log_dir) = goose::logging::prepare_log_directory("cli-acp", true) {
        let file_appender = tracing_appender::rolling::RollingFileAppender::new(
            tracing_appender::rolling::Rotation::NEVER,
            log_dir,
            format!("{}.log", chrono::Local::now().format("%Y%m%d_%H%M%S")),
        );
        tracing_subscriber::fmt()
            .with_env_filter(default_filter("info"))
            .with_writer(file_appender)
            .with_ansi(false)
            .init();
    } else {
        // Fallback: stderr with high filter to minimize noise on broken log dir.
        tracing_subscriber::fmt()
            .with_env_filter(default_filter("warn"))
            .init();
    }

    if let Some(recipe_path) = cli.recipe {
        return goose_cli_acp::run::run_recipe(&recipe_path, cli.auto_approve).await;
    }

    let plain_stream = cli.plain_stream
        || std::env::var("NO_COLOR").is_ok()
        || std::env::var("TERM").as_deref() == Ok("dumb");

    let prompt = cli.prompt.or_else(|| {
        if std::io::stdin().is_terminal() {
            None
        } else {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).ok()?;
            let trimmed = buf.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            }
        }
    });

    match prompt {
        Some(p) => goose_cli_acp::run::run_single_shot(&p, cli.auto_approve).await,
        None => goose_cli_acp::run::run(cli.session, cli.auto_approve, plain_stream).await,
    }
}
