use anyhow::Result;
use clap::Parser;
use goose::builtin_extension::register_builtin_extensions;
use goose::config::paths::Paths;
use goose_acp::server_factory::{AcpServer, AcpServerFactoryConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser)]
#[command(name = "goose-acp-server")]
#[command(about = "ACP server for goose over HTTP and WebSocket")]
struct Cli {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value = "3284")]
    port: u16,

    #[arg(long = "builtin", action = clap::ArgAction::Append)]
    builtins: Vec<String>,
}

fn main() -> Result<()> {
    // The ACP handler chain in sacp produces an ~85 KB async state machine.
    // The default tokio worker-thread stack (2 MiB) can overflow when this
    // future is combined with deep call stacks from the agent/provider code.
    // 8 MiB gives comfortable headroom.
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()?
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();

    register_builtin_extensions(goose_mcp::BUILTIN_EXTENSIONS.clone());

    let cli = Cli::parse();

    let builtins = if cli.builtins.is_empty() {
        vec!["developer".to_string()]
    } else {
        cli.builtins
    };

    let server = Arc::new(AcpServer::new(AcpServerFactoryConfig {
        builtins,
        data_dir: Paths::data_dir(),
        config_dir: Paths::config_dir(),
    }));
    let router = goose_acp::transport::create_router(server);

    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;
    info!("Starting goose-acp-server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
