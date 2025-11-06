pub mod config;
pub mod lapstone;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TunnelState {
    #[default]
    Idle,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunnelPids {
    pub goosed: u32,
    pub tailscale_serve: u32,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunnelConfig {
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStatus {
    pub state: TunnelState,
    pub info: Option<TunnelInfo>,
    pub auto_start: bool,
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
        let auto_start = self.config.read().await.auto_start;
        TunnelStatus {
            state,
            info,
            auto_start,
        }
    }

    pub async fn update_config<F>(&self, f: F)
    where
        F: FnOnce(&mut TunnelConfig),
    {
        let mut cfg = self.config.write().await;
        f(&mut cfg);
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

        let config = self.config.read().await.clone();

        let tunnel_secret = config.secret.clone().unwrap_or_else(generate_secret);
        let server_secret = std::env::var("GOOSE_SERVER__SECRET_KEY")
            .expect("GOOSE_SERVER__SECRET_KEY must be set for tunnel to work");
        let agent_id = config.agent_id.clone().unwrap_or_else(generate_agent_id);

        self.update_config(|c| {
            c.secret = Some(tunnel_secret.clone());
            c.agent_id = Some(agent_id.clone());
        })
        .await;

        let result = lapstone::start(
            port,
            tunnel_secret,
            server_secret,
            agent_id,
            self.lapstone_handle.clone(),
        )
        .await;

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
        lapstone::stop(self.lapstone_handle.clone()).await;

        *self.state.write().await = TunnelState::Idle;
        *self.info.write().await = None;

        if clear_auto_start {
            self.update_config(|c| c.auto_start = false).await;
        }
    }
}

fn generate_secret() -> String {
    let bytes: [u8; 32] = rand::random();
    hex::encode(bytes)
}

fn generate_agent_id() -> String {
    let bytes: [u8; 16] = rand::random();
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
}
