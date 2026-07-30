//! `goose roam` — peer-to-peer agent access over iroh.
//!
//! The model is deliberately infrastructural: roaming is just an authenticated
//! p2p ACP transport. Each node has one identity and produces a **connection
//! card** (`roam id`) — a non-secret string carrying its public key and how to
//! reach it. You swap cards with another node and each side chooses to **accept**
//! the other's key. A connection only succeeds when the host has accepted the
//! dialer's key; there is no bearer token that grants access by possession. An
//! accepted peer gets goose's full ACP surface.
//!
//! Subcommands:
//! * `id` — print this node's connection card (share it with a peer).
//! * `share` — serve this node's agent to accepted peers over ACP.
//! * `peers` — manage saved peer cards and which keys you accept.
//! * `connect` / `delegate` / `bridge` — reach a peer that has accepted you.
//! * `connections` (alias `list`) — show live/observed connections.

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Subcommand;
use goose::acp::server_factory::{AcpServer, AcpServerFactoryConfig};
use goose::agents::GoosePlatform;
use goose::config::paths::Paths;
use goose_roaming::{
    default_key_path, parse_endpoint_id, ConnectionCard, Directory, EndpointId, RelaySettings,
    RoamingConfig, RoamingIdentity, RoamingNode, TrustBook,
};

use crate::commands::roam_full_bridge::FullAcpBridge;

const CARD_SCHEME: &str = "goose+roam://";

fn directory_path() -> std::path::PathBuf {
    Paths::state_dir().join("roaming_directory.json")
}

fn trust_path() -> std::path::PathBuf {
    Paths::config_dir().join("roaming_trust.json")
}

fn peerbook_path() -> std::path::PathBuf {
    Paths::config_dir().join("roaming_peers.json")
}

#[derive(Debug, Subcommand)]
pub enum RoamCommand {
    /// Print this node's connection card — the non-secret string you share with
    /// a peer so it can find and identify this node. Nothing in it is a secret;
    /// a peer must still be accepted (`roam peers accept`) before it can connect.
    #[command(visible_alias = "card")]
    Id,

    /// Serve this node's agent to accepted peers over ACP.
    ///
    /// Only peers whose key you have accepted (`roam peers accept`) can connect.
    /// Each connected peer gets goose's full ACP surface — it drives its own
    /// sessions (new/list/load/prompt) backed by this node's session store.
    Share {
        /// Builtin extensions to load into the hosted agent.
        #[arg(long = "with-builtin", value_delimiter = ',')]
        builtins: Vec<String>,

        /// Working directory the hosted agent runs in. Defaults to the directory
        /// `roam share` was started in. The connecting client's own path is
        /// always ignored.
        #[arg(long)]
        cwd: Option<std::path::PathBuf>,
    },

    /// Open a quick interactive REPL against a remote agent (debug/peek).
    ///
    /// This is a minimal built-in chat loop, handy for a quick sanity check. For
    /// real work, prefer `bridge` (drive the remote agent from the goose desktop
    /// app, Zed, or any ACP client) or `delegate` (scriptable one-shot tasks).
    Connect {
        /// A saved peer nickname (see `roam peers`) or a `goose+roam://...` card.
        target: String,

        /// Optional label reported to the host's directory.
        #[arg(long)]
        label: Option<String>,
    },

    /// Delegate a one-shot task to a remote agent and print its response.
    ///
    /// This is a thin ACP client: it connects, opens a session (new, or the one
    /// named by `--session`), sends the task as a prompt, prints the reply, and
    /// exits. Session enumeration and resume are plain ACP (`session/list` /
    /// `session/load`) served by the remote's full ACP surface.
    Delegate {
        /// A saved peer nickname (see `roam peers`) or a `goose+roam://...` card.
        target: String,
        /// The task/question to send to the remote agent. Omit when using
        /// `--list-sessions`.
        task: Option<String>,
        /// Run the task against an existing remote session id (via `session/load`)
        /// instead of a fresh session. List ids with `--list-sessions`.
        #[arg(long, value_name = "SESSION_ID")]
        session: Option<String>,
        /// List the remote agent's sessions (`session/list`) and exit.
        #[arg(long)]
        list_sessions: bool,
    },

