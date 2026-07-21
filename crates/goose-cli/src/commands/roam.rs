//! `goose roam` — peer-to-peer agent sharing over iroh.
//!
//! Subcommands:
//! * `share` — bind a roaming endpoint and host this machine's agent, printing
//!   an invite token for a client to connect with.
//! * `connect` — dial a shared agent by saved peer name or invite token.
//! * `peers` — manage the address book of remote agents you can connect to.
//! * `connections` (alias `list`) — show live/observed connections.
//! * `id` — print this machine's host and client endpoint ids.

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use goose::acp::server_factory::{AcpServer, AcpServerFactoryConfig};
use goose::agents::GoosePlatform;
use goose::config::paths::Paths;
use goose::session::SessionManager;
use goose_roaming::{
    default_key_path, parse_endpoint_id, Directory, RelaySettings, RoamingConfig, RoamingIdentity,
    RoamingNode, Scope, TrustBook, TrustPolicy,
};

use crate::commands::shared_session_bridge::{
    GooseAgentBackend, ResumeTarget, SharedSessionBridge,
};

fn directory_path() -> std::path::PathBuf {
    Paths::state_dir().join("roaming_directory.json")
}

/// CLI surface for the roaming [`Scope`]. Kept separate so `goose-roaming`
/// stays free of the clap dependency.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShareScope {
    /// Drive the agent and answer tool-permission prompts (effectively remote
    /// shell access). Only grant to trusted peers.
    Control,
    /// Send prompts/steer a shared session, but never answer permission prompts.
    Attach,
    /// Watch session activity read-only.
    Observe,
}

