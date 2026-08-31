//! End-to-end test: two roaming nodes connect over iroh (direct, relays
//! disabled) and exchange bytes through an authorized ACP stream.
//!
//! This validates the whole seam: bind -> swap identities -> accept key -> dial
//! -> handshake -> authorize -> stream hand-off. It uses a trivial echo "ACP
//! server" in place of goose's real ACP protocol, since this crate has no
//! dependency on the agent machinery.

use std::sync::Arc;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use goose_roaming::{
    AcpStreamServer, Directory, RelaySettings, RevocableWork, RevocationSignal, RoamingConfig,
    RoamingIdentity, RoamingNode, TrustBook,
};
use iroh::EndpointId;

/// A stand-in for goose's detached prompt runs: records revocations so tests
/// can assert that revoking a peer reaches work that outlived its connection.
#[derive(Debug)]
struct FakeDetachedWork {
    alive: AtomicBool,
    revocations: AtomicUsize,
}

impl FakeDetachedWork {
    fn new(alive: bool) -> Arc<Self> {
        Arc::new(Self {
            alive: AtomicBool::new(alive),
            revocations: AtomicUsize::new(0),
        })
    }

    fn revocations(&self) -> usize {
        self.revocations.load(Ordering::Acquire)
    }
}

impl RevocableWork for FakeDetachedWork {
    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }

    fn revoke(&self) -> futures::future::BoxFuture<'static, ()> {
        self.revocations.fetch_add(1, Ordering::AcqRel);
        Box::pin(async {})
    }
}

/// A stand-in ACP server that echoes one line back, upper-cased.
#[derive(Debug, Default)]
struct EchoServer {
    /// Revocation signals handed to `serve_stream`, so tests can assert
    /// whether the node marked a connection as force-closed-by-revocation
    /// (versus an ordinary transport close).
    revocations: Arc<std::sync::Mutex<Vec<RevocationSignal>>>,
    /// When set, registered with the connection's revocation handle at serve
    /// start — as the real bridge parks its detached-run handle — so tests can
    /// assert peer-keyed revocation of work that outlives the connection.
    work_to_register: std::sync::Mutex<Option<Arc<FakeDetachedWork>>>,
}

impl AcpStreamServer for EchoServer {
    fn serve_stream(
        &self,
        _client: EndpointId,
        mut recv: Box<dyn AsyncRead + Send + Unpin>,
        mut send: Box<dyn AsyncWrite + Send + Unpin>,
        revocation: RevocationSignal,
    ) -> futures::future::BoxFuture<'static, anyhow::Result<()>> {
        self.revocations.lock().unwrap().push(revocation.clone());
        let work = self.work_to_register.lock().unwrap().take();
        Box::pin(async move {
            if let Some(work) = work {
                revocation.register_revocable_work(work).await;
            }
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
        relay_tls: None,
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
        relay_tls: None,
    })
    .await
    .expect("bind host");
    host.share(Arc::new(EchoServer::default()))
        .await
        .expect("share");

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
    let echo = Arc::new(EchoServer::default());
    host.share(echo.clone()).await.expect("share");

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

    // An ordinary close is not a revocation: work that survives transport
    // loss keys off this signal.
    {
        let signals = echo.revocations.lock().unwrap();
        assert_eq!(signals.len(), 1);
        assert!(
            !signals[0].is_revoked(),
            "a clean client close must not be marked as a revocation"
        );
    }

