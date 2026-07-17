//! The roaming node: owns the iroh [`Endpoint`] and [`Router`], hosts agents
//! over the `goose-acp/1` ALPN, and dials remote agents as a client.

use std::sync::Arc;

use futures::io::{AsyncRead, AsyncWrite};
use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler, Router},
    Endpoint, EndpointId,
};
use tokio::sync::Mutex;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::error::RoamingError;
use crate::frame::{read_frame, write_frame};
use crate::handshake::{ClientHello, HostAck};
use crate::identity::RoamingIdentity;
use crate::invite::{Scope, SignedInvite};
use crate::relay::RelaySettings;
use crate::trust::TrustBook;

/// ALPN identifying the goose ACP-over-iroh protocol.
pub const ROAMING_ACP_ALPN: &[u8] = b"goose-acp/1";

/// Serves an accepted, authorized ACP byte stream. Implemented by the
/// integration layer (e.g. `goose-cli`) so this crate does not depend on the
/// concrete agent/session machinery.
pub trait AcpStreamServer: Send + Sync + 'static {
    /// Drive the ACP protocol to completion over the given stream, having
    /// granted `scope` to the connecting peer identified by `client`.
    fn serve_stream(
        &self,
        client: EndpointId,
        scope: Scope,
        recv: Box<dyn AsyncRead + Send + Unpin>,
        send: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> futures::future::BoxFuture<'static, anyhow::Result<()>>;

    /// A stable, human-facing id for the agent being shared, surfaced to
    /// clients in the handshake ack.
    fn agent_id(&self) -> String;
}

/// Configuration for binding a roaming node.
pub struct RoamingConfig {
    pub identity: RoamingIdentity,
    pub relay: RelaySettings,
    pub trust: TrustBook,
}

/// A bound roaming node.
pub struct RoamingNode {
    endpoint: Endpoint,
    router: Mutex<Option<Router>>,
    trust: Arc<Mutex<TrustBook>>,
}

impl RoamingNode {
    /// Bind the iroh endpoint. Does not start accepting until [`Self::share`]
    /// (or a manual router) is set up.
    pub async fn bind(config: RoamingConfig) -> Result<Arc<Self>, RoamingError> {
        let relay_mode = config.relay.to_relay_mode()?;
        let endpoint = Endpoint::builder(iroh::endpoint::presets::Minimal)
            .secret_key(config.identity.secret_key().clone())
            .relay_mode(relay_mode)
            .bind()
            .await
            .map_err(|e| RoamingError::Transport(format!("failed to bind endpoint: {e}")))?;

        Ok(Arc::new(Self {
            endpoint,
            router: Mutex::new(None),
            trust: Arc::new(Mutex::new(config.trust)),
        }))
    }

    /// The node's public key / endpoint id.
    pub fn endpoint_id(&self) -> EndpointId {
        self.endpoint.id()
    }

    /// Access the underlying iroh endpoint (advanced use).
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Shared trust book (for CLI commands to inspect/mutate).
    pub fn trust(&self) -> Arc<Mutex<TrustBook>> {
        self.trust.clone()
    }

    /// Start accepting inbound ACP connections, serving each authorized stream
    /// via `server`. Returns once the router is spawned; it runs in the
    /// background until [`Self::shutdown`].
    pub async fn share(
        self: &Arc<Self>,
        server: Arc<dyn AcpStreamServer>,
    ) -> Result<(), RoamingError> {
        let handler = RoamingAcpHandler {
            node: self.clone(),
            server,
        };
        let router = Router::builder(self.endpoint.clone())
            .accept(ROAMING_ACP_ALPN, handler)
            .spawn();
        *self.router.lock().await = Some(router);
        Ok(())
    }

    /// Mint a signed invite for this node with the given scope and validity.
    pub fn make_invite(
        &self,
        identity: &RoamingIdentity,
        relay: &RelaySettings,
        scope: Scope,
        allowed_client_keys: Vec<EndpointId>,
        ttl_secs: u64,
        single_use: bool,
    ) -> SignedInvite {
        let now = now_ms();
        let claims = crate::invite::InviteClaims {
            version: 1,
            audience: self.endpoint_id(),
            relay_urls: relay.advertised_urls(),
            scope,
            allowed_client_keys,
            token_id: random_token_id(),
            not_before_ms: now,
            expires_at_ms: now + ttl_secs * 1000,
            single_use,
        };
        SignedInvite::sign(identity.secret_key(), claims)
    }

    /// Cleanly shut the router and endpoint down.
    pub async fn shutdown(&self) -> Result<(), RoamingError> {
        if let Some(router) = self.router.lock().await.take() {
            router
                .shutdown()
                .await
                .map_err(|e| RoamingError::Transport(format!("router shutdown: {e}")))?;
        }
        self.endpoint.close().await;
        Ok(())
    }

    /// Dial a remote agent using a decoded invite, returning the authorized
    /// bi-stream halves ready to feed to an ACP client transport.
    ///
    /// The dial target is reconstructed from the invite (audience id + relay
    /// URLs). Use [`Self::connect_with_addr`] when the caller already has a
    /// dialable [`EndpointAddr`] (e.g. a direct LAN address learned out of
    /// band).
    pub async fn connect(
        &self,
        invite: &SignedInvite,
        label: Option<String>,
    ) -> Result<RoamingClientStream, RoamingError> {
        let addr = invite.endpoint_addr()?;
        self.connect_with_addr(addr, invite, label).await
    }

