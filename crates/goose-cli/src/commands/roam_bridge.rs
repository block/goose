//! Bridges the roaming transport to goose's real ACP server.
//!
//! This is the one place that couples roaming to goose's agent machinery, so it
//! lives in the CLI (the composition boundary) rather than in the pure-transport
//! `goose-roaming` crate. It implements [`AcpStreamServer`] by creating a
//! **fresh** `GooseAcpAgent` per accepted connection (never sharing one agent
//! across clients) and driving goose's generic `acp::server::serve` over the
//! authorized iroh stream.

use std::sync::Arc;

use futures::future::BoxFuture;
use futures::io::{AsyncRead, AsyncWrite};
use goose::acp::server::serve;
use goose::acp::server_factory::AcpServer;
use goose_roaming::{AcpStreamServer, EndpointId, Scope};

/// An [`AcpStreamServer`] backed by a goose [`AcpServer`] factory.
pub struct GooseAcpBridge {
    server: Arc<AcpServer>,
    agent_id: String,
}

impl GooseAcpBridge {
    pub fn new(server: Arc<AcpServer>, agent_id: impl Into<String>) -> Self {
        Self {
            server,
            agent_id: agent_id.into(),
        }
    }
}

impl AcpStreamServer for GooseAcpBridge {
    fn serve_stream(
        &self,
        client: EndpointId,
        scope: Scope,
        recv: Box<dyn AsyncRead + Send + Unpin>,
        send: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> BoxFuture<'static, anyhow::Result<()>> {
        let server = self.server.clone();
        Box::pin(async move {
            // TODO(roaming): thread `scope` into the ACP handler via a
            // transport-neutral AcpConnectionPolicy so Observe/Attach
            // connections cannot answer tool-permission prompts (see design doc
            // §9). For now only full Control is meaningful; we log the scope.
            tracing::info!(%client, ?scope, "roaming: serving ACP session");
            let agent = server.create_agent().await?;
            serve(agent, recv, send).await
        })
    }

    fn agent_id(&self) -> String {
        self.agent_id.clone()
    }
}
