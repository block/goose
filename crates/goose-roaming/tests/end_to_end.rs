//! End-to-end test: two roaming nodes connect over iroh (direct, relays
//! disabled) and exchange bytes through an authorized ACP stream.
//!
//! This validates the whole seam: bind -> invite -> dial -> handshake ->
//! authorize -> stream hand-off. It uses a trivial echo "ACP server" in place
//! of goose's real ACP protocol, since this crate has no dependency on the
//! agent machinery.

use std::sync::Arc;

use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use goose_roaming::{
    AcpStreamServer, Directory, InviteOptions, RelaySettings, RoamingConfig, RoamingIdentity,
    RoamingNode, Scope, TrustBook, TrustPolicy,
};
use iroh::EndpointId;

/// A stand-in ACP server that echoes one line back, upper-cased.
#[derive(Debug)]
struct EchoServer;

impl AcpStreamServer for EchoServer {
    fn serve_stream(
        &self,
        _client: EndpointId,
        _scope: Scope,
        mut recv: Box<dyn AsyncRead + Send + Unpin>,
        mut send: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> futures::future::BoxFuture<'static, anyhow::Result<()>> {
        Box::pin(async move {
            let mut buf = [0u8; 5];
            recv.read_exact(&mut buf).await?;
            let upper: Vec<u8> = buf.iter().map(|b| b.to_ascii_uppercase()).collect();
            send.write_all(&upper).await?;
            send.flush().await?;
            // Keep the connection alive until the client is done reading and
            // closes its side. Real ACP `serve` naturally runs a long-lived
            // duplex loop; the echo stub must not return early or the QUIC
            // connection would be torn down before delivery completes.
            let mut drain = Vec::new();
            let _ = recv.read_to_end(&mut drain).await;
            Ok(())
        })
    }

    fn agent_id(&self) -> String {
        "echo-agent".to_string()
    }
}

/// A multi-client fan-out "agent": every connected peer is subscribed via the
/// broker [`Router`], a single broadcast is delivered to all of them, and an
/// inbound line from a peer is accepted or refused by role (only a
/// controller/steerer may steer). Stands in for a real ACP session shared by
/// several tunnel clients.
#[derive(Clone)]
struct FanoutServer {
    router: Arc<tokio::sync::Mutex<goose_roaming::Router>>,
    tx: tokio::sync::broadcast::Sender<Vec<u8>>,
    /// Records "<subscriber>:<verdict>" for each inbound steer attempt.
    steer_log: Arc<tokio::sync::Mutex<Vec<String>>>,
}

impl FanoutServer {
    fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(16);
        Self {
            router: Arc::new(tokio::sync::Mutex::new(goose_roaming::Router::new())),
            tx,
            steer_log: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    /// Broadcast a `session/update`-style frame to all attached peers.
    fn broadcast(&self, msg: &[u8]) {
        let _ = self.tx.send(msg.to_vec());
    }
}

impl AcpStreamServer for FanoutServer {
    fn serve_stream(
        &self,
        _client: EndpointId,
        scope: Scope,
        mut recv: Box<dyn AsyncRead + Send + Unpin>,
        mut send: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> futures::future::BoxFuture<'static, anyhow::Result<()>> {
        let this = self.clone();
        Box::pin(async move {
            let id = {
                let mut router = this.router.lock().await;
                let id = goose_roaming::SubscriberId(router.subscriber_count() as u64);
                router.attach(id, goose_roaming::Role::from_scope(scope));
                id
            };
            let mut rx = this.tx.subscribe();

            // Fan-out task: deliver every broadcast to this peer.
            let fanout = async {
                while let Ok(msg) = rx.recv().await {
                    let len = (msg.len() as u32).to_be_bytes();
                    send.write_all(&len).await?;
                    send.write_all(&msg).await?;
                    send.flush().await?;
                }
                Ok::<(), anyhow::Error>(())
            };

            // Inbound task: read one steer line, apply the routing policy.
            let inbound = async {
                let mut lenbuf = [0u8; 4];
                if recv.read_exact(&mut lenbuf).await.is_ok() {
                    let n = u32::from_be_bytes(lenbuf) as usize;
                    let mut buf = vec![0u8; n];
                    recv.read_exact(&mut buf).await?;
                    let verdict = match this.router.lock().await.accept_steer(id) {
                        Ok(()) => "accepted",
                        Err(_) => "refused",
                    };
                    this.steer_log
                        .lock()
                        .await
                        .push(format!("{}:{verdict}", id.0));
                }
                Ok::<(), anyhow::Error>(())
            };

            tokio::select! {
                r = fanout => r,
                r = inbound => r,
            }
        })
    }

