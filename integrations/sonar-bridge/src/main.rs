use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use fs2::FileExt;
use nostr::prelude::{PublicKey, RelayUrl, ToBech32};
use serde::{Deserialize, Serialize};
use sonar_core::client::SonarClient;
use sonar_core::identity::Identity;
use sonar_core::GroupId;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};
use tracing_subscriber::EnvFilter;

const PROTOCOL_VERSION: u16 = 1;
const CONFIG_VERSION: u32 = 1;
const CONFIG_FILE: &str = "config.json";
const LEDGER_FILE: &str = "commands.json";
const LOCK_FILE: &str = "bridge.lock";
const DB_DIR: &str = "marmot";
const DB_FILE: &str = "marmot.sqlite";
const MAX_LINE_BYTES: usize = 256 * 1024;
const MAX_MESSAGE_BYTES: usize = 64 * 1024;
const LIVE_WAIT_SECS: u64 = 5;
const SUBSCRIPTION_REFRESH_SECS: u64 = 60;
const SAFETY_SYNC_SECS: u64 = 5 * 60;
const DEFAULT_RELAYS: [&str; 3] = [
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
];

#[derive(Parser, Debug)]
#[command(name = "goose-sonar-bridge", version)]
#[command(about = "Sonar/Marmot transport bridge for the Goose gateway")]
struct Cli {
    #[arg(long, global = true)]
    home: Option<PathBuf>,
    #[arg(long = "relay", global = true)]
    relays: Vec<String>,
    #[arg(long = "controller", global = true)]
    controllers: Vec<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Serve,
    Identity,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentConfig {
    version: u32,
    nsec: String,
    db_key_hex: String,
    relays: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CommandLedger {
    known_groups: BTreeSet<String>,
    completed: BTreeSet<String>,
    executing: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BridgeCommand {
    Send {
        version: u16,
        request_id: String,
        group_id: String,
        text: String,
    },
    Complete {
        version: u16,
        request_id: String,
        message_id: String,
    },
    Shutdown {
        version: u16,
        request_id: String,
    },
}

impl BridgeCommand {
    fn version(&self) -> u16 {
        match self {
            Self::Send { version, .. }
            | Self::Complete { version, .. }
            | Self::Shutdown { version, .. } => *version,
        }
    }

    fn request_id(&self) -> &str {
        match self {
            Self::Send { request_id, .. }
            | Self::Complete { request_id, .. }
            | Self::Shutdown { request_id, .. } => request_id,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BridgeEvent<'a> {
    Ready {
        version: u16,
        npub: &'a str,
    },
    Message {
        version: u16,
        message_id: &'a str,
        group_id: &'a str,
        group_name: &'a str,
        sender: &'a str,
        content: &'a str,
    },
    GroupJoined {
        version: u16,
        group_id: &'a str,
        group_name: &'a str,
    },
    Response {
        version: u16,
        request_id: &'a str,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<&'a str>,
    },
}

struct LoadedConfig {
    home: PathBuf,
    config: AgentConfig,
    relays: Vec<RelayUrl>,
    controllers: HashSet<PublicKey>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(io::stderr)
        .init();

    if let Err(error) = run(Cli::parse()).await {
        error!(%error, "Sonar bridge failed");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let home = resolve_home(cli.home)?;
    let lock = acquire_instance_lock(&home)?;
    let loaded = LoadedConfig::load_or_init(home, cli.relays, cli.controllers)?;
    match cli.command {
        Command::Serve => serve(loaded, lock).await,
        Command::Identity => {
            println!("{}", loaded.npub()?);
            Ok(())
        }
    }
}

async fn serve(loaded: LoadedConfig, _lock: File) -> Result<()> {
    if loaded.controllers.is_empty() {
        return Err(anyhow!("at least one controller npub is required"));
    }
    let client = loaded.connect().await?;
    client.publish_key_package().await?;

    let ledger_path = loaded.home.join(LEDGER_FILE);
    let mut ledger = load_ledger(&ledger_path)?;
    recover_interrupted_commands(&client, &ledger_path, &mut ledger).await;
    client.sync().await?;
    accept_pending_invites(&client, &loaded.controllers).await?;
    let joined = checkpoint_new_groups(&client, &ledger_path, &mut ledger)?;

    let npub = client.identity().npub();
    emit(&BridgeEvent::Ready {
        version: PROTOCOL_VERSION,
        npub: &npub,
    })?;
    emit_joined_groups(&joined)?;

    let (input_tx, mut input_rx) = mpsc::channel::<String>(64);
    let input_task = tokio::spawn(read_commands(input_tx));
    let mut subscription_refresh =
        tokio::time::interval(Duration::from_secs(SUBSCRIPTION_REFRESH_SECS));
    subscription_refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut safety_sync = tokio::time::interval(Duration::from_secs(SAFETY_SYNC_SECS));
    safety_sync.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            line = input_rx.recv() => {
                let Some(line) = line else {
                    break;
                };
                if handle_command(&client, &ledger_path, &mut ledger, &line).await? {
                    break;
                }
            }
            live = client.wait_for_marmot_event(LIVE_WAIT_SECS) => {
                if live {
                    client.drain_pending_marmot().await?;
                    reconcile(&client, &loaded.controllers, &ledger_path, &mut ledger).await?;
                }
            }
            _ = subscription_refresh.tick() => {
                client.ensure_subscriptions().await?;
                reconcile(&client, &loaded.controllers, &ledger_path, &mut ledger).await?;
            }
            _ = safety_sync.tick() => {
                client.sync_force().await?;
                reconcile(&client, &loaded.controllers, &ledger_path, &mut ledger).await?;
            }
        }
    }

    input_task.abort();
    Ok(())
}

async fn reconcile(
    client: &SonarClient,
    controllers: &HashSet<PublicKey>,
    ledger_path: &Path,
    ledger: &mut CommandLedger,
) -> Result<()> {
    accept_pending_invites(client, controllers).await?;
    let joined = checkpoint_new_groups(client, ledger_path, ledger)?;
    emit_joined_groups(&joined)?;
    emit_new_messages(client, ledger_path, ledger)
}

async fn accept_pending_invites(
    client: &SonarClient,
    controllers: &HashSet<PublicKey>,
) -> Result<()> {
    for invite in client.pending_group_invites()? {
        if controllers.contains(&invite.welcomer) {
            client.accept_group_invite(&invite.id).await?;
        }
    }
    Ok(())
}

fn checkpoint_new_groups(
    client: &SonarClient,
    ledger_path: &Path,
    ledger: &mut CommandLedger,
) -> Result<Vec<(String, String)>> {
    let mut joined = Vec::new();
    for group in client.groups()? {
        let group_id = hex::encode(group.mls_group_id.as_slice());
        if ledger.known_groups.insert(group_id.clone()) {
            for message in client.messages(&group.mls_group_id)? {
                ledger.completed.insert(message.id.to_hex());
            }
            joined.push((group_id, group.name));
        }
    }
    if !joined.is_empty() {
        write_private_json(ledger_path, ledger)?;
    }
    Ok(joined)
}

fn emit_joined_groups(groups: &[(String, String)]) -> Result<()> {
    for (group_id, group_name) in groups {
        emit(&BridgeEvent::GroupJoined {
            version: PROTOCOL_VERSION,
            group_id,
            group_name,
        })?;
    }
    Ok(())
}

fn emit_new_messages(
    client: &SonarClient,
    ledger_path: &Path,
    ledger: &mut CommandLedger,
) -> Result<()> {
    for group in client.groups()? {
        let group_id = hex::encode(group.mls_group_id.as_slice());
        if !ledger.known_groups.contains(&group_id) {
            continue;
        }
        let mut messages = client.messages(&group.mls_group_id)?;
        messages.sort_by_key(|message| message.created_at);
        for message in messages {
            let message_id = message.id.to_hex();
            if message.mine
                || ledger.completed.contains(&message_id)
                || ledger.executing.contains_key(&message_id)
            {
                continue;
            }
            if message.content.is_empty() || message.content.len() > MAX_MESSAGE_BYTES {
                ledger.completed.insert(message_id);
                write_private_json(ledger_path, ledger)?;
                continue;
            }

            ledger
                .executing
                .insert(message_id.clone(), group_id.clone());
            write_private_json(ledger_path, ledger)?;
            let sender = message.sender.to_bech32()?;
            emit(&BridgeEvent::Message {
                version: PROTOCOL_VERSION,
                message_id: &message_id,
                group_id: &group_id,
                group_name: &group.name,
                sender: &sender,
                content: &message.content,
            })?;
        }
    }
    Ok(())
}

async fn recover_interrupted_commands(
    client: &SonarClient,
    ledger_path: &Path,
    ledger: &mut CommandLedger,
) {
    let interrupted = std::mem::take(&mut ledger.executing);
    for (message_id, group_id) in interrupted {
        match parse_group_id(&group_id) {
            Ok(group_id) => {
                let _ = client
                    .send_text(
                        &group_id,
                        "A Goose command was interrupted by a gateway restart. It was not replayed; resend it if it is still needed.",
                    )
                    .await;
            }
            Err(error) => warn!(%error, "invalid interrupted-command group id"),
        }
        ledger.completed.insert(message_id);
    }
    if let Err(error) = write_private_json(ledger_path, ledger) {
        warn!(%error, "failed to persist interrupted-command recovery");
    }
}

async fn handle_command(
    client: &SonarClient,
    ledger_path: &Path,
    ledger: &mut CommandLedger,
    line: &str,
) -> Result<bool> {
    if line.len() > MAX_LINE_BYTES {
        warn!(bytes = line.len(), "rejected oversized bridge command");
        return Ok(false);
    }
    let command: BridgeCommand = match serde_json::from_str(line) {
        Ok(command) => command,
        Err(error) => {
            warn!(%error, "rejected malformed bridge command");
            return Ok(false);
        }
    };
    if command.version() != PROTOCOL_VERSION {
        emit_response(
            command.request_id(),
            Err(anyhow!(
                "unsupported bridge protocol version {}",
                command.version()
            )),
        )?;
        return Ok(false);
    }

    match command {
        BridgeCommand::Send {
            request_id,
            group_id,
            text,
            ..
        } => {
            let result = async {
                if text.len() > MAX_MESSAGE_BYTES {
                    return Err(anyhow!("outgoing message exceeds size limit"));
                }
                let group_id = parse_group_id(&group_id)?;
                client.send_text(&group_id, &text).await?;
                Ok(())
            }
            .await;
            emit_response(&request_id, result)?;
            Ok(false)
        }
        BridgeCommand::Complete {
            request_id,
            message_id,
            ..
        } => {
            let result = if ledger.executing.remove(&message_id).is_some() {
                ledger.completed.insert(message_id);
                write_private_json(ledger_path, ledger)
            } else if ledger.completed.contains(&message_id) {
                Ok(())
            } else {
                Err(anyhow!("unknown incoming message id"))
            };
            emit_response(&request_id, result)?;
            Ok(false)
        }
        BridgeCommand::Shutdown { request_id, .. } => {
            emit_response(&request_id, Ok(()))?;
            Ok(true)
        }
    }
}

fn emit_response(request_id: &str, result: Result<()>) -> Result<()> {
    match result {
        Ok(()) => emit(&BridgeEvent::Response {
            version: PROTOCOL_VERSION,
            request_id,
            ok: true,
            error: None,
        }),
        Err(error) => {
            let error = error.to_string();
            emit(&BridgeEvent::Response {
                version: PROTOCOL_VERSION,
                request_id,
                ok: false,
                error: Some(&error),
            })
        }
    }
}

async fn read_commands(sender: mpsc::Sender<String>) {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if sender.send(line).await.is_err() {
                    break;
                }
            }
            Ok(None) => break,
            Err(error) => {
                debug!(%error, "bridge stdin closed with error");
                break;
            }
        }
    }
}

impl LoadedConfig {
    fn load_or_init(
        home: PathBuf,
        relay_overrides: Vec<String>,
        controller_values: Vec<String>,
    ) -> Result<Self> {
        ensure_private_dir(&home)?;
        let config_path = home.join(CONFIG_FILE);
        let mut config = match fs::read(&config_path) {
            Ok(bytes) => serde_json::from_slice::<AgentConfig>(&bytes)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let identity = Identity::generate();
                let config = AgentConfig {
                    version: CONFIG_VERSION,
                    nsec: identity.export_nsec(),
                    db_key_hex: random_hex_32()?,
                    relays: DEFAULT_RELAYS
                        .iter()
                        .map(|relay| relay.to_string())
                        .collect(),
                };
                write_private_json(&config_path, &config)?;
                config
            }
            Err(error) => return Err(error.into()),
        };
        if config.version != CONFIG_VERSION {
            return Err(anyhow!("unsupported config version {}", config.version));
        }
        if !relay_overrides.is_empty() {
            config.relays = relay_overrides;
        }
        let relays = validate_relays(&config.relays)?;
        let controllers = controller_values
            .iter()
            .map(|value| {
                PublicKey::parse(value).with_context(|| format!("invalid controller {value}"))
            })
            .collect::<Result<HashSet<_>>>()?;
        Ok(Self {
            home,
            config,
            relays,
            controllers,
        })
    }