    host.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unaccepted_key_is_rejected() {
    let host = bind_node().await;
    host.share(Arc::new(EchoServer::default()))
        .await
        .expect("share");

    // Client's key was never accepted: connection must be refused.
    let client = bind_node().await;
    let result = connect_direct(&client, &host).await;
    assert!(result.is_err(), "unaccepted client should be rejected");

    host.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoked_key_is_rejected() {
    let host = bind_node().await;
    host.share(Arc::new(EchoServer::default()))
        .await
        .expect("share");

    let client = bind_node().await;
    host_accepts(&host, &client).await;
    // Revoke after accepting: connection must now be refused.
    host.trust().lock().await.revoke_key(&client.endpoint_id());

    let result = connect_direct(&client, &host).await;
    assert!(result.is_err(), "revoked client should be rejected");

    host.shutdown().await.unwrap();
}

/// Revocation reaches into the open data plane: revoking a key while its
/// connection is live force-closes that connection — the peer cannot keep
/// using a capability it no longer holds. (The allowlist gating only new
/// dials is not enough; see #10906.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revocation_closes_live_connection() {
    let host = bind_node().await;
    let echo = Arc::new(EchoServer::default());
    host.share(echo.clone()).await.expect("share");

    let client = bind_node().await;
    host_accepts(&host, &client).await;

    let mut stream = connect_direct(&client, &host)
        .await
        .expect("client connects while accepted");

    // Prove the duplex is live mid-"prompt".
    stream.send.write_all(b"hello").await.unwrap();
    let mut out = [0u8; 5];
    stream.recv.read_exact(&mut out).await.unwrap();
    assert_eq!(&out, b"HELLO");

    // Revoke while the connection is open, then enforce.
    let trust = host.trust();
    let book = {
        let mut trust = trust.lock().await;
        trust.revoke_key(&client.endpoint_id());
        trust.clone()
    };
    let closed = host.enforce_trust(&book).await;
    assert_eq!(closed, 1, "the live connection should be force-closed");

    // The tab-side stream dies: the next read fails rather than hanging.
    let mut more = [0u8; 1];
    let read = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        stream.recv.read_exact(&mut more),
    )
    .await;
    assert!(
        matches!(read, Ok(Err(_))),
        "read after revocation should fail, got {read:?}"
    );

    // The stream server can tell this close was a revocation, so it can stop
    // work that would otherwise survive an ordinary transport loss.
    {
        let signals = echo.revocations.lock().unwrap();
        assert_eq!(signals.len(), 1);
        assert!(
            signals[0].is_revoked(),
            "a force-close for a revoked key must set the revocation signal"
        );
    }

    // And the next dial is refused.
    assert!(
        connect_direct(&client, &host).await.is_err(),
        "revoked client must not reconnect"
    );

    host.shutdown().await.unwrap();
}

/// The end-to-end shape of `roam peers revoke` against a running share: the
/// trust *file* changes out of band, the watcher notices, and the live
/// connection is closed without a restart.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revocation_watcher_closes_live_connection_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let trust_file = dir.path().join("trust.json");

    let client = bind_node().await;

    let mut book = TrustBook::new();
    book.accept(&client.endpoint_id());
    book.save(&trust_file).unwrap();

    let host = RoamingNode::bind(RoamingConfig {
        identity: RoamingIdentity::generate(),
        relay: RelaySettings::Disabled,
        trust: TrustBook::new(),
        trust_path: Some(trust_file.clone()),
        directory: Directory::new(),
        bind_addr: Some(loopback()),
        relay_tls: None,
    })
    .await
    .expect("bind host");
    let echo = Arc::new(EchoServer::default());
    host.share(echo.clone()).await.expect("share");
    host.watch_revocations(std::time::Duration::from_millis(100))
        .await;

    let mut stream = connect_direct(&client, &host)
        .await
        .expect("client connects while accepted");
    stream.send.write_all(b"hello").await.unwrap();
    let mut out = [0u8; 5];
    stream.recv.read_exact(&mut out).await.unwrap();

    // Revoke out of band by rewriting the trust file (as the CLI does).
    let mut book = TrustBook::load(&trust_file).unwrap();
    book.revoke_key(&client.endpoint_id());
    book.save(&trust_file).unwrap();

    // The watcher should notice and kill the live connection.
    let mut more = [0u8; 1];
    let read = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.recv.read_exact(&mut more),
    )
    .await;
    assert!(
        matches!(read, Ok(Err(_))),
        "watcher should force-close the live connection, got {read:?}"
    );

    // The watcher's force-close is a revocation, and the stream server must
    // be able to see that.
    {
        let signals = echo.revocations.lock().unwrap();
        assert_eq!(signals.len(), 1);
        assert!(
            signals[0].is_revoked(),
            "the watcher's force-close must set the revocation signal"
        );
    }

    host.shutdown().await.unwrap();
}

