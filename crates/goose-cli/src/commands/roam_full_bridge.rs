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

use std::sync::{Arc, Weak};

use futures::future::BoxFuture;
use futures::io::{AsyncRead, AsyncWrite};

use goose::acp::server::{serve, GooseAcpAgent};
use goose::acp::server_factory::AcpServer;
use goose_roaming::{AcpStreamServer, EndpointId, RevocableWork, RevocationSignal};

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

/// Revocation handle for a connection-agent's detached runs, registered with
/// the roaming node so they stay revocable by peer key after the connection
/// ends: an authorized peer may disconnect normally mid-run, and a later
/// `peers revoke` must still stop that run even though there is no live
/// connection left to force-close.
///
/// Holds the agent weakly, which encodes the intended lifetime: each detached
/// prompt task owns an `Arc` of its agent for the task's whole duration (see
/// the `PromptRequest` handler in `acp/server/dispatch.rs`), so this handle
/// stays alive exactly while the stream is being served or any of the agent's
/// runs are still executing, and reports dead — and is pruned by the node —
/// once both are over.
struct RevocableAgentRuns {
    agent: Weak<GooseAcpAgent>,
}

impl RevocableWork for RevocableAgentRuns {
    fn is_alive(&self) -> bool {
        self.agent.strong_count() > 0
    }

    fn revoke(&self) -> BoxFuture<'static, ()> {
        let agent = self.agent.upgrade();
        Box::pin(async move {
            if let Some(agent) = agent {
                let cancelled = agent.revoke_and_cancel_own_runs().await;
                if cancelled > 0 {
                    tracing::info!(
                        cancelled,
                        "roaming: cancelled detached run(s) of revoked peer"
                    );
                }
            }
        })
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
            // Registered before serving so no prompt can start a run that a
            // revocation cannot reach; the node revokes it by peer key even
            // after this connection — and this future — are long gone.
            revocation
                .register_revocable_work(Arc::new(RevocableAgentRuns {
                    agent: Arc::downgrade(&agent),
                }))
                .await;
            let result = serve(agent.clone(), recv, send).await;
            // Detached prompt runs deliberately survive ordinary transport
            // loss so a reconnecting peer can `session/load` the finished
            // work. Revocation is different: the node force-closed this
            // connection because the peer's authority was withdrawn, so its
            // in-flight turns must stop rather than keep executing tools and
            // consuming the provider — and any prompt still racing through
            // dispatch must be fenced out, not just the runs already
            // registered. Fencing this agent permanently is safe because it
            // is per-connection and this future owns it: an ordinary
            // disconnect never sets the fence, and a peer that merely lost
            // its network gets a fresh, unfenced agent on reconnect.
            if revocation.is_revoked() {
                let cancelled = agent.revoke_and_cancel_own_runs().await;
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
