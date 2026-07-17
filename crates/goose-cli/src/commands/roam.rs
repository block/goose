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
use goose_roaming::{
    default_key_path, parse_endpoint_id, Directory, RelaySettings, RoamingConfig, RoamingIdentity,
    RoamingNode, Scope, TrustBook, TrustPolicy,
};

use crate::commands::roam_bridge::GooseAcpBridge;

fn directory_path() -> std::path::PathBuf {
    Paths::state_dir().join("roaming_directory.json")
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShareScope {
    /// Full ACP control (effectively remote shell access). Trusted peers only.
    Control,
    /// Observe session activity without approving tool permissions.
    Observe,
}

impl From<ShareScope> for Scope {
    fn from(s: ShareScope) -> Self {
        match s {
            ShareScope::Control => Scope::Control,
            ShareScope::Observe => Scope::Observe,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum RoamCommand {
    /// Host this machine's agent and print an invite token.
    Share {
        /// Capability granted to the connecting client.
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
    },

    /// Connect to a shared agent by saved peer name or invite token.
    Connect {
        /// A saved peer nickname (see `roam peers`) or a `goose+roam://...` token.
        target: String,

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
        } => handle_share(scope.into(), ttl, allow_keys, pair, builtins).await,
        RoamCommand::Connect { target, label } => handle_connect(target, label).await,
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

async fn handle_share(
    scope: Scope,
    ttl: u64,
    allow_keys: Vec<String>,
    pair: bool,
    builtins: Vec<String>,
) -> Result<()> {
    let identity = load_identity()?;

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
    }));
    let bridge = Arc::new(GooseAcpBridge::new(
        acp_server,
        identity.public_key().to_string(),
    ));

    node.share(bridge).await?;

    // Wait for the endpoint to reach a relay so the invite can carry a live
    // relay URL the client can dial (the Minimal preset has no DNS discovery).
    eprintln!("connecting to relay...");
    if !node.wait_online(std::time::Duration::from_secs(15)).await {
        eprintln!("warning: endpoint did not come online; invite may lack a reachable address");
    }

    let invite = node.make_invite(&identity, &relay, scope, allowed_client_keys, ttl, pair);
    let token = invite.encode()?;

    eprintln!("roaming agent is live");
    eprintln!("  endpoint id : {}", node.endpoint_id());
    eprintln!("  scope       : {scope:?}");
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

async fn handle_connect(target: String, label: Option<String>) -> Result<()> {
    use goose_roaming::SignedInvite;

    // Resolve the target: a saved peer nickname or a raw invite token.
    let token = if target.starts_with("goose+roam://") {
        target.clone()
    } else {
        let book = goose_roaming::PeerBook::load(peerbook_path())?;
        match book.get(&target) {
            Some(rec) => rec.invite.clone(),
            None => {
                anyhow::bail!(
                    "no saved peer named `{target}` (and it is not an invite token); \
                     see `goose roam peers`"
                );
            }
        }
    };

    let invite = SignedInvite::decode(&token)?;
    // Clients use a *stable* outbound identity (persisted, distinct from the
    // host key so a node never dials itself). A stable client key is required
    // for durable client-key-bound grants and pairing across reconnects.
    let identity = load_client_identity()?;

    let node = RoamingNode::bind(RoamingConfig {
        identity,
        relay: RelaySettings::N0Default,
        trust: TrustBook::new(TrustPolicy::Bearer),
        directory: Directory::new(),
        bind_addr: None,
    })
    .await?;

    eprintln!("connecting to {}...", invite.claims.audience);
    let stream = node.connect(&invite, label).await?;
    let agent_label = stream.agent_id.clone();
    eprintln!("authorized with scope {:?}", stream.scope);

    let result = crate::commands::roam_client::run_interactive(stream, agent_label).await;

    node.shutdown().await?;
    result
}
