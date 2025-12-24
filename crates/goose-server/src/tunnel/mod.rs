pub mod cgnat;
pub mod lapstone;

#[cfg(test)]
mod lapstone_test;

use crate::configuration::Settings;
use fs2::FileExt as _;
use goose::config::{paths::Paths, Config};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use utoipa::ToSchema;

fn get_server_port() -> anyhow::Result<u16> {
    let settings = Settings::new()?;
    Ok(settings.port)
}

fn get_lock_path() -> std::path::PathBuf {
    Paths::config_dir().join("tunnel.lock")
}

fn try_acquire_tunnel_lock() -> anyhow::Result<File> {
    let lock_path = get_lock_path();

    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&lock_path)?;

    file.try_lock_exclusive()
        .map_err(|_| anyhow::anyhow!("Another goose instance is already running the tunnel"))?;

    writeln!(file, "{}", std::process::id())?;
    file.sync_all()?;

    Ok(file)
}

fn is_tunnel_locked_by_another() -> bool {
    let lock_path = get_lock_path();

    let file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
    {
        Ok(f) => f,
        Err(_) => return false,
    };

    if file.try_lock_exclusive().is_err() {
        return true;
    }

    // Lock released when file is dropped
    false
}

#[derive(Debug, Clone, PartialEq)]
pub enum TunnelMode {
    Disabled,
    Lapstone,
    Cgnat,
}

impl TunnelMode {
    pub fn from_env() -> Self {
        match std::env::var("GOOSE_TUNNEL").ok().as_deref() {
            Some("no") | Some("none") => TunnelMode::Disabled,
            Some("cgnat") => TunnelMode::Cgnat,
            _ => TunnelMode::Lapstone,
        }
    }
}

/// Detect CGNAT IP (100.64.0.0/10 range) from network interfaces.
/// Returns the first 100.x.x.x address found, typically from a VPN tunnel interface.
pub fn detect_cgnat_ip() -> Option<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(r"ifconfig | grep -Eo '100\.[0-9]+\.[0-9]+\.[0-9]+' | head -1")
        .output()
        .ok()?;

    if output.status.success() {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() {
            return Some(ip);
        }
    }
    None
}