    async fn connect(&self) -> Result<SonarClient> {
        let identity = Identity::import(&self.config.nsec)?;
        let db_key = parse_db_key(&self.config.db_key_hex)?;
        let db_dir = self.home.join(DB_DIR);
        ensure_private_dir(&db_dir)?;
        SonarClient::connect(identity, self.relays.clone(), db_dir.join(DB_FILE), db_key)
            .await
            .map_err(Into::into)
    }

    fn npub(&self) -> Result<String> {
        Ok(Identity::import(&self.config.nsec)?.npub())
    }
}

fn parse_group_id(value: &str) -> Result<GroupId> {
    let bytes = hex::decode(value).context("group id must be hex")?;
    if bytes.is_empty() {
        return Err(anyhow!("group id cannot be empty"));
    }
    Ok(GroupId::from_slice(&bytes))
}

fn parse_db_key(value: &str) -> Result<[u8; 32]> {
    hex::decode(value)?
        .try_into()
        .map_err(|_| anyhow!("database key must be 32 bytes"))
}

fn validate_relays(values: &[String]) -> Result<Vec<RelayUrl>> {
    if values.is_empty() {
        return Err(anyhow!("at least one relay is required"));
    }
    values
        .iter()
        .map(|value| RelayUrl::parse(value).with_context(|| format!("invalid relay {value}")))
        .collect()
}

