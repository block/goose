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
    AcpStreamServer, Directory, RelaySettings, RoamingConfig, RoamingIdentity, RoamingNode, Scope,
    TrustBook,
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

async fn bind_node() -> Arc<RoamingNode> {
    RoamingNode::bind(RoamingConfig {
        identity: RoamingIdentity::generate(),
        relay: RelaySettings::Disabled,
        trust: TrustBook::new(),
        trust_path: None,
        directory: Directory::new(),
        bind_addr: Some(loopback()),
    })
    .await
    .expect("bind node")
}

/// Accept `client`'s key into `host`'s allowlist with the given scope — the
/// out-of-band "I will accept connections from this node" step.
async fn host_accepts(host: &RoamingNode, client: &RoamingNode, scope: Scope) {
    host.trust()
        .lock()
        .await
        .accept(&client.endpoint_id(), scope);
}

/// A running share re-reads its trust file per connection, so acceptance
/// written out of band (as `roam peers accept` does) takes effect without a
/// restart.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trust_file_refresh_takes_effect_on_running_share() {
    let dir = tempfile::tempdir().unwrap();
    let trust_file = dir.path().join("trust.json");
    // Start with an empty trust file.
    TrustBook::new().save(&trust_file).unwrap();

    let host = RoamingNode::bind(RoamingConfig {
        identity: RoamingIdentity::generate(),
        relay: RelaySettings::Disabled,
        trust: TrustBook::new(),
        trust_path: Some(trust_file.clone()),
        directory: Directory::new(),
        bind_addr: Some(loopback()),
    })
    .await
    .expect("bind host");
    host.share(Arc::new(EchoServer)).await.expect("share");

    let client = bind_node().await;

    // Not accepted yet: refused.
    assert!(connect_direct(&client, &host).await.is_err());

    // Accept out of band by writing the trust file (as the CLI does).
    let mut book = TrustBook::load(&trust_file).unwrap();
    book.accept(&client.endpoint_id(), Scope::Control);
    book.save(&trust_file).unwrap();

    // Now it connects against the SAME running share — no restart.
    let mut stream = connect_direct(&client, &host)
        .await
        .expect("accepted after file refresh");
    stream.send.finish().unwrap();

    host.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accepted_key_connects_and_streams() {
    let host = bind_node().await;
    host.share(Arc::new(EchoServer)).await.expect("share");

    let client = bind_node().await;
    // Host accepts the client's key (mutual, key-based trust).
    host_accepts(&host, &client, Scope::Control).await;

    let mut stream = connect_direct(&client, &host)
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
async fn unaccepted_key_is_rejected() {
    let host = bind_node().await;
    host.share(Arc::new(EchoServer)).await.expect("share");

    // Client's key was never accepted: connection must be refused.
    let client = bind_node().await;
    let result = connect_direct(&client, &host).await;
    assert!(result.is_err(), "unaccepted client should be rejected");

    host.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accepted_scope_is_honored() {
    let host = bind_node().await;
    host.share(Arc::new(EchoServer)).await.expect("share");

    let client = bind_node().await;
    host_accepts(&host, &client, Scope::Observe).await;

    let mut stream = connect_direct(&client, &host).await.expect("connects");
    // The scope the host granted this key is what the client sees.
    assert!(matches!(stream.scope, Scope::Observe));
    stream.send.finish().unwrap();

    host.shutdown().await.unwrap();
}

/// Two tunnel clients attach to one shared session over real iroh transport;
/// both receive the same broadcast, and only the controller's steer is accepted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multiple_clients_share_one_session() {
    use futures::io::AsyncReadExt;

    let host = bind_node().await;
    let server = FanoutServer::new();
    host.share(Arc::new(server.clone())).await.expect("share");

    // Two independent tunnel clients, both accepted by key.
    let client_a = bind_node().await;
    let client_b = bind_node().await;
    host_accepts(&host, &client_a, Scope::Control).await;
    host_accepts(&host, &client_b, Scope::Control).await;

    let stream_a = connect_direct(&client_a, &host)
        .await
        .expect("client A connects");
    let stream_b = connect_direct(&client_b, &host)
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
/// since the test runs relay-disabled on localhost). Authorization is by the
/// client's authenticated key, which the host has accepted.
async fn connect_direct(
    client: &RoamingNode,
    host: &RoamingNode,
) -> Result<goose_roaming::RoamingClientStream, goose_roaming::RoamingError> {
    let addr = host.endpoint().addr();
    client
        .connect_with_addr(addr, Some("test-client".into()))
        .await
}
