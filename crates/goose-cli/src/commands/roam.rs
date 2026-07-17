//! `goose roam` — peer-to-peer agent sharing over iroh.
//!
//! Subcommands:
//! * `share` — bind a roaming endpoint and host this machine's agent, printing
//!   an invite token for a client to connect with.
//! * `connect` — dial a shared agent using an invite token.
//! * `id` — print this machine's roaming endpoint id.

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use goose::acp::server_factory::{AcpServer, AcpServerFactoryConfig};
use goose::agents::GoosePlatform;
use goose::config::paths::Paths;
use goose_roaming::{
    default_key_path, parse_endpoint_id, GooseAcpBridge, RelaySettings, RoamingConfig,
    RoamingIdentity, RoamingNode, Scope, TrustBook, TrustPolicy,
};

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

    /// Connect to a shared agent using an invite token.
    Connect {
        /// The `goose+roam://...` invite token.
        token: String,

        /// Optional label reported to the host's directory.
        #[arg(long)]
        label: Option<String>,
    },

    /// Print this machine's roaming endpoint id.
    Id,
}

pub async fn handle_roam_command(command: RoamCommand) -> Result<()> {
    match command {
        RoamCommand::Id => {
            let identity = load_identity()?;
            println!("{}", identity.public_key());
            Ok(())
        }
        RoamCommand::Share {
            scope,
            ttl,
            allow_keys,
            pair,
            builtins,
        } => handle_share(scope.into(), ttl, allow_keys, pair, builtins).await,
        RoamCommand::Connect { token, label } => handle_connect(token, label).await,
    }
}

fn load_identity() -> Result<RoamingIdentity> {
    let path = default_key_path(&Paths::config_dir());
    RoamingIdentity::load_or_create(&path).context("failed to load roaming identity")
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

async fn handle_connect(token: String, label: Option<String>) -> Result<()> {
    use goose_roaming::SignedInvite;

    let invite = SignedInvite::decode(&token)?;
    let identity = load_identity()?;

    let node = RoamingNode::bind(RoamingConfig {
        identity,
        relay: RelaySettings::N0Default,
        trust: TrustBook::new(TrustPolicy::Bearer),
    })
    .await?;

    eprintln!("connecting to {}...", invite.claims.audience);
    let stream = node.connect(&invite, label).await?;
    eprintln!(
        "connected to agent `{}` with scope {:?}",
        stream.agent_id, stream.scope
    );

    // TODO(roaming): feed `stream` into the ACP client transport so the remote
    // agent becomes the local provider and an interactive session can begin.
    // For now we confirm the authorized channel is established.
    eprintln!("(interactive session bridging is not yet wired up)");

    node.shutdown().await?;
    Ok(())
}