/// Detached work must stay revocable after its connection ends: an authorized
/// peer that disconnects NORMALLY mid-run leaves no live connection to
/// force-close, yet a later `peers revoke` must still stop the work it left
/// behind.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoking_a_disconnected_peer_stops_its_detached_work() {
    let host = bind_node().await;
    let echo = Arc::new(EchoServer::default());
    let work = FakeDetachedWork::new(true);
    *echo.work_to_register.lock().unwrap() = Some(work.clone());
    host.share(echo.clone()).await.expect("share");

    let client = bind_node().await;
    host_accepts(&host, &client).await;

    let mut stream = connect_direct(&client, &host)
        .await
        .expect("client connects while accepted");
    stream.send.write_all(b"hello").await.unwrap();
    let mut out = [0u8; 5];
    stream.recv.read_exact(&mut out).await.unwrap();
    assert_eq!(&out, b"HELLO");

    // Disconnect normally: close the client's send half and read to EOF so
    // the host's serve future finishes and the live connection unregisters.
    stream.send.finish().unwrap();
    let mut more = [0u8; 1];
    let _ = stream.recv.read_exact(&mut more).await;
    drop(stream);
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert_eq!(
        work.revocations(),
        0,
        "an ordinary disconnect must not revoke the peer's detached work"
    );

    // Revoke the peer AFTER it disconnected.
    let trust = host.trust();
    let book = {
        let mut trust = trust.lock().await;
        trust.revoke_key(&client.endpoint_id());
        trust.clone()
    };
    let closed = host.enforce_trust(&book).await;

    assert_eq!(closed, 0, "no live connection should remain to force-close");
    assert_eq!(
        work.revocations(),
        1,
        "revocation must reach detached work by peer key when no connection is live"
    );

    host.shutdown().await.unwrap();
}

/// Registry mechanics for detached work: revocation is peer-keyed, finished
/// work is pruned before it can see a revocation, and entries drain on
/// revocation so a later re-accepted peer starts from a clean slate.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revocation_registry_is_peer_keyed_and_prunes_dead_work() {
    let host = bind_node().await;
    let peer_a = bind_node().await.endpoint_id();
    let peer_b = bind_node().await.endpoint_id();
    // Work is only parked for peers that currently hold authority.
    {
        let trust = host.trust();
        let mut trust = trust.lock().await;
        trust.accept(&peer_a);
        trust.accept(&peer_b);
    }

    // A handle whose work already finished is pruned when the same peer
    // registers again, so it never sees a later revocation.
    let finished = FakeDetachedWork::new(false);
    host.register_revocable_work(peer_a, finished.clone()).await;
    let running_a = FakeDetachedWork::new(true);
    host.register_revocable_work(peer_a, running_a.clone())
        .await;
    let running_b = FakeDetachedWork::new(true);
    host.register_revocable_work(peer_b, running_b.clone())
        .await;

    // Revoke A only (B stays accepted).
    let mut book = TrustBook::new();
    book.accept(&peer_b);
    host.enforce_trust(&book).await;

    assert_eq!(
        running_a.revocations(),
        1,
        "the revoked peer's running work must be stopped"
    );
    assert_eq!(
        finished.revocations(),
        0,
        "finished work was pruned before the revocation"
    );
    assert_eq!(
        running_b.revocations(),
        0,
        "another peer's work must be untouched"
    );

    // Entries drain on revocation: enforcing again revokes nothing new, so a
    // peer that is later re-accepted starts from a clean slate.
    host.enforce_trust(&book).await;
    assert_eq!(running_a.revocations(), 1);

    host.shutdown().await.unwrap();
}

