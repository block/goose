//! Minimal end-to-end example of the `goose-roaming` library API.
//!
//! It shows the whole surface a consumer touches — identity, bind, share,
//! mint an invite, dial it, and exchange bytes over the authorized stream —
//! without any dependency on goose's agent internals. The "agent" here is a
//! trivial echo server plugged in via the [`AcpStreamServer`] trait; a real
//! consumer would call goose's ACP `serve` instead (see `goose-cli`'s bridge).
//!
//! Run it:
//!
//! ```bash
//! cargo run -p goose-roaming --example echo_roundtrip
//! ```
//!
//! It runs both ends in one process over loopback (relays disabled), so it
//! needs no network. For a real two-machine test, use the `goose roam` CLI.

use std::sync::Arc;

use futures::io::{AsyncReadExt, AsyncWriteExt};
use goose_roaming::{
    AcpStreamServer, EndpointId, InviteOptions, RelaySettings, RoamingConfig, RoamingIdentity,
    RoamingNode, Scope, TrustBook, TrustPolicy,
};

const MSG: &[u8] = b"hello from the client";

/// A stand-in "agent": echoes back whatever the client sends, upper-cased.
struct EchoServer;

impl AcpStreamServer for EchoServer {
    fn serve_stream(
        &self,
        _client: EndpointId,
        _scope: Scope,
        mut recv: Box<dyn futures::io::AsyncRead + Send + Unpin>,
        mut send: Box<dyn futures::io::AsyncWrite + Send + Unpin>,
    ) -> futures::future::BoxFuture<'static, anyhow::Result<()>> {
        Box::pin(async move {
            // Read the client's fixed-size message, reply upper-cased, then stay
            // alive by draining until the client closes its send half. Returning
            // early would tear down the QUIC connection before delivery (a real
            // ACP `serve` runs a long-lived duplex loop, so this doesn't arise).
            let mut buf = [0u8; MSG.len()];
            recv.read_exact(&mut buf).await?;
            send.write_all(&buf.to_ascii_uppercase()).await?;
            send.flush().await?;
            let mut drain = Vec::new();
            let _ = recv.read_to_end(&mut drain).await;
            Ok(())
        })
    }

    fn agent_id(&self) -> String {
        "echo-agent".to_string()
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let loopback = "127.0.0.1:0".parse().unwrap();

    // --- Host side ---------------------------------------------------------
    // Bind a node with the lean config builder, then start sharing the agent.
    let host = RoamingNode::bind(
        RoamingConfig::new(RoamingIdentity::generate())
            .with_relay(RelaySettings::Disabled)
            .with_bind_addr(loopback),
    )
    .await?;
    host.share(Arc::new(EchoServer)).await?;

    // Mint a bearer invite valid for 60s.
    let invite = host.make_invite(InviteOptions::new(
        Scope::Control,
        std::time::Duration::from_secs(60),
    ));
    println!("host endpoint id : {}", host.endpoint_id());
    println!("invite token     : {}", invite.encode()?);

    // --- Client side -------------------------------------------------------
    // A separate node dials the host directly (relays disabled → LAN address).
    let client = RoamingNode::bind(
        RoamingConfig::new(RoamingIdentity::generate())
            .with_relay(RelaySettings::Disabled)
            .with_trust(TrustBook::new(TrustPolicy::Bearer))
            .with_bind_addr(loopback),
    )
    .await?;

    let stream = client
        .connect_with_addr(host.endpoint().addr(), &invite, Some("example".into()))
        .await?;
    println!(
        "connected to `{}` with scope {:?}",
        stream.agent_id(),
        stream.scope()
    );

    // Use the ergonomic helper: no manual tokio-compat dance.
    let (mut send, mut recv, _conn) = stream.into_futures_io();
    send.write_all(MSG).await?;
    send.flush().await?;
    let mut out = [0u8; MSG.len()];
    recv.read_exact(&mut out).await?;
    // Close our send half so the host's drain read completes cleanly.
    send.close().await?;
    let reply = String::from_utf8_lossy(&out);
    println!("agent replied    : {reply}");
    assert_eq!(out, MSG.to_ascii_uppercase().as_slice());

    host.shutdown().await?;
    client.shutdown().await?;
    println!("ok");
    Ok(())
}
