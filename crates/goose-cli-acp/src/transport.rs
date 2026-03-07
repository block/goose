use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use goose::builtin_extension::register_builtin_extensions;
use goose::config::paths::Paths;
use goose_acp::server::GooseAcpAgent;
use goose_acp::server_factory::{AcpServer, AcpServerFactoryConfig};
use tokio::task::JoinHandle;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub type Transport = sacp::ByteStreams<
    tokio_util::compat::Compat<tokio::io::DuplexStream>,
    tokio_util::compat::Compat<tokio::io::DuplexStream>,
>;

pub fn create_acp_server() -> AcpServer {
    register_builtin_extensions(goose_mcp::BUILTIN_EXTENSIONS.clone());
    AcpServer::new(AcpServerFactoryConfig {
        builtins: vec!["developer".to_string()],
        data_dir: Paths::data_dir(),
        config_dir: Paths::config_dir(),
    })
}

pub async fn create_agent() -> Result<Arc<GooseAcpAgent>> {
    create_acp_server().create_agent().await
}

pub async fn serve_in_process(agent: Arc<GooseAcpAgent>) -> Result<(Transport, JoinHandle<()>)> {
    // 1MB: large tool outputs (read_file, shell) can exceed 64KB; in-process so cost is negligible.
    let (client_read, server_write) = tokio::io::duplex(1024 * 1024);
    let (server_read, client_write) = tokio::io::duplex(1024 * 1024);

    let handle = tokio::spawn(async move {
        if let Err(e) =
            goose_acp::server::serve(agent, server_read.compat(), server_write.compat_write()).await
        {
            tracing::error!("ACP server error: {e}");
        }
    });

    let transport = sacp::ByteStreams::new(client_write.compat_write(), client_read.compat());
    Ok((transport, handle))
}

/// Run as a standalone HTTP server (used by `--server` CLI flag).
pub async fn run_server(host: String, port: u16) -> Result<()> {
    let server = Arc::new(create_acp_server());
    let router = goose_acp::transport::create_router(server);
    let addr: SocketAddr = format!("{host}:{port}").parse()?;

    eprintln!("ACP server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
