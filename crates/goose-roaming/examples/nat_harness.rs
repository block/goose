//! Multi-process harness binary for the real-NAT soak (`tests/nat-harness/`).
//!
//! `path_upgrade.rs` pins the relay→direct upgrade on localhost, where every
//! candidate address is directly reachable. This binary is the field version:
//! run as three roles in separate network namespaces (a relay on the "WAN", a
//! host and a client on isolated subnets behind one NAT router), it measures
//! what actually happens across a real NAT — sustained-traffic integrity on
//! the relay path, upgrade behavior when the router does/doesn't hairpin,
//! cold-connect latency, dial bursts, and recovery from a SIGKILLed host.
//!
//! Roles:
//!
//! ```bash
//! nat_harness relay --bind 0.0.0.0:3340
//! nat_harness host --shared /shared --relay-url http://10.99.0.10:3340
//! nat_harness client --shared /shared --relay-url http://10.99.0.10:3340 \
//!     --scenario soak|burst|cold|crash [--frames N] [--duration-secs N] [--dials N]
//! ```
//!
//! The card/trust bootstrap rides a shared volume: the host writes its
//! connection card, clients drop their endpoint id, the host accepts every id
//! it sees (the harness models the QR/paste exchange, not the approval UX).
//! Machine-readable results go to stdout as single `RESULT {json}` lines.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use goose_roaming::{
    parse_endpoint_id, AcpStreamServer, ConnectionCard, EndpointId, RelayEntry, RelaySettings,
    RoamingClientStream, RoamingConfig, RoamingError, RoamingIdentity, RoamingNode, TrustBook,
};
use iroh::endpoint::PathEvent;
use iroh::TransportAddr;

/// Both peers bind this UDP port so the router's NAT rules can map them
/// deterministically (each peer is alone in its namespace).
const QUIC_PORT: u16 = 7777;

const FRAME_LEN: usize = 14;
const ECHO_TIMEOUT: Duration = Duration::from_secs(10);

fn main() -> anyhow::Result<()> {
    let args = Args::parse(std::env::args().skip(1))?;
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?
        .block_on(async move {
            match args.role.as_str() {
                "relay" => run_relay(&args).await,
                "host" => run_host(&args).await,
                "client" => run_client(&args).await,
                other => anyhow::bail!("unknown role `{other}` (relay|host|client)"),
            }
        })
}

struct Args {
    role: String,
    shared: PathBuf,
    relay_url: String,
    bind: String,
    scenario: String,
    frames: usize,
    duration_secs: u64,
    dials: usize,
    pace_ms: u64,
}

impl Args {
    fn parse(mut argv: impl Iterator<Item = String>) -> anyhow::Result<Self> {
        let role = argv.next().unwrap_or_default();
        let mut args = Self {
            role,
            shared: PathBuf::from("/shared"),
            relay_url: String::new(),
            bind: "0.0.0.0:3340".to_string(),
            scenario: "soak".to_string(),
            frames: 300,
            duration_secs: 90,
            dials: 8,
            pace_ms: 20,
        };
        while let Some(flag) = argv.next() {
            let mut value = || {
                argv.next()
                    .ok_or_else(|| anyhow::anyhow!("missing value for {flag}"))
            };
            match flag.as_str() {
                "--shared" => args.shared = PathBuf::from(value()?),
                "--relay-url" => args.relay_url = value()?,
                "--bind" => args.bind = value()?,
                "--scenario" => args.scenario = value()?,
                "--frames" => args.frames = value()?.parse()?,
                "--duration-secs" => args.duration_secs = value()?.parse()?,
                "--dials" => args.dials = value()?.parse()?,
                "--pace-ms" => args.pace_ms = value()?.parse()?,
                other => anyhow::bail!("unknown flag `{other}`"),
            }
        }
        Ok(args)
    }
}

/// Plain-HTTP relay (WebSocket transport, no TLS) bound publicly so the
/// peer namespaces can reach it — `iroh::test_utils::run_relay_server` binds
/// loopback-only, which is exactly what this harness must not do.
async fn run_relay(args: &Args) -> anyhow::Result<()> {
    let mut relay =
        iroh_relay::server::RelayConfig::new(args.bind.parse::<std::net::SocketAddr>()?);
    relay.key_cache_capacity = Some(1024);
    let mut config = iroh_relay::server::ServerConfig::default();
    config.relay = Some(relay);
    let server = iroh_relay::server::Server::spawn(config).await?;
    println!(
        "RELAY_READY {}",
        server
            .http_addr()
            .map(|a| a.to_string())
            .unwrap_or_default()
    );
    std::future::pending::<()>().await;
    unreachable!()
}

