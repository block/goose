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

use crate::directory::{Direction, Directory};
use crate::error::RoamingError;
use crate::frame::{read_frame, write_frame};
use crate::handshake::{ClientHello, HostAck};
use crate::identity::RoamingIdentity;
use crate::invite::{Scope, SignedInvite};
use crate::relay::RelaySettings;
use crate::trust::{TrustBook, TrustPolicy};

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
///
/// For the common case use [`RoamingConfig::new`] and the `with_*` chainers,
/// which default to iroh's public relays, a bearer trust policy, and an
/// in-memory directory:
///
/// ```no_run
/// use goose_roaming::{RoamingConfig, RoamingIdentity, RoamingNode};
/// # async fn f() -> anyhow::Result<()> {
/// let node = RoamingNode::bind(RoamingConfig::new(RoamingIdentity::generate())).await?;
/// # Ok(()) }
/// ```
pub struct RoamingConfig {
    pub identity: RoamingIdentity,
    pub relay: RelaySettings,
    pub trust: TrustBook,
    /// Directory used to track observed connections. Defaults to an in-memory
    /// directory; pass [`Directory::persistent`] to make `roam list` work from
    /// a separate process.
    pub directory: Directory,
    /// Optional explicit socket address to bind the QUIC endpoint to. When set
    /// with relays disabled, the default IP transports are cleared first so a
    /// single-family local path is used — iroh's multipath negotiation
    /// otherwise stalls (`MultipathNotNegotiated`) when both the specified IPv4
    /// and a default `[::]` IPv6 socket are candidates with no relay fallback.
    pub bind_addr: Option<std::net::SocketAddr>,
    /// Optional path to persist the [`TrustBook`] to. When set, admission
    /// changes made during a live session — pairing pinning a client key,
    /// single-use tokens being consumed — are flushed here so they survive a
    /// restart. When `None` the trust state is in-memory only.
    pub trust_path: Option<std::path::PathBuf>,
}

impl RoamingConfig {
    /// A config for `identity` with sensible defaults: iroh's public relays,
    /// bearer trust (anyone with a valid invite), an in-memory directory, and
    /// no explicit bind address.
    pub fn new(identity: RoamingIdentity) -> Self {
        Self {
            identity,
            relay: RelaySettings::N0Default,
            trust: TrustBook::new(TrustPolicy::Bearer),
            directory: Directory::new(),
            bind_addr: None,
            trust_path: None,
        }
    }

    /// Use a specific relay configuration (default: iroh's public relays).
    pub fn with_relay(mut self, relay: RelaySettings) -> Self {
        self.relay = relay;
        self
    }

    /// Use a specific trust policy / allowlist (default: bearer).
    pub fn with_trust(mut self, trust: TrustBook) -> Self {
        self.trust = trust;
        self
    }

    /// Track observed connections in `directory` (default: in-memory).
    pub fn with_directory(mut self, directory: Directory) -> Self {
        self.directory = directory;
        self
    }

    /// Bind the QUIC endpoint to a specific socket address.
    pub fn with_bind_addr(mut self, addr: std::net::SocketAddr) -> Self {
        self.bind_addr = Some(addr);
        self
    }

    /// Persist trust changes (pairing, single-use redemption) to `path` so they
    /// survive a restart (default: in-memory only).
    pub fn with_trust_path(mut self, path: std::path::PathBuf) -> Self {
        self.trust_path = Some(path);
        self
    }
}

/// Options for minting a [`SignedInvite`] via [`RoamingNode::make_invite`].
pub struct InviteOptions {
    /// Capability granted to the connecting client.
    pub scope: Scope,
    /// How long the invite is valid for.
    pub ttl: std::time::Duration,
    /// If non-empty, only these client keys may redeem the invite.
    pub allowed_client_keys: Vec<EndpointId>,
    /// If true, the invite is consumed on first redemption and the redeeming
    /// client's key is pinned to the host's allowlist (pairing).
    pub single_use: bool,
}