    /// Expose a remote agent as a local ACP endpoint that any ACP client can drive.
    ///
    /// Unlike `connect` (which has its own terminal UI), `bridge` runs no UI and
    /// no agent: it transparently proxies ACP between a local transport and the
    /// remote agent. Point the goose desktop app, Zed, or any ACP client at it
    /// and the remote agent behaves as if it were running locally.
    ///
    /// Defaults to stdio (for a client that spawns `goose roam bridge ...` as a
    /// subprocess). Use `--listen` to accept a single TCP connection instead.
    Bridge {
        /// A saved peer nickname (see `roam peers`) or a `goose+roam://...` card.
        target: String,

        /// Listen for one ACP client on this TCP address (e.g. `127.0.0.1:8900`)
        /// instead of using stdio.
        #[arg(long, value_name = "ADDR")]
        listen: Option<String>,

        /// Optional label reported to the host's directory.
        #[arg(long)]
        label: Option<String>,
    },

    /// Manage saved peer cards and which peer keys this node accepts.
    Peers {
        #[command(subcommand)]
        command: Option<PeersCommand>,
    },

    /// Show live/observed connections to and from this node.
    #[command(visible_alias = "list")]
    Connections,
}

#[derive(Debug, Subcommand)]
pub enum PeersCommand {
    /// Save a peer's connection card to the address book so you can reach it by
    /// name. Does NOT let them connect to you — use `accept` for that.
    Add {
        /// The peer's `goose+roam://...` card.
        card: String,
        /// Friendly nickname (defaults to a short id if omitted).
        name: Option<String>,
    },
    /// Accept inbound connections from a peer's key. The target is a saved
    /// nickname or a `goose+roam://...` card (which is also saved to the address
    /// book). An accepted peer gets goose's full ACP surface.
    Accept {
        /// A saved nickname or a `goose+roam://...` card.
        target: String,
        /// Nickname to save an inline card under (defaults to a short id).
        /// Ignored when the target is already a saved nickname.
        name: Option<String>,
    },
    /// Stop accepting a peer: a saved nickname, a card, or a raw endpoint id.
    /// A live session continues until it disconnects.
    Revoke { target: String },
    /// Remove a saved peer from the address book (does not change acceptance).
    Remove { name: String },
    /// Rename a saved peer.
    Rename { from: String, to: String },
    /// List saved peers and which keys are accepted (default).
    List,
}

pub async fn handle_roam_command(command: RoamCommand) -> Result<()> {
    match command {
        RoamCommand::Id => handle_id().await,
        RoamCommand::Share { builtins, cwd } => handle_share(builtins, cwd).await,
        RoamCommand::Connect { target, label } => handle_connect(target, label).await,
        RoamCommand::Delegate {
            target,
            task,
            session,
            list_sessions,
        } => handle_delegate(target, task, session, list_sessions).await,
        RoamCommand::Bridge {
            target,
            listen,
            label,
        } => handle_bridge(target, listen, label).await,
        RoamCommand::Peers { command } => handle_peers(command.unwrap_or(PeersCommand::List)).await,
        RoamCommand::Connections => handle_list().await,
    }
}

/// Bind a node briefly to read its live card (id + relay URLs), waiting for a
/// relay so the card carries a reachable address.
async fn handle_id() -> Result<()> {
    let identity = load_identity()?;
    let node = RoamingNode::bind(RoamingConfig {
        identity,
        relay: RelaySettings::N0Default,
        trust: TrustBook::new(),
        trust_path: None,
        directory: Directory::new(),
        bind_addr: None,
    })
    .await?;
    eprintln!("contacting relay so the card carries a reachable address...");
    node.wait_online(std::time::Duration::from_secs(15)).await;
    let card = node.card();
    eprintln!("your connection card (share this with a peer):");
    println!("{}", card.encode()?);
    eprintln!();
    eprintln!("  endpoint id : {}", card.endpoint_id);
    eprintln!("  fingerprint : {}", card.fingerprint());
    eprintln!();
    eprintln!("the peer adds it with:  goose roam peers add '<card>' <name>");
    eprintln!("and accepts you with:   goose roam peers accept <name>");
    node.shutdown().await?;
    Ok(())
}

