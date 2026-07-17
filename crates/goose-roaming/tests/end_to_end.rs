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
    TrustBook, TrustPolicy,
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

async fn bind_node(trust: TrustBook) -> Arc<RoamingNode> {
    RoamingNode::bind(RoamingConfig {
        identity: RoamingIdentity::generate(),
        relay: RelaySettings::Disabled,
        trust,
        directory: Directory::new(),
    })
    .await
    .expect("bind node")
}

#[tokio::test]
async fn bearer_invite_connects_and_streams() {
    let host_identity = RoamingIdentity::generate();
    let host = RoamingNode::bind(RoamingConfig {
        identity: host_identity.clone(),
        relay: RelaySettings::Disabled,
        trust: TrustBook::new(TrustPolicy::Bearer),
        directory: Directory::new(),
    })
    .await
    .expect("bind host");

    host.share(Arc::new(EchoServer)).await.expect("share");

    let invite = host.make_invite(
        &host_identity,
        &RelaySettings::Disabled,
        Scope::Control,
        vec![],
        60,
        false,
    );

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

#[tokio::test]
async fn allowlist_rejects_unknown_client() {
    let host_identity = RoamingIdentity::generate();
    let host = RoamingNode::bind(RoamingConfig {
        identity: host_identity.clone(),
        relay: RelaySettings::Disabled,
        // Allowlist policy with an empty allowlist: nobody is authorized.
        trust: TrustBook::new(TrustPolicy::Allowlist),
        directory: Directory::new(),
    })
    .await
    .expect("bind host");
    host.share(Arc::new(EchoServer)).await.expect("share");

    let invite = host.make_invite(
        &host_identity,
        &RelaySettings::Disabled,
        Scope::Control,
        vec![],
        60,
        false,
    );

    let client = bind_node(TrustBook::new(TrustPolicy::Bearer)).await;
    let result = connect_direct(&client, &host, &invite).await;
    assert!(result.is_err(), "unlisted client should be rejected");

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