fn get_or_create_tunnel_secret() -> String {
    if let Ok(secret) = Config::global().get_secret::<String>("tunnel_secret") {
        return secret;
    }
    let secret = generate_secret();
    let _ = Config::global().set_secret("tunnel_secret", &secret);
    secret
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TunnelState {
    #[default]
    Idle,
    Starting,
    Running,
    Error,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TunnelInfo {
    pub state: TunnelState,
    pub url: String,
    pub hostname: String,
    pub secret: String,
}

pub struct TunnelManager {
    state: Arc<RwLock<TunnelState>>,
    info: Arc<RwLock<Option<TunnelInfo>>>,
    lapstone_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cgnat_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    restart_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
    watchdog_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    lock_file: Arc<std::sync::Mutex<Option<File>>>,
}

impl Default for TunnelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TunnelManager {
    pub fn new() -> Self {
        TunnelManager {
            state: Arc::new(RwLock::new(TunnelState::Idle)),
            info: Arc::new(RwLock::new(None)),
            lapstone_handle: Arc::new(RwLock::new(None)),
            cgnat_handle: Arc::new(RwLock::new(None)),
            restart_tx: Arc::new(RwLock::new(None)),
            watchdog_handle: Arc::new(RwLock::new(None)),
            lock_file: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    fn get_auto_start() -> bool {
        Config::global()
            .get_param("tunnel_auto_start")
            .unwrap_or(false)
    }

    fn get_secret() -> Option<String> {
        Config::global().get_secret("tunnel_secret").ok()
    }

    fn get_agent_id() -> Option<String> {
        Config::global().get_secret("tunnel_agent_id").ok()
    }

    pub async fn check_auto_start(&self) {
        let mode = TunnelMode::from_env();

        if mode == TunnelMode::Disabled {
            return;
        }

        // For CGNAT mode, auto-start the CGNAT proxy
        if mode == TunnelMode::Cgnat {
            if let Some(cgnat_ip) = detect_cgnat_ip() {
                tracing::info!("CGNAT mode: detected IP {}, starting proxy", cgnat_ip);
                match self.start_cgnat(cgnat_ip).await {
                    Ok(info) => {
                        tracing::info!("CGNAT proxy started: {}", info.url);
                    }
                    Err(e) => {
                        tracing::error!("Failed to start CGNAT proxy: {}", e);
                    }
                }
            } else {
                tracing::warn!("CGNAT mode enabled but no 100.x.x.x IP detected");
            }
            return;
        }

        // Lapstone mode
        let auto_start = Self::get_auto_start();
        let state = self.state.read().await.clone();

        if auto_start && state == TunnelState::Idle {
            if is_tunnel_locked_by_another() {
                tracing::info!(
                    "Tunnel already running on another goose instance, skipping auto-start"
                );
                return;
            }

            tracing::info!("Auto-starting tunnel");
            match self.start().await {
                Ok(info) => {
                    tracing::info!("Tunnel auto-started successfully: {:?}", info.url);
                }
                Err(e) => {
                    tracing::info!("Tunnel auto-start skipped: {}", e);
                }
            }
        }
    }

    async fn start_cgnat(&self, cgnat_ip: String) -> anyhow::Result<TunnelInfo> {
        let server_port = get_server_port()?;
        let tunnel_secret = get_or_create_tunnel_secret();
        let server_secret =
            std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());

        let (info, handle) =
            cgnat::start(server_port, tunnel_secret, server_secret, cgnat_ip).await?;

        *self.cgnat_handle.write().await = Some(handle);
        *self.state.write().await = TunnelState::Running;
        *self.info.write().await = Some(info.clone());

        Ok(info)
    }

    fn is_tunnel_disabled() -> bool {
        TunnelMode::from_env() == TunnelMode::Disabled
    }

    pub async fn get_info(&self) -> TunnelInfo {
        let mode = TunnelMode::from_env();

        if mode == TunnelMode::Disabled {
            return TunnelInfo {
                state: TunnelState::Disabled,
                url: String::new(),
                hostname: String::new(),
                secret: String::new(),
            };
        }

        let state = self.state.read().await.clone();
        let info = self.info.read().await.clone();

        match info {
            Some(mut tunnel_info) => {
                tunnel_info.state = state;
                tunnel_info
            }
            None => {
                let effective_state = if state == TunnelState::Idle && is_tunnel_locked_by_another()
                {
                    TunnelState::Running
                } else {
                    state
                };
                TunnelInfo {
                    state: effective_state,
                    url: String::new(),
                    hostname: String::new(),
                    secret: String::new(),
                }
            }
        }
    }

    pub fn set_auto_start(auto_start: bool) -> anyhow::Result<()> {
        Config::global()
            .set_param("tunnel_auto_start", auto_start)
            .map_err(|e| anyhow::anyhow!("Failed to save tunnel config: {}", e))
    }

    pub fn set_secret(secret: &str) -> anyhow::Result<()> {
        Config::global()
            .set_secret("tunnel_secret", &secret.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to save tunnel secret: {}", e))
    }

    pub fn set_agent_id(agent_id: &str) -> anyhow::Result<()> {
        Config::global()
            .set_secret("tunnel_agent_id", &agent_id.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to save tunnel agent_id: {}", e))
    }

    async fn start_tunnel_internal(&self) -> anyhow::Result<(TunnelInfo, mpsc::Receiver<()>)> {
        let server_port = get_server_port()?;
        let tunnel_secret = Self::get_secret().unwrap_or_else(generate_secret);
        let server_secret =
            std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());
        let agent_id = Self::get_agent_id().unwrap_or_else(generate_agent_id);

        Self::set_secret(&tunnel_secret)?;
        Self::set_agent_id(&agent_id)?;

        let (restart_tx, restart_rx) = mpsc::channel::<()>(1);
        *self.restart_tx.write().await = Some(restart_tx.clone());

        let result = lapstone::start(
            server_port,
            tunnel_secret,
            server_secret,
            agent_id,
            self.lapstone_handle.clone(),
            restart_tx,
        )
        .await;

        match result {
            Ok(info) => Ok((info, restart_rx)),
            Err(e) => Err(e),
        }
    }

    pub async fn start(&self) -> anyhow::Result<TunnelInfo> {
        if Self::is_tunnel_disabled() {
            anyhow::bail!("Tunnel is disabled via GOOSE_TUNNEL environment variable");
        }

        // In CGNAT mode, start the CGNAT proxy if not already running
        if TunnelMode::from_env() == TunnelMode::Cgnat {
            let info = self.info.read().await.clone();
            if let Some(info) = info {
                return Ok(info);
            }
            // Try to start CGNAT proxy
            if let Some(cgnat_ip) = detect_cgnat_ip() {
                return self.start_cgnat(cgnat_ip).await;
            }
            anyhow::bail!("CGNAT mode requires a 100.x.x.x IP to be detected");
        }

        let mut state = self.state.write().await;
        if *state != TunnelState::Idle {
            anyhow::bail!("Tunnel is already running or starting");
        }

        let lock = try_acquire_tunnel_lock()?;
        *self.lock_file.lock().unwrap() = Some(lock);

        *state = TunnelState::Starting;
        drop(state);

        match self.start_tunnel_internal().await {
            Ok((info, mut restart_rx)) => {
                *self.state.write().await = TunnelState::Running;
                *self.info.write().await = Some(info.clone());
                let _ = Self::set_auto_start(true);

                let state = self.state.clone();
                let lapstone_handle = self.lapstone_handle.clone();
                let watchdog_handle_arc = self.watchdog_handle.clone();
                let manager = Arc::new(self.clone_for_watchdog());

                let watchdog = tokio::spawn(async move {
                    while restart_rx.recv().await.is_some() {
                        let auto_start = Self::get_auto_start();
                        if !auto_start {
                            tracing::info!("Tunnel connection lost but auto_start is disabled");
                            break;
                        }

                        tracing::warn!("Tunnel connection lost, initiating restart...");
                        lapstone::stop(lapstone_handle.clone()).await;
                        *state.write().await = TunnelState::Idle;
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        *state.write().await = TunnelState::Starting;

                        match manager.start_tunnel_internal().await {
                            Ok((_, new_restart_rx)) => {
                                *state.write().await = TunnelState::Running;
                                tracing::info!("Tunnel restarted successfully");
                                restart_rx = new_restart_rx;
                            }
                            Err(e) => {
                                tracing::error!("Failed to restart tunnel: {}", e);
                                *state.write().await = TunnelState::Error;
                                break;
                            }
                        }
                    }
                });

                *watchdog_handle_arc.write().await = Some(watchdog);

                Ok(info)
            }
            Err(e) => {
                self.release_lock();
                *self.state.write().await = TunnelState::Error;
                Err(e)
            }
        }
    }

    fn clone_for_watchdog(&self) -> Self {
        TunnelManager {
            state: self.state.clone(),
            info: self.info.clone(),
            lapstone_handle: self.lapstone_handle.clone(),
            cgnat_handle: self.cgnat_handle.clone(),
            restart_tx: self.restart_tx.clone(),
            watchdog_handle: self.watchdog_handle.clone(),
            lock_file: self.lock_file.clone(),
        }
    }

    fn release_lock(&self) {
        if let Ok(mut guard) = self.lock_file.lock() {
            // Dropping the file releases the lock
            guard.take();
        }
    }

    pub async fn stop(&self, clear_auto_start: bool) {
        if let Some(handle) = self.watchdog_handle.write().await.take() {
            handle.abort();
        }

        *self.restart_tx.write().await = None;

        lapstone::stop(self.lapstone_handle.clone()).await;

        // Stop CGNAT proxy if running
        if let Some(handle) = self.cgnat_handle.write().await.take() {
            handle.abort();
        }

        self.release_lock();

        *self.state.write().await = TunnelState::Idle;
        *self.info.write().await = None;

        if clear_auto_start {
            let _ = Self::set_auto_start(false);
        }
    }
}

fn generate_secret() -> String {
    let bytes: [u8; 32] = rand::random();
    hex::encode(bytes)
}

pub(super) fn generate_agent_id() -> String {
    let bytes: [u8; 32] = rand::random();
    hex::encode(bytes)
}
