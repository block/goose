//! End-to-end test: two roaming nodes connect over iroh (direct, relays
//! disabled) and exchange bytes through an authorized ACP stream.
//!
//! This validates the whole seam: bind -> swap identities -> accept key -> dial
//! -> handshake -> authorize -> stream hand-off. It uses a trivial echo "ACP
//! server" in place of goose's real ACP protocol, since this crate has no
//! dependency on the agent machinery.

use std::sync::Arc;

use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use goose_roaming::{
    AcpStreamServer, Directory, RelaySettings, RoamingConfig, RoamingIdentity, RoamingNode,
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

/// Accept `client`'s key into `host`'s allowlist — the out-of-band "I will
/// accept connections from this node" step.
async fn host_accepts(host: &RoamingNode, client: &RoamingNode) {
    host.trust().lock().await.accept(&client.endpoint_id());
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
    book.accept(&client.endpoint_id());
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
    host_accepts(&host, &client).await;

    let mut stream = connect_direct(&client, &host)
        .await
        .expect("client connects");

    assert_eq!(stream.agent_id, "echo-agent");

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
async fn revoked_key_is_rejected() {
    let host = bind_node().await;
    host.share(Arc::new(EchoServer)).await.expect("share");

    let client = bind_node().await;
    host_accepts(&host, &client).await;
    // Revoke after accepting: connection must now be refused.
    host.trust().lock().await.revoke_key(&client.endpoint_id());

    let result = connect_direct(&client, &host).await;
    assert!(result.is_err(), "revoked client should be rejected");

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
