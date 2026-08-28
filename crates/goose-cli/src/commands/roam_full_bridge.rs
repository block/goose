//! The roaming host adapter: serve goose's **full** ACP surface to each
//! connecting peer.
//!
//! Roaming is just an authenticated p2p ACP transport. This adapter is the thin
//! seam that hands an authorized iroh stream straight to goose's real
//! `acp::server::serve`, so a connected client gets the entire ACP surface —
//! `session/new`, `session/list`, `session/load`, `session/prompt` — backed by
//! the host's own `SessionManager`. Anything "session-shaped" is therefore plain
//! ACP that happens to run over a roaming connection; roaming adds no session
//! semantics of its own.
//!
//! Each accepted connection gets a **fresh** agent (never one shared across
//! clients); every client drives its own independent sessions.

use std::sync::Arc;

use futures::future::BoxFuture;
use futures::io::{AsyncRead, AsyncWrite};

use goose::acp::server::serve;
use goose::acp::server_factory::AcpServer;
use goose_roaming::{AcpStreamServer, EndpointId, RevocationSignal};

/// An [`AcpStreamServer`] that serves goose's full ACP surface, a fresh agent
/// per connection.
pub struct FullAcpBridge {
    server: Arc<AcpServer>,
    agent_id: String,
    /// Host-controlled working directory for sessions created over roaming.
    /// The connector's machine-local absolute path is meaningless on this
    /// host, so every roaming agent gets this instead — even when the shared
    /// `AcpServer` (e.g. `goose serve`) leaves `session_cwd` unset for its
    /// local clients.
    session_cwd: std::path::PathBuf,
}

impl FullAcpBridge {
    pub fn new(
        server: Arc<AcpServer>,
        agent_id: impl Into<String>,
        session_cwd: std::path::PathBuf,
    ) -> Self {
        Self {
            server,
            agent_id: agent_id.into(),
            session_cwd,
        }
    }
}

impl AcpStreamServer for FullAcpBridge {
    fn serve_stream(
        &self,
        client: EndpointId,
        recv: Box<dyn AsyncRead + Send + Unpin>,
        send: Box<dyn AsyncWrite + Send + Unpin>,
        revocation: RevocationSignal,
    ) -> BoxFuture<'static, anyhow::Result<()>> {
        let server = self.server.clone();
        let session_cwd = self.session_cwd.clone();
        Box::pin(async move {
            tracing::info!(%client, "roaming: serving full ACP surface");
            let agent = server
                .create_agent_with_session_cwd(Some(session_cwd))
                .await?;
            let result = serve(agent.clone(), recv, send).await;
            // Detached prompt runs deliberately survive ordinary transport
            // loss so a reconnecting peer can `session/load` the finished
            // work. Revocation is different: the node force-closed this
            // connection because the peer's authority was withdrawn, so its
            // in-flight turns must stop rather than keep executing tools and
            // consuming the provider.
            if revocation.is_revoked() {
                let cancelled = agent.cancel_own_active_runs().await;
                if cancelled > 0 {
                    tracing::info!(
                        %client,
                        cancelled,
                        "roaming: cancelled active run(s) of revoked peer"
                    );
                }
            }
            result
        })
    }

    fn agent_id(&self) -> String {
        self.agent_id.clone()
    }
}