    /// Dial a remote agent at an explicit [`EndpointAddr`], authorizing with the
    /// given invite.
    pub async fn connect_with_addr(
        &self,
        addr: iroh::EndpointAddr,
        invite: &SignedInvite,
        label: Option<String>,
    ) -> Result<RoamingClientStream, RoamingError> {
        invite.verify(now_ms())?;
        let conn = self
            .endpoint
            .connect(addr, ROAMING_ACP_ALPN)
            .await
            .map_err(|e| RoamingError::Transport(format!("connect failed: {e}")))?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| RoamingError::Transport(format!("open_bi failed: {e}")))?;

        let hello = ClientHello::new(invite.clone(), label);
        let hello_bytes = serde_json::to_vec(&hello)
            .map_err(|e| RoamingError::Transport(format!("encode hello: {e}")))?;
        write_frame(&mut send, &hello_bytes).await?;

        let ack_bytes = read_frame(&mut recv).await?;
        let ack: HostAck = serde_json::from_slice(&ack_bytes)
            .map_err(|e| RoamingError::Transport(format!("decode ack: {e}")))?;

        match ack {
            HostAck::Accepted { scope, agent_id } => Ok(RoamingClientStream {
                scope,
                agent_id,
                conn,
                send,
                recv,
            }),
            HostAck::Rejected { code } => Err(RoamingError::Rejected(code)),
        }
    }
}

/// A dialed, authorized client stream to a remote agent.
pub struct RoamingClientStream {
    pub scope: Scope,
    pub agent_id: String,
    /// Kept alive so the connection isn't dropped while the stream is in use.
    pub conn: Connection,
    pub send: iroh::endpoint::SendStream,
    pub recv: iroh::endpoint::RecvStream,
}

struct RoamingAcpHandler {
    node: Arc<RoamingNode>,
    server: Arc<dyn AcpStreamServer>,
}

impl std::fmt::Debug for RoamingAcpHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RoamingAcpHandler")
    }
}

impl ProtocolHandler for RoamingAcpHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let client = connection.remote_id();

        let (mut send, mut recv) = connection.accept_bi().await?;

        let decision = self.authorize(client, &mut recv).await;
        match decision {
            Ok(scope) => {
                let ack = HostAck::Accepted {
                    scope,
                    agent_id: self.server.agent_id(),
                };
                if let Err(e) = send_ack(&mut send, &ack).await {
                    tracing::warn!("roaming: failed to send accept ack: {e}");
                    return Ok(());
                }
                let recv_box: Box<dyn AsyncRead + Send + Unpin> = Box::new(recv.compat());
                let send_box: Box<dyn AsyncWrite + Send + Unpin> = Box::new(send.compat_write());
                if let Err(e) = self
                    .server
                    .serve_stream(client, scope, recv_box, send_box)
                    .await
                {
                    tracing::warn!("roaming: ACP session ended with error: {e}");
                }
            }
            Err(reason) => {
                let ack = HostAck::Rejected {
                    code: reason.to_string(),
                };
                let _ = send_ack(&mut send, &ack).await;
                tracing::info!(%client, reason = %reason, "roaming: rejected connection");
            }
        }
        Ok(())
    }
}

impl RoamingAcpHandler {
    async fn authorize(
        &self,
        client: EndpointId,
        recv: &mut iroh::endpoint::RecvStream,
    ) -> Result<Scope, String> {
        let hello_bytes = read_frame(recv).await.map_err(|e| e.to_string())?;
        let hello: ClientHello =
            serde_json::from_slice(&hello_bytes).map_err(|e| format!("bad hello: {e}"))?;

        hello
            .invite
            .verify(now_ms())
            .map_err(|_| "invalid_capability".to_string())?;

        if hello.invite.claims.audience != self.node.endpoint_id() {
            return Err("invalid_capability".to_string());
        }
        if !hello.invite.permits_client(&client) {
            return Err("not_allowlisted".to_string());
        }

        let mut trust = self.node.trust.lock().await;
        if trust.is_key_revoked(&client) {
            return Err("revoked".to_string());
        }
        let token_id = &hello.invite.claims.token_id;
        if hello.invite.claims.single_use {
            if !trust.redeem_single_use(token_id) {
                return Err("invalid_capability".to_string());
            }
            trust.allow(&client);
        } else if trust.is_token_revoked(token_id) {
            return Err("invalid_capability".to_string());
        }
        if !trust.is_allowed(&client) {
            return Err("not_allowlisted".to_string());
        }

        Ok(hello.invite.claims.scope)
    }
}

async fn send_ack(
    send: &mut iroh::endpoint::SendStream,
    ack: &HostAck,
) -> Result<(), RoamingError> {
    let bytes =
        serde_json::to_vec(ack).map_err(|e| RoamingError::Transport(format!("encode ack: {e}")))?;
    write_frame(send, &bytes).await
}

fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}

fn random_token_id() -> String {
    let mut bytes = [0u8; 16];
    rand::fill(&mut bytes);
    let mut s = String::with_capacity(32);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