/// Registration and revocation are mutually exclusive: once a peer has been
/// revoked, work registered afterwards is revoked instead of parked. This is
/// the interleaving the peer-keyed registry exists to close — a worker that
/// registers *after* the revocation drain must not stay registered and
/// unrevoked — pinned in the deterministic order (drain first, then register)
/// rather than by timing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoked_peer_cannot_park_work_after_the_revocation_drain() {
    let host = bind_node().await;
    let peer = bind_node().await.endpoint_id();
    host.trust().lock().await.accept(&peer);

    // While trusted, work registers normally and a later revocation stops it.
    let before = FakeDetachedWork::new(true);
    host.register_revocable_work(peer, before.clone()).await;

    let revoked_book = {
        let trust = host.trust();
        let mut trust = trust.lock().await;
        trust.revoke_key(&peer);
        trust.clone()
    };
    host.enforce_trust(&revoked_book).await;
    assert_eq!(
        before.revocations(),
        1,
        "work parked while trusted must be revoked by the drain"
    );

    // The drain has run. A registration arriving now — the race window — must
    // be revoked by the registration path itself, not parked.
    let after = FakeDetachedWork::new(true);
    host.register_revocable_work(peer, after.clone()).await;
    assert_eq!(
        after.revocations(),
        1,
        "work registered for a revoked peer must be revoked, not parked"
    );

    // And it must not be left in the registry: if it had been parked, this
    // second enforcement pass would revoke it a second time.
    host.enforce_trust(&revoked_book).await;
    assert_eq!(
        after.revocations(),
        1,
        "rejected work must not be left registered under the revoked key"
    );
    assert_eq!(before.revocations(), 1, "revocation must stay idempotent");

    host.shutdown().await.unwrap();
}

/// A peer that is re-accepted registers cleanly again: rejecting work for a
/// revoked key must not fence the key permanently.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn re_accepted_peer_registers_work_again() {
    let host = bind_node().await;
    let peer = bind_node().await.endpoint_id();

    // Revoked: refused.
    let revoked_book = {
        let trust = host.trust();
        let mut trust = trust.lock().await;
        trust.revoke_key(&peer);
        trust.clone()
    };
    let refused = FakeDetachedWork::new(true);
    host.register_revocable_work(peer, refused.clone()).await;
    assert_eq!(refused.revocations(), 1);

    // Re-accepted out of band: the next connection's work parks normally, with
    // no leftover state from the revoked era.
    let allowed_book = {
        let trust = host.trust();
        let mut trust = trust.lock().await;
        trust.accept(&peer);
        trust.clone()
    };
    let fresh = FakeDetachedWork::new(true);
    host.register_revocable_work(peer, fresh.clone()).await;
    assert_eq!(
        fresh.revocations(),
        0,
        "a re-accepted peer's work must be parked, not revoked"
    );

    // Enforcing the allowing book keeps it parked; revoking again reaches it.
    host.enforce_trust(&allowed_book).await;
    assert_eq!(fresh.revocations(), 0);
    host.enforce_trust(&revoked_book).await;
    assert_eq!(
        fresh.revocations(),
        1,
        "the re-registered work must be revocable by peer key"
    );

    host.shutdown().await.unwrap();
}

/// The same invariant under a genuinely concurrent registration/enforcement:
/// whichever order the two critical sections happen to take, the work of a
/// revoked peer is revoked exactly once and nothing stays parked. Serializing
/// the decision with the drain is what makes both orders safe — admitted before
/// the drain (the drain revokes it) or after it (the registration revokes it).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_registration_and_revocation_revoke_exactly_once() {
    for _ in 0..32 {
        let host = bind_node().await;
        let peer = bind_node().await.endpoint_id();
        host.trust().lock().await.accept(&peer);

        let revoked_book = {
            let trust = host.trust();
            let mut trust = trust.lock().await;
            trust.revoke_key(&peer);
            trust.clone()
        };

        let work = FakeDetachedWork::new(true);
        let registrar = {
            let host = host.clone();
            let work = work.clone();
            tokio::spawn(async move { host.register_revocable_work(peer, work).await })
        };
        let enforcer = {
            let host = host.clone();
            let book = revoked_book.clone();
            tokio::spawn(async move { host.enforce_trust(&book).await })
        };
        registrar.await.unwrap();
        enforcer.await.unwrap();

        assert_eq!(
            work.revocations(),
            1,
            "a revoked peer's work must be revoked exactly once, whichever \
             order registration and enforcement ran in"
        );

        // Nothing may remain parked under the revoked key.
        host.enforce_trust(&revoked_book).await;
        assert_eq!(
            work.revocations(),
            1,
            "no work may be left registered under the revoked key"
        );

        host.shutdown().await.unwrap();
    }
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