fn resolve_home(home: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(home) = home {
        return Ok(home);
    }
    if let Ok(home) = env::var("GOOSE_SONAR_HOME") {
        return Ok(PathBuf::from(home));
    }
    if let Ok(data_home) = env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(data_home).join("goose/sonar-gateway"));
    }
    let home = env::var("HOME").context("pass --home or set GOOSE_SONAR_HOME")?;
    Ok(PathBuf::from(home).join(".local/share/goose/sonar-gateway"))
}

fn acquire_instance_lock(home: &Path) -> Result<File> {
    ensure_private_dir(home)?;
    let path = home.join(LOCK_FILE);
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)?;
    FileExt::try_lock_exclusive(&file)
        .with_context(|| format!("another Sonar bridge is using {}", home.display()))?;
    Ok(file)
}

fn load_ledger(path: &Path) -> Result<CommandLedger> {
    match fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(CommandLedger::default()),
        Err(error) => Err(error.into()),
    }
}

fn random_hex_32() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|error| anyhow!("secure random failed: {error}"))?;
    Ok(hex::encode(bytes))
}

fn emit<T: Serialize>(value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > MAX_LINE_BYTES {
        return Err(anyhow!("bridge event exceeds protocol limit"));
    }
    let stdout = io::stdout();
    let mut output = stdout.lock();
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}