/// Echoes bytes until the client closes — same duplex load shape as
/// `path_upgrade.rs`, so a mid-stream path migration happens under traffic.
#[derive(Debug)]
struct StreamingEchoServer;

impl AcpStreamServer for StreamingEchoServer {
    fn serve_stream(
        &self,
        _client: EndpointId,
        mut recv: Box<dyn AsyncRead + Send + Unpin>,
        mut send: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> futures::future::BoxFuture<'static, anyhow::Result<()>> {
        Box::pin(async move {
            let mut buf = [0u8; 4096];
            loop {
                let n = recv.read(&mut buf).await?;
                if n == 0 {
                    return Ok(());
                }
                send.write_all(&buf[..n]).await?;
                send.flush().await?;
            }
        })
    }

    fn agent_id(&self) -> String {
        "nat-harness-echo".to_string()
    }
}

async fn run_host(args: &Args) -> anyhow::Result<()> {
    anyhow::ensure!(!args.relay_url.is_empty(), "host needs --relay-url");
    // Persisted identity: a SIGKILL + restart must come back as the same key
    // so the client's card and the accepted trust survive the crash.
    let identity = RoamingIdentity::load_or_create(&args.shared.join("host.key"))?;
    let node = bind_node(identity, &args.relay_url).await?;
    node.share(Arc::new(StreamingEchoServer)).await?;
    anyhow::ensure!(
        node.wait_online(Duration::from_secs(30)).await,
        "host never reached the relay"
    );
    atomic_write(
        &args.shared.join("host.card"),
        node.card().encode()?.as_bytes(),
    )?;
    println!("HOST_READY {}", node.endpoint_id());

    loop {
        accept_pending_clients(&node, &args.shared).await?;
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Accept every client id dropped in the shared dir. The harness models the
/// out-of-band card/id exchange; approval UX is out of scope here.
async fn accept_pending_clients(node: &Arc<RoamingNode>, shared: &Path) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(shared)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !(name.starts_with("client-") && name.ends_with(".id")) {
            continue;
        }
        let id = parse_endpoint_id(std::fs::read_to_string(&path)?.trim())?;
        let trust_book = node.trust();
        let mut trust = trust_book.lock().await;
        if !trust.is_allowed(&id) {
            trust.accept(&id);
            println!("HOST_ACCEPTED {id}");
        }
    }
    Ok(())
}

async fn run_client(args: &Args) -> anyhow::Result<()> {
    anyhow::ensure!(!args.relay_url.is_empty(), "client needs --relay-url");
    let t0 = Instant::now();
    let node = bind_node(RoamingIdentity::generate(), &args.relay_url).await?;
    atomic_write(
        &args
            .shared
            .join(format!("client-{}.id", std::process::id())),
        node.endpoint_id().to_string().as_bytes(),
    )?;
    anyhow::ensure!(
        node.wait_online(Duration::from_secs(30)).await,
        "client never reached the relay"
    );
    let online_ms = t0.elapsed().as_millis() as u64;
    let card = wait_for_card(&args.shared).await?;

    match args.scenario.as_str() {
        "soak" => scenario_soak(&node, &card, args).await,
        "burst" => scenario_burst(&node, &card, args).await,
        "cold" => scenario_cold(&node, &card, online_ms).await,
        "crash" => scenario_crash(&node, &card, args).await,
        other => anyhow::bail!("unknown scenario `{other}`"),
    }
}

async fn bind_node(identity: RoamingIdentity, relay_url: &str) -> anyhow::Result<Arc<RoamingNode>> {
    let mut config = RoamingConfig::new(identity)
        .with_relay(RelaySettings::Custom(vec![RelayEntry::new(relay_url)]))
        .with_bind_addr(std::net::SocketAddr::from(([0, 0, 0, 0], QUIC_PORT)));
    config.trust = TrustBook::new();
    Ok(RoamingNode::bind(config).await?)
}