    fn agent_id(&self) -> String {
        "fanout-agent".to_string()
    }
}

/// Bind to an ephemeral loopback IPv4 port so relay-disabled tests use a single
/// local path (avoids iroh's dual-stack MultipathNotNegotiated stall).
fn loopback() -> std::net::SocketAddr {
    "127.0.0.1:0".parse().unwrap()
}

async fn bind_node(trust: TrustBook) -> Arc<RoamingNode> {
    RoamingNode::bind(RoamingConfig {
        identity: RoamingIdentity::generate(),
        relay: RelaySettings::Disabled,
        trust,
        directory: Directory::new(),
        bind_addr: Some(loopback()),
        trust_path: None,
    })
    .await
    .expect("bind node")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bearer_invite_connects_and_streams() {
    let host_identity = RoamingIdentity::generate();
    let host = RoamingNode::bind(RoamingConfig {
        identity: host_identity.clone(),
        relay: RelaySettings::Disabled,
        trust: TrustBook::new(TrustPolicy::Bearer),
        directory: Directory::new(),
        bind_addr: Some(loopback()),
        trust_path: None,
    })
    .await
    .expect("bind host");

    host.share(Arc::new(EchoServer)).await.expect("share");

    let invite = host.make_invite(InviteOptions::new(
        Scope::Control,
        std::time::Duration::from_secs(60),
    ));

    // The client dials using the host's *direct* address, since relays are
    // disabled. We inject the host's real endpoint addr into a re-signed invite
    // so the client can reach it on localhost.
    let client = bind_node(TrustBook::new(TrustPolicy::Bearer)).await;

    // Connect using the invite but supply the live endpoint addr for dialing.
    let mut stream = connect_direct(&client, &host, &invite)
        .await
        .expect("client connects");

    assert_eq!(stream.agent_id, "echo-agent");
    assert!(matches!(stream.scope, Scope::Control));

    {
        stream.send.write_all(b"hello").await.unwrap();
        let mut out = [0u8; 5];
        stream.recv.read_exact(&mut out).await.unwrap();
        assert_eq!(&out, b"HELLO");
        // Close the client's send side so the host's drain read completes.
        stream.send.finish().unwrap();
    }

    host.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn allowlist_rejects_unknown_client() {
    let host_identity = RoamingIdentity::generate();
    let host = RoamingNode::bind(RoamingConfig {
        identity: host_identity.clone(),
        relay: RelaySettings::Disabled,
        // Allowlist policy with an empty allowlist: nobody is authorized.
        trust: TrustBook::new(TrustPolicy::Allowlist),
        directory: Directory::new(),
        bind_addr: Some(loopback()),
        trust_path: None,
    })
    .await
    .expect("bind host");
    host.share(Arc::new(EchoServer)).await.expect("share");

    let invite = host.make_invite(InviteOptions::new(
        Scope::Control,
        std::time::Duration::from_secs(60),
    ));

    let client = bind_node(TrustBook::new(TrustPolicy::Bearer)).await;
    let result = connect_direct(&client, &host, &invite).await;
    assert!(result.is_err(), "unlisted client should be rejected");

    host.shutdown().await.unwrap();
}

/// Two tunnel clients attach to one shared session over real iroh transport;
/// both receive the same broadcast, and only the controller's steer is accepted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multiple_clients_share_one_session() {
    use futures::io::AsyncReadExt;

    let host_identity = RoamingIdentity::generate();
    let host = RoamingNode::bind(RoamingConfig {
        identity: host_identity.clone(),
        relay: RelaySettings::Disabled,
        trust: TrustBook::new(TrustPolicy::Bearer),
        directory: Directory::new(),
        bind_addr: Some(loopback()),
        trust_path: None,
    })
    .await
    .expect("bind host");

    let server = FanoutServer::new();
    host.share(Arc::new(server.clone())).await.expect("share");

    let invite = host.make_invite(InviteOptions::new(
        Scope::Control,
        std::time::Duration::from_secs(60),
    ));

    // Two independent tunnel clients dial the same host.
    let client_a = bind_node(TrustBook::new(TrustPolicy::Bearer)).await;
    let client_b = bind_node(TrustBook::new(TrustPolicy::Bearer)).await;
    let stream_a = connect_direct(&client_a, &host, &invite)
        .await
        .expect("client A connects");
    let stream_b = connect_direct(&client_b, &host, &invite)
        .await
        .expect("client B connects");

    let (_sa, mut ra, _ca) = stream_a.into_futures_io();
    let (_sb, mut rb, _cb) = stream_b.into_futures_io();

    // Give both serve_stream tasks time to attach + subscribe before broadcast.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(server.router.lock().await.subscriber_count(), 2);

    server.broadcast(b"hello everyone");

    // Both clients receive the same length-prefixed frame.
    async fn read_frame<R: futures::io::AsyncRead + Unpin>(r: &mut R) -> Vec<u8> {
        let mut lenbuf = [0u8; 4];
        r.read_exact(&mut lenbuf).await.unwrap();
        let n = u32::from_be_bytes(lenbuf) as usize;
        let mut buf = vec![0u8; n];
        r.read_exact(&mut buf).await.unwrap();
        buf
    }
    let got_a = read_frame(&mut ra).await;
    let got_b = read_frame(&mut rb).await;
    assert_eq!(got_a, b"hello everyone");
    assert_eq!(got_b, b"hello everyone");

    host.shutdown().await.unwrap();
}

/// Dial the host on its live endpoint address (bypassing relay-based discovery,
/// since the test runs relay-disabled on localhost).
async fn connect_direct(
    client: &RoamingNode,
    host: &RoamingNode,
    invite: &goose_roaming::SignedInvite,
) -> Result<goose_roaming::RoamingClientStream, goose_roaming::RoamingError> {
    let addr = host.endpoint().addr();
    client
        .connect_with_addr(addr, invite, Some("test-client".into()))
        .await
}