async fn handle_peers(command: PeersCommand) -> Result<()> {
    let mut book = goose_roaming::PeerBook::load(peerbook_path())?;
    match command {
        PeersCommand::Add { card, name } => {
            let decoded = ConnectionCard::decode(&card)?;
            let name = name.unwrap_or_else(|| short_id(&decoded.endpoint_id.to_string()));
            book.save(&name, &card, now_ms())?;
            eprintln!(
                "saved peer `{name}` -> {} (fingerprint {})",
                decoded.endpoint_id,
                decoded.fingerprint()
            );
            eprintln!("accept connections from it with: goose roam peers accept {name}");
            Ok(())
        }
        PeersCommand::Accept { target, name } => {
            // Resolve to a card: a saved name, or an inline card we also save.
            let card = match ConnectionCard::decode(&target) {
                Ok(card) => {
                    let name = name.unwrap_or_else(|| short_id(&card.endpoint_id.to_string()));
                    book.save(&name, &target, now_ms())?;
                    card
                }
                Err(_) => {
                    if name.is_some() {
                        eprintln!("note: `{target}` is a saved peer; ignoring the extra name arg");
                    }
                    let rec = book.get(&target).ok_or_else(|| {
                        anyhow::anyhow!(
                            "no saved peer `{target}` and it is not a card; add it first with \
                             `goose roam peers add`"
                        )
                    })?;
                    rec.card.clone()
                }
            };
            let path = trust_path();
            let mut trust = TrustBook::load(&path).unwrap_or_default();
            trust.accept(&card.endpoint_id);
            trust.save(&path)?;
            eprintln!("accepting connections from {}", card.endpoint_id);
            eprintln!("verify the fingerprint out of band: {}", card.fingerprint());
            eprintln!("a running `goose roam share` picks this up on the next connection");
            Ok(())
        }
        PeersCommand::Revoke { target } => {
            let key = resolve_key(&book, &target)?;
            let path = trust_path();
            let mut trust = TrustBook::load(&path).unwrap_or_default();
            trust.revoke_key(&key);
            trust.save(&path)?;
            eprintln!("revoked {key}; it can no longer connect");
            eprintln!("note: an already-open session is unaffected until it disconnects");
            Ok(())
        }
        PeersCommand::Remove { name } => {
            if book.remove(&name)? {
                eprintln!("removed peer `{name}` from the address book");
            } else {
                eprintln!("no peer named `{name}`");
            }
            Ok(())
        }
        PeersCommand::Rename { from, to } => {
            book.rename(&from, &to)?;
            eprintln!("renamed `{from}` -> `{to}`");
            Ok(())
        }
        PeersCommand::List => {
            let trust = TrustBook::load(&trust_path()).unwrap_or_default();
            let accepted: std::collections::HashSet<String> =
                trust.allowed_keys().into_iter().collect();
            let peers = book.list();
            if peers.is_empty() && accepted.is_empty() {
                eprintln!("no saved peers; add one with `goose roam peers add '<card>' <name>`");
                return Ok(());
            }
            println!("{:<16} {:<8} ENDPOINT ID", "NAME", "ACCEPT");
            for p in &peers {
                let accept = if accepted.contains(&p.endpoint_id) {
                    "yes"
                } else {
                    "no"
                };
                println!("{:<16} {accept:<8} {}", p.name, p.endpoint_id);
            }
            // Accepted keys with no saved card (accepted by raw id).
            let known: std::collections::HashSet<String> =
                peers.iter().map(|p| p.endpoint_id.clone()).collect();
            for id in &accepted {
                if !known.contains(id) {
                    println!("{:<16} {:<8} {id}", "(unsaved)", "yes");
                }
            }
            Ok(())
        }
    }
}

/// Resolve a target (saved nickname, inline card, or raw endpoint id) to a key.
fn resolve_key(book: &goose_roaming::PeerBook, target: &str) -> Result<EndpointId> {
    if let Ok(card) = ConnectionCard::decode(target) {
        return Ok(card.endpoint_id);
    }
    if let Some(rec) = book.get(target) {
        return Ok(rec.card.endpoint_id);
    }
    parse_endpoint_id(target)
        .map_err(|_| anyhow::anyhow!("`{target}` is not a saved peer, a card, or an endpoint id"))
}