impl InviteOptions {
    /// A `Control` invite valid for `ttl`, bearer (any holder), reusable.
    pub fn new(scope: Scope, ttl: std::time::Duration) -> Self {
        Self {
            scope,
            ttl,
            allowed_client_keys: Vec::new(),
            single_use: false,
        }
    }

    /// Restrict redemption to a specific client key (repeatable).
    pub fn allow_client(mut self, id: EndpointId) -> Self {
        self.allowed_client_keys.push(id);
        self
    }

    /// Make the invite single-use (pairing): the first redeemer's key is pinned.
    pub fn single_use(mut self) -> Self {
        self.single_use = true;
        self
    }
}

/// A bound roaming node.
pub struct RoamingNode {
    endpoint: Endpoint,
    router: Mutex<Option<Router>>,
    trust: Arc<Mutex<TrustBook>>,
    trust_path: Option<std::path::PathBuf>,
    directory: Directory,
    identity: RoamingIdentity,
    relay: RelaySettings,
}

impl RoamingNode {
    /// Bind the iroh endpoint. Does not start accepting until [`Self::share`]
    /// (or a manual router) is set up.
    pub async fn bind(config: RoamingConfig) -> Result<Arc<Self>, RoamingError> {
        let relay_mode = config.relay.to_relay_mode()?;
        let identity = config.identity.clone();
        let relay = config.relay.clone();
        let relays_disabled = matches!(config.relay, RelaySettings::Disabled);
        let mut builder = Endpoint::builder(iroh::endpoint::presets::Minimal)
            .secret_key(config.identity.secret_key().clone())
            .relay_mode(relay_mode);
        if let Some(addr) = config.bind_addr {
            if relays_disabled && addr.is_ipv4() {
                builder = builder.clear_ip_transports();
            }
            builder = builder
                .bind_addr(addr)
                .map_err(|e| RoamingError::Transport(format!("invalid bind address: {e}")))?;
        }
        let endpoint = builder
            .bind()
            .await
            .map_err(|e| RoamingError::Transport(format!("failed to bind endpoint: {e}")))?;

        Ok(Arc::new(Self {
            endpoint,
            router: Mutex::new(None),
            trust: Arc::new(Mutex::new(config.trust)),
            trust_path: config.trust_path,
            directory: config.directory,
            identity,
            relay,
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

    /// The connected-peers directory, built out of band from observed
    /// connections (no gossip).
    pub fn directory(&self) -> &Directory {
        &self.directory
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

    /// Wait (up to `timeout`) for the endpoint to contact a relay and be
    /// reachable. Returns `true` if it came online.
    pub async fn wait_online(&self, timeout: std::time::Duration) -> bool {
        tokio::time::timeout(timeout, self.endpoint.online())
            .await
            .is_ok()
    }

    /// The endpoint's currently-known relay URLs, read from its live address.
    /// These are what let a client reach this node when no static relay URLs
    /// are configured (e.g. under [`RelaySettings::N0Default`]).
    pub fn live_relay_urls(&self) -> Vec<String> {
        self.endpoint
            .addr()
            .addrs
            .into_iter()
            .filter_map(|addr| match addr {
                iroh::TransportAddr::Relay(url) => Some(url.to_string()),
                _ => None,
            })
            .collect()
    }

    /// Mint a signed invite for this node with the given scope and validity.
    ///
    /// The invite advertises the configured relay URLs plus the endpoint's
    /// live relay URL(s), so a client can reach this node through a relay.
    /// Call [`Self::wait_online`] first so a live relay URL is available.
    /// Mint a signed invite for this node, using the identity and relay settings
    /// it was bound with. Relay URLs advertised in the invite merge the
    /// configured relays with any live relay the endpoint has since discovered.
    pub fn make_invite(&self, options: InviteOptions) -> SignedInvite {
        let now = now_ms();
        let mut relay_urls = self.relay.advertised_urls();
        for url in self.live_relay_urls() {
            if !relay_urls.contains(&url) {
                relay_urls.push(url);
            }
        }
        let claims = crate::invite::InviteClaims {
            version: 1,
            audience: self.endpoint_id(),
            relay_urls,
            scope: options.scope,
            allowed_client_keys: options.allowed_client_keys,
            token_id: random_token_id(),
            not_before_ms: now,
            expires_at_ms: now + options.ttl.as_secs() * 1000,
            single_use: options.single_use,
        };
        SignedInvite::sign(self.identity.secret_key(), claims)
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
            HostAck::Accepted { scope, agent_id } => {
                self.directory
                    .record_connect(
                        conn.remote_id(),
                        None,
                        Direction::Outbound,
                        scope,
                        Some(agent_id.clone()),
                        now_ms(),
                    )
                    .await;
                Ok(RoamingClientStream {
                    scope,
                    agent_id,
                    conn,
                    send,
                    recv,
                })
            }
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

impl RoamingClientStream {
    /// Capability the host granted this connection.
    pub fn scope(&self) -> Scope {
        self.scope
    }

    /// The host-facing id of the agent on the other end.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// The authenticated remote endpoint id (the host's public key).
    pub fn peer_id(&self) -> EndpointId {
        self.conn.remote_id()
    }

    /// Consume the stream into `futures::io` read/write halves ready to feed to
    /// an ACP client transport (e.g. `ByteStreams::new(send, recv)`), plus the
    /// live [`Connection`] which the caller must keep alive for the duration of
    /// the session. This saves consumers from repeating the tokio-compat dance.
    pub fn into_futures_io(
        self,
    ) -> (
        impl AsyncWrite + Send + Unpin,
        impl AsyncRead + Send + Unpin,
        Connection,
    ) {
        (self.send.compat_write(), self.recv.compat(), self.conn)
    }
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
            Ok((scope, label)) => {
                let agent_id = self.server.agent_id();
                let ack = HostAck::Accepted {
                    scope,
                    agent_id: agent_id.clone(),
                };
                if let Err(e) = send_ack(&mut send, &ack).await {
                    tracing::warn!("roaming: failed to send accept ack: {e}");
                    return Ok(());
                }
                self.node
                    .directory
                    .record_connect(
                        client,
                        label,
                        Direction::Inbound,
                        scope,
                        Some(agent_id),
                        now_ms(),
                    )
                    .await;
                let recv_box: Box<dyn AsyncRead + Send + Unpin> = Box::new(recv.compat());
                let send_box: Box<dyn AsyncWrite + Send + Unpin> = Box::new(send.compat_write());
                if let Err(e) = self
                    .server
                    .serve_stream(client, scope, recv_box, send_box)
                    .await
                {
                    tracing::warn!("roaming: ACP session ended with error: {e}");
                }
                self.node
                    .directory
                    .record_disconnect(client, now_ms())
                    .await;
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
    ) -> Result<(Scope, Option<String>), String> {
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
        let mut trust_changed = false;
        if hello.invite.claims.single_use {
            if !trust.redeem_single_use(token_id) {
                return Err("invalid_capability".to_string());
            }
            trust.allow(&client);
            trust_changed = true;
        } else if trust.is_token_revoked(token_id) {
            return Err("invalid_capability".to_string());
        }
        if !trust.is_allowed(&client) {
            return Err("not_allowlisted".to_string());
        }

        // Pairing pinned a key and consumed a single-use token; persist so the
        // relationship survives a restart (and the token stays consumed).
        if trust_changed {
            if let Some(path) = &self.node.trust_path {
                if let Err(e) = trust.save(path) {
                    tracing::warn!("roaming: failed to persist trust book: {e}");
                }
            }
        }
        drop(trust);

        Ok((hello.invite.claims.scope, hello.label))
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
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
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
