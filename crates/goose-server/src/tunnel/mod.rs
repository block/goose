pub mod config;
pub mod lapstone;
pub mod tailscale;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TunnelMode {
    Lapstone,
    Tailscale,
}

impl Default for TunnelMode {
    fn default() -> Self {
        TunnelMode::Lapstone
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TunnelState {
    Idle,
    Starting,
    Running,
    Error,
}

impl Default for TunnelState {
    fn default() -> Self {
        TunnelState::Idle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelPids {
    pub goosed: u32,
    pub tailscale_serve: u32,
}

impl Default for TunnelPids {
    fn default() -> Self {
        TunnelPids {
            goosed: 0,
            tailscale_serve: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInfo {
    pub url: String,
    pub ipv4: String,
    pub ipv6: String,
    pub hostname: String,
    pub secret: String,
    pub port: u16,
    pub pids: TunnelPids,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub mode: TunnelMode,
    pub auto_start: bool,
    pub secret: Option<String>,
    pub agent_id: Option<String>,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        TunnelConfig {
            mode: TunnelMode::default(),
            auto_start: false,
            secret: None,
            agent_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStatus {
    pub state: TunnelState,
    pub info: Option<TunnelInfo>,
}

pub struct TunnelManager {
    state: Arc<RwLock<TunnelState>>,
    info: Arc<RwLock<Option<TunnelInfo>>>,
    config: Arc<RwLock<TunnelConfig>>,
    lapstone_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl TunnelManager {
    pub fn new(config: TunnelConfig) -> Self {
        TunnelManager {
            state: Arc::new(RwLock::new(TunnelState::Idle)),
            info: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(config)),
            lapstone_handle: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_status(&self) -> TunnelStatus {
        let state = self.state.read().await.clone();
        let info = self.info.read().await.clone();
        TunnelStatus { state, info }
    }

    pub async fn get_mode(&self) -> TunnelMode {
        self.config.read().await.mode.clone()
    }

    pub async fn set_mode(&self, mode: TunnelMode) {
        self.config.write().await.mode = mode;
    }

    pub async fn update_config<F>(&self, f: F)
    where
        F: FnOnce(&mut TunnelConfig),
    {
        let mut cfg = self.config.write().await;
        f(&mut cfg);
        // Save config via goose Config system (keyring + config.yaml)
        if let Err(e) = config::save_config(&cfg).await {
            tracing::error!("Failed to save tunnel config: {}", e);
        }
    }

    pub async fn start(&self, port: u16) -> anyhow::Result<TunnelInfo> {
        let mut state = self.state.write().await;
        if *state != TunnelState::Idle {
            anyhow::bail!("Tunnel is already running or starting");
        }
        *state = TunnelState::Starting;
        drop(state);

        let mode = self.get_mode().await;
        let config = self.config.read().await.clone();

        let result = match mode {
            TunnelMode::Lapstone => {
                // Use the server secret from env var (same secret that authenticates API requests)
                // This ensures the QR code secret matches what goosed expects
                let secret = std::env::var("GOOSE_SERVER__SECRET_KEY")
                    .unwrap_or_else(|_| {
                        config.secret.clone().unwrap_or_else(|| generate_secret())
                    });
                    
                let agent_id = config
                    .agent_id
                    .clone()
                    .unwrap_or_else(|| generate_agent_id());

                // Update config with values (but don't persist server secret to disk for security)
                self.update_config(|c| {
                    c.agent_id = Some(agent_id.clone());
                })
                .await;

                let info = lapstone::start(port, secret, agent_id, self.lapstone_handle.clone()).await?;
                Ok(info)
            }
            TunnelMode::Tailscale => {
                // Use server secret for tailscale too
                let secret = std::env::var("GOOSE_SERVER__SECRET_KEY")
                    .unwrap_or_else(|_| {
                        config.secret.clone().unwrap_or_else(|| generate_secret())
                    });

                tailscale::start(port, secret).await
            }
        };

        match result {
            Ok(info) => {
                *self.state.write().await = TunnelState::Running;
                *self.info.write().await = Some(info.clone());
                self.update_config(|c| c.auto_start = true).await;
                Ok(info)
            }
            Err(e) => {
                *self.state.write().await = TunnelState::Error;
                Err(e)
            }
        }
    }

    pub async fn stop(&self, clear_auto_start: bool) {
        let mode = self.get_mode().await;

        match mode {
            TunnelMode::Lapstone => {
                lapstone::stop(self.lapstone_handle.clone()).await;
            }
            TunnelMode::Tailscale => {
                tailscale::stop().await;
            }
        }

        *self.state.write().await = TunnelState::Idle;
        *self.info.write().await = None;

        if clear_auto_start {
            self.update_config(|c| c.auto_start = false).await;
        }
    }
}

fn generate_secret() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..32).map(|_| rand::Rng::random(&mut rng)).collect();
    hex::encode(bytes)
}

fn generate_agent_id() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..16).map(|_| rand::Rng::random(&mut rng)).collect();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let secret = generate_secret();
        assert_eq!(secret.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_generate_agent_id() {
        let agent_id = generate_agent_id();
        assert_eq!(agent_id.len(), 32); // 16 bytes = 32 hex chars
    }

    #[tokio::test]
    async fn test_tunnel_manager_initial_state() {
        let manager = TunnelManager::new(TunnelConfig::default());
        let status = manager.get_status().await;
        assert_eq!(status.state, TunnelState::Idle);
        assert!(status.info.is_none());
    }

    #[tokio::test]
    async fn test_tunnel_mode() {
        let manager = TunnelManager::new(TunnelConfig::default());
        assert_eq!(manager.get_mode().await, TunnelMode::Lapstone);

        manager.set_mode(TunnelMode::Tailscale).await;
        assert_eq!(manager.get_mode().await, TunnelMode::Tailscale);
    }
}