async fn wait_for_card(shared: &Path) -> anyhow::Result<ConnectionCard> {
    let deadline = Instant::now() + Duration::from_secs(60);
    let path = shared.join("host.card");
    loop {
        if let Ok(text) = std::fs::read_to_string(&path) {
            return Ok(ConnectionCard::decode(text.trim())?);
        }
        anyhow::ensure!(Instant::now() < deadline, "host.card never appeared");
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Dial until accepted. Freshly-dropped ids race the host's accept poll, and
/// during the crash scenario the host is down entirely, so both rejection and
/// transport errors retry until the deadline.
async fn connect_trusted(
    node: &RoamingNode,
    card: &ConnectionCard,
    label: &str,
    deadline: Duration,
) -> anyhow::Result<RoamingClientStream> {
    let t0 = Instant::now();
    loop {
        match node.connect(card, Some(label.to_string())).await {
            Ok(stream) => return Ok(stream),
            Err(RoamingError::Rejected(_)) => tokio::time::sleep(Duration::from_millis(300)).await,
            Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
        }
        anyhow::ensure!(
            t0.elapsed() < deadline,
            "could not establish an accepted connection within {deadline:?}"
        );
    }
}

/// Record every path event on the connection; flag whether a direct (IP) path
/// was ever selected. On a NAT where hairpin can't work — or while reflexive
/// address discovery isn't wired — the flag staying false IS the measurement.
struct PathWatch {
    events: Arc<std::sync::Mutex<Vec<(u64, String)>>>,
    direct_selected: Arc<std::sync::atomic::AtomicBool>,
    task: tokio::task::JoinHandle<()>,
}

impl PathWatch {
    fn start(conn: &iroh::endpoint::Connection, t0: Instant) -> Self {
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let direct_selected = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let task = watch_paths(conn, t0, events.clone(), direct_selected.clone(), 0);
        Self {
            events,
            direct_selected,
            task,
        }
    }

    /// Stop watching and return the timeline. The flag is finalized from the
    /// connection's current path set: a selection that landed after the last
    /// processed event must not be underreported.
    fn finish(self, conn: &iroh::endpoint::Connection) -> (Vec<(u64, String)>, bool) {
        self.task.abort();
        seed_direct_from_paths(conn, &self.direct_selected);
        let events = self.events.lock().unwrap().clone();
        let direct = self
            .direct_selected
            .load(std::sync::atomic::Ordering::Relaxed);
        (events, direct)
    }
}

/// Set the direct flag if the current path set already holds a selected
/// direct (IP) path. Run when subscribing (an upgrade completed during
/// connect/handshake emits no later event) and at shutdown (one completed
/// after the last processed event).
fn seed_direct_from_paths(
    conn: &iroh::endpoint::Connection,
    direct_selected: &std::sync::atomic::AtomicBool,
) {
    for path in conn.paths().iter() {
        if path.is_selected() && matches!(path.remote_addr(), TransportAddr::Ip(_)) {
            direct_selected.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// Stream one connection's path events into shared storage. `seq` labels
/// events per connection so a reconnecting scenario yields one legible
/// timeline across all the connections it went through.
///
/// The current path set is recorded (and the direct flag seeded) before the
/// event subscription: an upgrade that completed during connect/handshake
/// never emits a later event, so subscribing alone would miss it.
fn watch_paths(
    conn: &iroh::endpoint::Connection,
    t0: Instant,
    events: Arc<std::sync::Mutex<Vec<(u64, String)>>>,
    direct_selected: Arc<std::sync::atomic::AtomicBool>,
    seq: usize,
) -> tokio::task::JoinHandle<()> {
    let mut stream = conn.path_events();
    seed_direct_from_paths(conn, &direct_selected);
    events.lock().unwrap().push((
        t0.elapsed().as_millis() as u64,
        format!("c{seq} subscribed paths={:?}", paths_snapshot(conn)),
    ));
    tokio::spawn(async move {
        while let Some(event) = futures::StreamExt::next(&mut stream).await {
            if matches!(
                event,
                PathEvent::Selected {
                    remote_addr: TransportAddr::Ip(_),
                    ..
                }
            ) {
                direct_selected.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            events
                .lock()
                .unwrap()
                .push((t0.elapsed().as_millis() as u64, format!("c{seq} {event:?}")));
        }
    })
}

fn paths_snapshot(conn: &iroh::endpoint::Connection) -> Vec<String> {
    conn.paths()
        .iter()
        .map(|p| {
            let selected = if p.is_selected() { " selected" } else { "" };
            format!("{:?}{selected}", p.remote_addr())
        })
        .collect()
}

async fn exchange_frame(
    stream: &mut RoamingClientStream,
    counter: u64,
) -> anyhow::Result<Duration> {
    let msg = format!("frame-{counter:08}");
    debug_assert_eq!(msg.len(), FRAME_LEN);
    let t0 = Instant::now();
    stream.send.write_all(msg.as_bytes()).await?;
    let mut buf = [0u8; FRAME_LEN];
    tokio::time::timeout(ECHO_TIMEOUT, stream.recv.read_exact(&mut buf))
        .await
        .map_err(|_| anyhow::anyhow!("echo timed out at frame {counter}"))??;
    anyhow::ensure!(
        buf == msg.as_bytes(),
        "frame {counter} corrupted (got {:?})",
        String::from_utf8_lossy(&buf)
    );
    Ok(t0.elapsed())
}

fn latency_stats(samples: &mut [u64]) -> serde_json::Value {
    if samples.is_empty() {
        return serde_json::json!(null);
    }
    samples.sort_unstable();
    let at = |q: f64| samples[((samples.len() - 1) as f64 * q) as usize];
    serde_json::json!({
        "min_ms": samples[0],
        "p50_ms": at(0.50),
        "p90_ms": at(0.90),
        "p99_ms": at(0.99),
        "max_ms": samples[samples.len() - 1],
    })
}

/// Sustained numbered frames over one connection. Any dropped, reordered, or
/// corrupted frame fails loudly with its number — the field failure this
/// harness exists to catch was exactly "relay works, then queued messages
/// vanish silently".
async fn scenario_soak(
    node: &Arc<RoamingNode>,
    card: &ConnectionCard,
    args: &Args,
) -> anyhow::Result<()> {
    let t0 = Instant::now();
    let mut stream = connect_trusted(node, card, "soak", Duration::from_secs(60)).await?;
    let connect_ms = t0.elapsed().as_millis() as u64;
    let watch = PathWatch::start(&stream.conn, t0);

    let mut latencies = Vec::with_capacity(args.frames);
    for counter in 0..args.frames as u64 {
        let rtt = exchange_frame(&mut stream, counter).await?;
        latencies.push(rtt.as_millis() as u64);
        tokio::time::sleep(Duration::from_millis(args.pace_ms)).await;
    }

    let paths = paths_snapshot(&stream.conn);
    let (events, direct) = watch.finish(&stream.conn);
    println!(
        "RESULT {}",
        serde_json::json!({
            "scenario": "soak",
            "connect_ms": connect_ms,
            "frames_ok": latencies.len(),
            "frames_requested": args.frames,
            "latency": latency_stats(&mut latencies),
            "direct_path_selected": direct,
            "paths_final": paths,
            "path_events": events.iter().map(|(ms, e)| format!("{ms}ms {e}")).collect::<Vec<_>>(),
            "duration_ms": t0.elapsed().as_millis() as u64,
        })
    );
    stream.send.finish()?;
    Ok(())
}

/// Parallel dials to one host across the NAT — the field report that motivated
/// `concurrent_dial_burst`, now with every packet actually traversing a router.
async fn scenario_burst(
    node: &Arc<RoamingNode>,
    card: &ConnectionCard,
    args: &Args,
) -> anyhow::Result<()> {
    connect_trusted(node, card, "burst-warm", Duration::from_secs(60))
        .await?
        .send
        .finish()?;

    let t0 = Instant::now();
    let mut tasks = Vec::new();
    for i in 0..args.dials as u64 {
        let node = node.clone();
        let card = card.clone();
        tasks.push(tokio::spawn(async move {
            let dial_t0 = Instant::now();
            let mut stream = node.connect(&card, Some(format!("burst-{i}"))).await?;
            let connect_ms = dial_t0.elapsed().as_millis() as u64;
            for round in 0..3u64 {
                exchange_frame(&mut stream, i * 1000 + round).await?;
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            stream.send.finish()?;
            Ok::<_, anyhow::Error>(connect_ms)
        }));
    }
    let mut connects = Vec::new();
    let mut failures = Vec::new();
    for (i, task) in tasks.into_iter().enumerate() {
        match task.await? {
            Ok(ms) => connects.push(ms),
            Err(e) => failures.push(format!("dial {i}: {e}")),
        }
    }
    println!(
        "RESULT {}",
        serde_json::json!({
            "scenario": "burst",
            "dials": args.dials,
            "ok": connects.len(),
            "failures": failures,
            "connect": latency_stats(&mut connects),
            "duration_ms": t0.elapsed().as_millis() as u64,
        })
    );
    anyhow::ensure!(failures.is_empty(), "burst had failures");
    Ok(())
}

/// One fresh-process dial (the run script loops this): endpoint state is
/// empty, so this measures the true cold path — relay registration, first
/// dial, first frame.
async fn scenario_cold(
    node: &Arc<RoamingNode>,
    card: &ConnectionCard,
    online_ms: u64,
) -> anyhow::Result<()> {
    let t0 = Instant::now();
    let mut stream = connect_trusted(node, card, "cold", Duration::from_secs(60)).await?;
    let connect_ms = t0.elapsed().as_millis() as u64;
    let rtt = exchange_frame(&mut stream, 0).await?;
    println!(
        "RESULT {}",
        serde_json::json!({
            "scenario": "cold",
            "online_ms": online_ms,
            "connect_ms": connect_ms,
            "first_frame_ms": rtt.as_millis() as u64,
        })
    );
    stream.send.finish()?;
    Ok(())
}

/// Soak that survives a host SIGKILL: on stream failure, reconnect in a loop
/// and record the outage. The run script kills and restarts the host while
/// this runs; the measurement is how long until frames flow again.
async fn scenario_crash(
    node: &Arc<RoamingNode>,
    card: &ConnectionCard,
    args: &Args,
) -> anyhow::Result<()> {
    let t0 = Instant::now();
    let mut stream = connect_trusted(node, card, "crash", Duration::from_secs(60)).await?;
    let mut counter: u64 = 0;
    let mut frames_ok: u64 = 0;
    let mut outages: Vec<(u64, u64)> = Vec::new();

    // One shared path timeline across every connection the scenario goes
    // through: which path was in use when the victim died, and which path the
    // recovered connection landed on, are part of the result.
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let direct_selected = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut seq = 0usize;
    let mut watch = watch_paths(
        &stream.conn,
        t0,
        events.clone(),
        direct_selected.clone(),
        seq,
    );

    // Prime with one frame so the data plane is provably live, then announce
    // it: the run script anchors its kill timer on this marker, so a slow
    // setup can't let the kill land before anything is under test.
    exchange_frame(&mut stream, counter).await?;
    frames_ok += 1;
    counter += 1;
    println!("CRASH_RUNNING");
    // The measured window starts here — the same instant the run script
    // anchors its kill timer on — so setup time can't eat the window and end
    // the loop before the kill ever lands.
    let end = Instant::now() + Duration::from_secs(args.duration_secs);

    let reconnect = |stream: &mut RoamingClientStream,
                     watch: &mut tokio::task::JoinHandle<()>,
                     seq: &mut usize| {
        watch.abort();
        *seq += 1;
        *watch = watch_paths(
            &stream.conn,
            t0,
            events.clone(),
            direct_selected.clone(),
            *seq,
        );
    };

    while Instant::now() < end {
        // A SIGKILLed remote surfaces no QUIC error — the frame just times
        // out — so the user-visible blackout starts when the data plane last
        // worked, at the top of the failed attempt, not after the echo
        // timeout has already burned.
        let attempt_start_ms = t0.elapsed().as_millis() as u64;
        match exchange_frame(&mut stream, counter).await {
            Ok(_) => {
                frames_ok += 1;
                counter += 1;
                tokio::time::sleep(Duration::from_millis(args.pace_ms)).await;
            }
            Err(_) => {
                let outage_start = attempt_start_ms;
                // Bound the whole recovery: reconnects that keep being
                // accepted while every frame still dies must fail the
                // scenario, not hang the battery.
                let recovery_deadline = Instant::now() + Duration::from_secs(120);
                drop(stream);
                stream = connect_trusted(node, card, "crash", Duration::from_secs(120)).await?;
                reconnect(&mut stream, &mut watch, &mut seq);
                // Confirm the link actually carries data again before closing
                // the outage window — an accepted dial with a dead data plane
                // must keep counting as downtime.
                counter += 1;
                loop {
                    match exchange_frame(&mut stream, counter).await {
                        Ok(_) => break,
                        Err(_) => {
                            anyhow::ensure!(
                                Instant::now() < recovery_deadline,
                                "data plane did not recover within 120s of outage start"
                            );
                            drop(stream);
                            stream = connect_trusted(node, card, "crash", Duration::from_secs(120))
                                .await?;
                            reconnect(&mut stream, &mut watch, &mut seq);
                        }
                    }
                }
                frames_ok += 1;
                counter += 1;
                outages.push((outage_start, t0.elapsed().as_millis() as u64));
            }
        }
    }

    watch.abort();
    seed_direct_from_paths(&stream.conn, &direct_selected);
    let path_events = events.lock().unwrap().clone();
    println!(
        "RESULT {}",
        serde_json::json!({
            "scenario": "crash",
            "frames_ok": frames_ok,
            "connections": seq + 1,
            "outages": outages
                .iter()
                .map(|(start, end)| serde_json::json!({
                    "start_ms": start,
                    "end_ms": end,
                    "outage_ms": end - start,
                }))
                .collect::<Vec<_>>(),
            "direct_path_selected": direct_selected.load(std::sync::atomic::Ordering::Relaxed),
            "paths_final": paths_snapshot(&stream.conn),
            "path_events": path_events.iter().map(|(ms, e)| format!("{ms}ms {e}")).collect::<Vec<_>>(),
            "duration_ms": t0.elapsed().as_millis() as u64,
        })
    );
    stream.send.finish()?;
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
