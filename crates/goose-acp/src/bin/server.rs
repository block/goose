use anyhow::Result;
use clap::Parser;
use goose_acp::{
    http::{self, HttpState},
    server_factory::{AcpServer, AcpServerFactoryConfig},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser)]
#[command(name = "goose-acp-server")]
#[command(about = "ACP server for goose over streamable HTTP")]
struct Cli {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value = "3284")]
    port: u16,

    #[arg(long = "builtin", action = clap::ArgAction::Append)]
    builtins: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();

    let cli = Cli::parse();

    let builtins = if cli.builtins.is_empty() {
        vec!["developer".to_string()]
    } else {
        cli.builtins
    };

    let config = AcpServerFactoryConfig {
        builtins,
        ..Default::default()
    };

    let server = Arc::new(AcpServer::new(config));
    let state = Arc::new(HttpState::new(server));

    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;
    info!("Starting goose-acp-server on {}", addr);

    http::serve(state, addr).await?;

    Ok(())
}