fn short_id(id: &str) -> String {
    id.chars().take(12).collect()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

async fn handle_list() -> Result<()> {
    let entries = Directory::read_persisted(&directory_path());
    if entries.is_empty() {
        eprintln!("no roaming peers recorded yet");
        return Ok(());
    }
    println!("{:<10} {:<9} {:<20} ENDPOINT ID", "STATUS", "DIR", "AGENT");
    for e in entries {
        let status = if e.connected { "connected" } else { "seen" };
        let dir = match e.direction {
            goose_roaming::Direction::Inbound => "inbound",
            goose_roaming::Direction::Outbound => "outbound",
        };
        let agent = e.agent_id.unwrap_or_else(|| "-".to_string());
        let agent = if agent.chars().count() > 20 {
            let truncated: String = agent.chars().take(19).collect();
            format!("{truncated}…")
        } else {
            agent
        };
        println!("{status:<10} {dir:<9} {agent:<20} {}", e.endpoint_id);
    }
    Ok(())
}

/// This node's single long-lived identity. Its public key is what peers accept
/// and what the connection card advertises.
fn load_identity() -> Result<RoamingIdentity> {
    let path = default_key_path(&Paths::config_dir());
    RoamingIdentity::load_or_create(&path).context("failed to load roaming identity")
}

async fn handle_share(builtins: Vec<String>, cwd: Option<std::path::PathBuf>) -> Result<()> {
    let identity = load_identity()?;

    // The hosted agent runs in `--cwd` or the directory `roam share` was started
    // in; the connecting client's own path is meaningless here and is ignored.
    let session_cwd = match &cwd {
        Some(dir) => std::fs::canonicalize(dir)
            .with_context(|| format!("invalid --cwd: {}", dir.display()))?,
        None => std::env::current_dir().context("could not determine current directory")?,
    };

    // Load the accepted-peer allowlist. Peers are accepted out of band with
    // `roam peers accept`; this serve loop re-reads it per connection.
    let trust = TrustBook::load(&trust_path()).unwrap_or_default();
    let accepted_count = trust.allowed_keys().len();
    if accepted_count == 0 {
        eprintln!(
            "warning: no peers are accepted yet — no one can connect.\n\
             Accept a peer's key first: goose roam peers accept <name|card>"
        );
    }

    // Default to the developer extension when none are specified.
    let builtins = if builtins.is_empty() {
        vec!["developer".to_string()]
    } else {
        builtins
    };

    let node = RoamingNode::bind(RoamingConfig {
        identity,
        relay: RelaySettings::N0Default,
        trust,
        // Re-read acceptance on each connection so `peers accept`/`revoke` from
        // another process take effect against this live share without restart.
        trust_path: Some(trust_path()),
        directory: Directory::persistent(directory_path()),
        bind_addr: None,
    })
    .await?;

    let acp_server = Arc::new(AcpServer::new(AcpServerFactoryConfig {
        builtins,
        data_dir: Paths::data_dir(),
        config_dir: Paths::config_dir(),
        goose_platform: GoosePlatform::GooseCli,
        additional_source_roots: Vec::new(),
        session_cwd: Some(session_cwd.clone()),
        enable_scheduler: false,
    }));
    let agent_id = node.endpoint_id().to_string();
    let bridge = Arc::new(FullAcpBridge::new(acp_server, agent_id));
    node.share(bridge).await?;

    eprintln!("contacting relay...");
    if !node.wait_online(std::time::Duration::from_secs(15)).await {
        eprintln!("warning: endpoint did not come online; the card may lack a reachable address");
    }

    eprintln!("roaming agent is live");
    eprintln!("  endpoint id : {}", node.endpoint_id());
    eprintln!("  working dir : {}", session_cwd.display());
    eprintln!("  accepted    : {accepted_count} peer key(s)");
    eprintln!();
    eprintln!("your connection card (share with a peer so it can reach you):");
    println!("{}", node.card().encode()?);
    eprintln!();
    eprintln!("press Ctrl-C to stop sharing");

    tokio::signal::ctrl_c().await?;
    eprintln!("\nshutting down roaming endpoint...");
    node.shutdown().await?;
    Ok(())
}

/// Resolve a target (saved peer nickname or inline card) to a [`ConnectionCard`].
fn resolve_card(target: &str) -> Result<ConnectionCard> {
    if target.starts_with(CARD_SCHEME) {
        return ConnectionCard::decode(target).map_err(Into::into);
    }
    let book = goose_roaming::PeerBook::load(peerbook_path())?;
    match book.get(target) {
        Some(rec) => Ok(rec.card.clone()),
        None => anyhow::bail!(
            "no saved peer named `{target}` (and it is not a card); see `goose roam peers`"
        ),
    }
}

/// Bind this node and dial the target's card, returning the node + authorized
/// stream. The connection succeeds only if the remote has accepted this node's
/// key.
async fn dial_target(
    target: &str,
    label: Option<String>,
) -> Result<(
    std::sync::Arc<RoamingNode>,
    goose_roaming::RoamingClientStream,
)> {
    let card = resolve_card(target)?;
    let node = RoamingNode::bind(RoamingConfig {
        identity: load_identity()?,
        relay: RelaySettings::N0Default,
        trust: TrustBook::new(),
        trust_path: None,
        directory: Directory::new(),
        bind_addr: None,
    })
    .await?;
    eprintln!("connecting to {}...", card.endpoint_id);
    let stream = node.connect(&card, label).await?;
    Ok((node, stream))
}

async fn handle_connect(target: String, label: Option<String>) -> Result<()> {
    let (node, stream) = dial_target(&target, label).await?;
    let agent_label = stream.agent_id.clone();
    eprintln!("connected to `{agent_label}`");
    let result = crate::commands::roam_client::run_interactive(stream, agent_label).await;
    node.shutdown().await?;
    result
}

async fn handle_delegate(
    target: String,
    task: Option<String>,
    session: Option<String>,
    list_sessions: bool,
) -> Result<()> {
    if list_sessions {
        let (node, stream) = dial_target(&target, Some("delegate".to_string())).await?;
        eprintln!("listing sessions on `{}`...", stream.agent_id);
        let result = crate::commands::roam_client::list_sessions(stream).await;
        node.shutdown().await?;
        let sessions = result?;
        if sessions.is_empty() {
            eprintln!("no sessions on the remote agent");
            return Ok(());
        }
        println!("{:<40} {:<20} UPDATED", "SESSION ID", "TITLE");
        for s in sessions {
            let title = s.title.unwrap_or_default();
            let title = if title.chars().count() > 20 {
                format!("{}…", title.chars().take(19).collect::<String>())
            } else {
                title
            };
            println!(
                "{:<40} {title:<20} {}",
                s.session_id,
                s.updated_at.unwrap_or_default()
            );
        }
        return Ok(());
    }

    let task = task.context("a task is required (or pass --list-sessions)")?;
    let (node, stream) = dial_target(&target, Some("delegate".to_string())).await?;
    match &session {
        Some(id) => eprintln!("delegating to `{}` session {id}...", stream.agent_id),
        None => eprintln!("delegating task to `{}`...", stream.agent_id),
    }
    let result = crate::commands::roam_client::delegate(stream, task, session).await;
    node.shutdown().await?;
    match result {
        Ok(response) => {
            println!("{response}");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

async fn handle_bridge(
    target: String,
    listen: Option<String>,
    label: Option<String>,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let label = label.or_else(|| Some("bridge".to_string()));
    let (node, stream) = dial_target(&target, label).await?;
    let agent_id = stream.agent_id.clone();
    // The raw iroh streams carry post-handshake ACP and already implement
    // tokio's AsyncRead/AsyncWrite, so we splice them directly. `conn` must
    // outlive the splice.
    let goose_roaming::RoamingClientStream {
        conn,
        send: remote_send,
        recv: remote_recv,
        ..
    } = stream;

    let result = match listen {
        Some(addr) => {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            let local = listener.local_addr()?;
            eprintln!("bridging remote agent `{agent_id}` on tcp://{local}");
            eprintln!("point an ACP client at this address; serving one connection");
            let (socket, peer) = listener.accept().await?;
            eprintln!("ACP client connected from {peer}");
            let (lr, lw) = socket.into_split();
            crate::commands::roam_proxy::splice(lr, lw, remote_send, remote_recv).await
        }
        None => {
            // stdio is a pure ACP transport: ONLY the splice may touch stdout.
            // All status goes to stderr so an ACP client reading stdout sees a
            // clean protocol stream.
            eprintln!("bridging remote agent `{agent_id}` over stdio; speak ACP on stdin/stdout");
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            crate::commands::roam_proxy::splice(stdin, stdout, remote_send, remote_recv).await
        }
    };

    let _ = tokio::io::stderr().flush().await;
    drop(conn);
    node.shutdown().await?;
    result
}