impl From<ShareScope> for Scope {
    fn from(value: ShareScope) -> Self {
        match value {
            ShareScope::Control => Scope::Control,
            ShareScope::Attach => Scope::Attach,
            ShareScope::Observe => Scope::Observe,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum RoamCommand {
    /// Host this machine's agent and print an invite token.
    ///
    /// By default a shared agent grants full control (the connecting peer can
    /// drive the agent and approve its tool use — effectively remote shell
    /// access), so only ever share `control` with peers you trust. Use
    /// `--scope attach` or `--scope observe` to hand out narrower capabilities
    /// that are enforced host-side.
    Share {
        /// What a connecting peer may do:
        /// `control` (default) drives the agent and answers tool-permission
        /// prompts; `attach` may send prompts/steer but never answers permission
        /// prompts; `observe` watches session activity read-only. Multiple peers
        /// can share one live session — e.g. share `control` for yourself and
        /// hand out `observe` invites for others to watch.
        #[arg(long, value_enum, default_value_t = ShareScope::Control)]
        scope: ShareScope,

        /// Invite lifetime in seconds.
        #[arg(long, default_value_t = 3600)]
        ttl: u64,

        /// Only these client endpoint ids may connect (repeatable). When
        /// omitted the invite is bearer: anyone holding it may connect.
        #[arg(long = "allow-key", value_name = "ENDPOINT_ID", action = clap::ArgAction::Append)]
        allow_keys: Vec<String>,

        /// Make the invite single-use: it is consumed on first redemption and
        /// the redeeming client's key is pinned to the allowlist.
        #[arg(long)]
        pair: bool,

        /// Builtin extensions to load into the hosted agent.
        #[arg(long = "with-builtin", value_delimiter = ',')]
        builtins: Vec<String>,

        /// Working directory the hosted agent runs in. Defaults to the directory
        /// `roam share` was started in. The connecting client's own path is
        /// always ignored — it is meaningless on this machine. Ignored when
        /// `--session` is given (a resumed session keeps its own directory).
        #[arg(long)]
        cwd: Option<std::path::PathBuf>,

        /// Resume an existing local session by id instead of starting fresh.
        /// Its conversation history is replayed into the hosted agent and it
        /// runs in the session's own working directory. Use `roam sessions` to
        /// list ids.
        #[arg(long, value_name = "SESSION_ID")]
        session: Option<String>,
    },

    /// List local sessions that can be resumed with `roam share --session`.
    Sessions,

    /// Connect to a shared agent by saved peer name or invite token.
    Connect {
        /// A saved peer nickname (see `roam peers`) or a `goose+roam://...` token.
        target: String,

        /// Optional label reported to the host's directory.
        #[arg(long)]
        label: Option<String>,
    },

    /// Delegate a one-shot task to a remote agent and print its response.
    Delegate {
        /// A saved peer nickname (see `roam peers`) or a `goose+roam://...` token.
        target: String,
        /// The task/question to send to the remote agent.
        task: String,
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
        /// A saved peer nickname (see `roam peers`) or a `goose+roam://...` token.
        target: String,

        /// Listen for one ACP client on this TCP address (e.g. `127.0.0.1:8900`)
        /// instead of using stdio.
        #[arg(long, value_name = "ADDR")]
        listen: Option<String>,

        /// Optional label reported to the host's directory.
        #[arg(long)]
        label: Option<String>,
    },

    /// Manage the address book of remote agents you can connect to.
    Peers {
        #[command(subcommand)]
        command: Option<PeersCommand>,
    },

    /// Print this machine's roaming endpoint id.
    Id,

    /// Show live/observed connections to and from this node.
    #[command(visible_alias = "list")]
    Connections,
}

#[derive(Debug, Subcommand)]
pub enum PeersCommand {
    /// Add or refresh a saved remote's credential.
    Save {
        /// Friendly nickname.
        name: String,
        /// The `goose+roam://...` invite token.
        invite: String,
    },
    /// Remove a saved remote.
    Remove { name: String },
    /// Rename a saved remote.
    Rename { from: String, to: String },
    /// List saved remotes (default).
    List,
}

pub async fn handle_roam_command(command: RoamCommand) -> Result<()> {
    match command {
        RoamCommand::Id => {
            let host = load_identity()?;
            let client = load_client_identity()?;
            println!("host   (share) : {}", host.public_key());
            println!("client (connect): {}", client.public_key());
            Ok(())
        }
        RoamCommand::Share {
            scope,
            ttl,
            allow_keys,
            pair,
            builtins,
            cwd,
            session,
        } => handle_share(scope.into(), ttl, allow_keys, pair, builtins, cwd, session).await,
        RoamCommand::Sessions => handle_sessions().await,
        RoamCommand::Connect { target, label } => handle_connect(target, label).await,
        RoamCommand::Delegate { target, task } => handle_delegate(target, task).await,
        RoamCommand::Bridge {
            target,
            listen,
            label,
        } => handle_bridge(target, listen, label).await,
        RoamCommand::Peers { command } => handle_peers(command.unwrap_or(PeersCommand::List)).await,
        RoamCommand::Connections => handle_list().await,
    }
}

fn peerbook_path() -> std::path::PathBuf {
    Paths::config_dir().join("roaming_peers.json")
}

async fn handle_peers(command: PeersCommand) -> Result<()> {
    let mut book = goose_roaming::PeerBook::load(peerbook_path())?;
    match command {
        PeersCommand::Save { name, invite } => {
            book.save(&name, &invite, now_ms())?;
            let rec = book.get(&name).expect("just saved");
            if rec.bearer {
                eprintln!(
                    "warning: `{name}` is a BEARER credential — anyone holding it can connect. \
                     Prefer a client-key-bound invite (host uses --allow-key)."
                );
            }
            eprintln!(
                "saved peer `{name}` -> {} (scope {:?})",
                rec.endpoint_id, rec.scope
            );
            Ok(())
        }
        PeersCommand::Remove { name } => {
            if book.remove(&name)? {
                eprintln!("removed peer `{name}`");
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
            let peers = book.list();
            if peers.is_empty() {
                eprintln!("no saved peers; add one with `goose roam peers save <name> <invite>`");
                return Ok(());
            }
            println!("{:<16} {:<8} {:<9} ENDPOINT ID", "NAME", "SCOPE", "CRED");
            let now = now_ms();
            for p in peers {
                let scope = format!("{:?}", p.scope).to_lowercase();
                let cred = if p.expires_at_ms <= now {
                    "expired"
                } else if p.bearer {
                    "bearer"
                } else {
                    "key-bound"
                };
                println!("{:<16} {scope:<8} {cred:<9} {}", p.name, p.endpoint_id);
            }
            Ok(())
        }
    }
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
    println!(
        "{:<10} {:<9} {:<8} {:<20} ENDPOINT ID",
        "STATUS", "DIR", "SCOPE", "AGENT"
    );
    for e in entries {
        let status = if e.connected { "connected" } else { "seen" };
        let dir = match e.direction {
            goose_roaming::Direction::Inbound => "inbound",
            goose_roaming::Direction::Outbound => "outbound",
        };
        let scope = format!("{:?}", e.scope).to_lowercase();
        let agent = e.agent_id.unwrap_or_else(|| "-".to_string());
        let agent = if agent.chars().count() > 20 {
            let truncated: String = agent.chars().take(19).collect();
            format!("{truncated}…")
        } else {
            agent
        };
        println!(
            "{status:<10} {dir:<9} {scope:<8} {agent:<20} {}",
            e.endpoint_id
        );
    }
    Ok(())
}

fn load_identity() -> Result<RoamingIdentity> {
    let path = default_key_path(&Paths::config_dir());
    RoamingIdentity::load_or_create(&path).context("failed to load roaming identity")
}

/// Stable outbound (client) identity, distinct from the host key so a node
/// never dials itself. Required for durable client-key-bound grants and pairing.
fn load_client_identity() -> Result<RoamingIdentity> {
    let path = Paths::config_dir().join("roaming_client_key");
    RoamingIdentity::load_or_create(&path).context("failed to load roaming client identity")
}

async fn handle_sessions() -> Result<()> {
    let manager = SessionManager::new(Paths::data_dir());
    let mut sessions = manager.list_sessions().await?;
    if sessions.is_empty() {
        eprintln!("no local sessions found");
        return Ok(());
    }
    sessions.sort_by_key(|s| std::cmp::Reverse(s.updated_at));

    println!("{:<40} {:<20} UPDATED", "SESSION ID", "NAME");
    for s in sessions {
        let name = if s.name.chars().count() > 20 {
            let truncated: String = s.name.chars().take(19).collect();
            format!("{truncated}…")
        } else {
            s.name
        };
        println!(
            "{:<40} {name:<20} {}",
            s.id,
            s.updated_at.format("%Y-%m-%d %H:%M")
        );
    }
    eprintln!("\nresume one with: goose roam share --session <SESSION ID>");
    Ok(())
}

async fn handle_share(
    scope: Scope,
    ttl: u64,
    allow_keys: Vec<String>,
    pair: bool,
    builtins: Vec<String>,
    cwd: Option<std::path::PathBuf>,
    session: Option<String>,
) -> Result<()> {
    let identity = load_identity()?;

    // A resumed session keeps its own working directory (replayed history is
    // meaningless in a different tree); a fresh session uses `--cwd` or the
    // directory `roam share` was started in. The connector's path is always
    // ignored — it is meaningless on this machine.
    let resume = match session {
        Some(session_id) => {
            let manager = SessionManager::new(Paths::data_dir());
            let session = manager
                .get_session(&session_id, false)
                .await
                .with_context(|| format!("no local session with id `{session_id}`"))?;
            ResumeTarget::Existing {
                session_id,
                cwd: session.working_dir,
            }
        }
        None => {
            let cwd = match cwd {
                Some(dir) => std::fs::canonicalize(&dir)
                    .with_context(|| format!("invalid --cwd: {}", dir.display()))?,
                None => std::env::current_dir().context("could not determine current directory")?,
            };
            ResumeTarget::New { cwd }
        }
    };
    let session_cwd = match &resume {
        ResumeTarget::New { cwd } | ResumeTarget::Existing { cwd, .. } => cwd.clone(),
    };

    let allowed_client_keys = allow_keys
        .iter()
        .map(|s| parse_endpoint_id(s).map_err(anyhow::Error::from))
        .collect::<Result<Vec<_>>>()?;
    let policy = if allowed_client_keys.is_empty() && !pair {
        TrustPolicy::Bearer
    } else {
        TrustPolicy::Allowlist
    };
    let mut trust = TrustBook::new(policy);
    for key in &allowed_client_keys {
        trust.allow(key);
    }

    // Default to the developer extension when none are specified, matching
    // `goose acp` / `goose serve`.
    let builtins = if builtins.is_empty() {
        vec!["developer".to_string()]
    } else {
        builtins
    };

    let relay = RelaySettings::N0Default;
    let node = RoamingNode::bind(RoamingConfig {
        identity: identity.clone(),
        relay: relay.clone(),
        trust,
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
    }));
    let backend = Arc::new(GooseAgentBackend::new(acp_server));
    let bridge = Arc::new(SharedSessionBridge::start(
        backend,
        identity.public_key().to_string(),
        resume,
    ));

    node.share(bridge).await?;

    // Wait for the endpoint to reach a relay so the invite can carry a live
    // relay URL the client can dial (the Minimal preset has no DNS discovery).
    eprintln!("connecting to relay...");
    if !node.wait_online(std::time::Duration::from_secs(15)).await {
        eprintln!("warning: endpoint did not come online; invite may lack a reachable address");
    }

    let mut invite_opts =
        goose_roaming::InviteOptions::new(scope, std::time::Duration::from_secs(ttl));
    invite_opts.allowed_client_keys = allowed_client_keys;
    invite_opts.single_use = pair;
    let invite = node.make_invite(invite_opts);
    let token = invite.encode()?;

    eprintln!("roaming agent is live");
    eprintln!("  endpoint id : {}", node.endpoint_id());
    eprintln!("  scope       : {scope:?}");
    eprintln!("  working dir : {}", session_cwd.display());
    eprintln!(
        "  invite ttl  : {ttl}s{}",
        if pair { " (single-use pairing)" } else { "" }
    );
    eprintln!();
    eprintln!("share this invite token with the connecting client:");
    println!("{token}");
    eprintln!();
    eprintln!("connect from another machine with:");
    eprintln!("  goose roam connect '{token}'");
    eprintln!();
    eprintln!("press Ctrl-C to stop sharing");

    tokio::signal::ctrl_c().await?;
    eprintln!("\nshutting down roaming endpoint...");
    node.shutdown().await?;
    Ok(())
}

/// Resolve a target (saved peer nickname or raw invite token) to a token.
fn resolve_target(target: &str) -> Result<String> {
    if target.starts_with("goose+roam://") {
        return Ok(target.to_string());
    }
    let book = goose_roaming::PeerBook::load(peerbook_path())?;
    match book.get(target) {
        Some(rec) => Ok(rec.invite.clone()),
        None => anyhow::bail!(
            "no saved peer named `{target}` (and it is not an invite token); \
             see `goose roam peers`"
        ),
    }
}

/// Bind a client node and dial the target, returning the node + authorized
/// stream. Clients use a *stable* outbound identity (persisted, distinct from
/// the host key so a node never dials itself).
async fn dial_target(
    target: &str,
    label: Option<String>,
) -> Result<(
    std::sync::Arc<RoamingNode>,
    goose_roaming::RoamingClientStream,
)> {
    use goose_roaming::SignedInvite;
    let token = resolve_target(target)?;
    let invite = SignedInvite::decode(&token)?;
    let node = RoamingNode::bind(RoamingConfig {
        identity: load_client_identity()?,
        relay: RelaySettings::N0Default,
        trust: TrustBook::new(TrustPolicy::Bearer),
        directory: Directory::new(),
        bind_addr: None,
    })
    .await?;
    eprintln!("connecting to {}...", invite.claims.audience);
    let stream = node.connect(&invite, label).await?;
    Ok((node, stream))
}

async fn handle_connect(target: String, label: Option<String>) -> Result<()> {
    let (node, stream) = dial_target(&target, label).await?;
    let agent_label = stream.agent_id.clone();
    eprintln!("authorized with scope {:?}", stream.scope);
    let result = crate::commands::roam_client::run_interactive(stream, agent_label).await;
    node.shutdown().await?;
    result
}

async fn handle_delegate(target: String, task: String) -> Result<()> {
    let (node, stream) = dial_target(&target, Some("delegate".to_string())).await?;
    eprintln!("delegating task to `{}`...", stream.agent_id);
    let result = crate::commands::roam_client::delegate(stream, task).await;
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
    let scope = stream.scope;
    // The raw iroh streams carry post-handshake ACP and already implement
    // tokio's AsyncRead/AsyncWrite, so we splice them directly (no ACP-client
    // wrapper, no futures-io compat). `conn` must outlive the splice.
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
            eprintln!("bridging remote agent `{agent_id}` (scope {scope:?}) on tcp://{local}");
            eprintln!("point an ACP client at this address; serving one connection");
            let (socket, peer) = listener.accept().await?;
            eprintln!("ACP client connected from {peer}");
            let (lr, lw) = socket.into_split();
            crate::commands::roam_proxy::splice(lr, lw, remote_send, remote_recv).await
        }
        None => {
            eprintln!(
                "bridging remote agent `{agent_id}` (scope {scope:?}) over stdio; \
                 speak ACP on stdin/stdout"
            );
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            crate::commands::roam_proxy::splice(stdin, stdout, remote_send, remote_recv).await
        }
    };

    // Flush stdout before tearing down (stdio mode); harmless otherwise.
    let _ = tokio::io::stdout().flush().await;
    drop(conn);
    node.shutdown().await?;
    result
}