fn ensure_private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn write_private_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    let temp = path.with_extension(format!("json.tmp.{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(value)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    fs::rename(temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_protocol_rejects_unknown_versions() {
        let command: BridgeCommand = serde_json::from_str(
            r#"{"type":"complete","version":2,"request_id":"request","message_id":"message"}"#,
        )
        .unwrap();
        assert_eq!(command.version(), 2);
    }

    #[test]
    fn config_initialization_uses_private_files() {
        let temp =
            std::env::temp_dir().join(format!("goose-sonar-bridge-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let loaded = LoadedConfig::load_or_init(temp.clone(), Vec::new(), Vec::new()).unwrap();
        assert_eq!(loaded.config.version, CONFIG_VERSION);
        assert_eq!(loaded.relays.len(), DEFAULT_RELAYS.len());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(temp.join(CONFIG_FILE))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn ledger_round_trips_executing_commands() {
        let temp =
            std::env::temp_dir().join(format!("goose-sonar-ledger-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let path = temp.join(LEDGER_FILE);
        let mut ledger = CommandLedger::default();
        ledger.executing.insert("message".into(), "group".into());
        write_private_json(&path, &ledger).unwrap();
        let loaded = load_ledger(&path).unwrap();
        assert_eq!(loaded.executing.get("message"), Some(&"group".into()));
        fs::remove_dir_all(temp).unwrap();
    }
}
