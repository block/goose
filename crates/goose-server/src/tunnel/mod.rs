pub mod lapstone;

#[cfg(test)]
mod lapstone_test;

use crate::configuration::Settings;
use goose::config::Config;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use utoipa::ToSchema;

fn get_server_port() -> anyhow::Result<u16> {
    let settings = Settings::new()?;
    Ok(settings.port)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TunnelState {
    #[default]
    Idle,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TunnelInfo {
    pub state: TunnelState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
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

pub struct TunnelManager {
    state: Arc<RwLock<TunnelState>>,
    info: Arc<RwLock<Option<TunnelInfo>>>,
    config: Arc<RwLock<TunnelConfig>>,
    lapstone_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    restart_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
    watchdog_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
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
            config: Arc::new(RwLock::new(TunnelConfig::default())),
            lapstone_handle: Arc::new(RwLock::new(None)),
            restart_tx: Arc::new(RwLock::new(None)),
            watchdog_handle: Arc::new(RwLock::new(None)),
        }
    }

    async fn load_config(&self) {
        let cfg = Config::global();
        let auto_start = cfg.get_param("tunnel_auto_start").unwrap_or(false);
        let secret = cfg.get_secret("tunnel_secret").ok();
        let agent_id = cfg.get_secret("tunnel_agent_id").ok();

        *self.config.write().await = TunnelConfig {
            auto_start,
            secret,
            agent_id,
        };
    }

    pub async fn check_auto_start(&self) {
        self.load_config().await;

        let auto_start = self.config.read().await.auto_start;
        let state = self.state.read().await.clone();

        if auto_start && state == TunnelState::Idle {
            tracing::info!("Auto-starting tunnel");
            match self.start().await {
                Ok(info) => {
                    tracing::info!("Tunnel auto-started successfully: {:?}", info.url);
                }
                Err(e) => {
                    tracing::error!("Failed to auto-start tunnel: {}", e);
                }
            }
        }
    }

    pub async fn get_info(&self) -> TunnelInfo {
        let state = self.state.read().await.clone();
        let info = self.info.read().await.clone();

        match info {
            Some(mut tunnel_info) => {
                tunnel_info.state = state;
                tunnel_info
            }
            None => TunnelInfo {
                state,
                url: None,
                hostname: None,
                secret: None,
            },
        }
    }

    pub async fn update_config<F>(&self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut TunnelConfig),
    {
        let mut cfg = self.config.write().await;
        f(&mut cfg);

        let global_cfg = Config::global();
        global_cfg
            .set_param("tunnel_auto_start", cfg.auto_start)
            .map_err(|e| anyhow::anyhow!("Failed to save tunnel config: {}", e))?;

        if let Some(secret) = &cfg.secret {
            global_cfg
                .set_secret("tunnel_secret", secret)
                .map_err(|e| anyhow::anyhow!("Failed to save tunnel secret: {}", e))?;
        }
        if let Some(agent_id) = &cfg.agent_id {
            global_cfg
                .set_secret("tunnel_agent_id", agent_id)
                .map_err(|e| anyhow::anyhow!("Failed to save tunnel agent_id: {}", e))?;
        }
        Ok(())
    }

    async fn start_tunnel_internal(&self) -> anyhow::Result<(TunnelInfo, mpsc::Receiver<()>)> {
        let config = self.config.read().await.clone();
        let server_port = get_server_port()?;

        let tunnel_secret = config.secret.clone().unwrap_or_else(generate_secret);
        let server_secret =
            std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());
        let agent_id = config.agent_id.clone().unwrap_or_else(generate_agent_id);

        self.update_config(|c| {
            c.secret = Some(tunnel_secret.clone());
            c.agent_id = Some(agent_id.clone());
        })
        .await?;

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
        if let Ok(val) = std::env::var("GOOSE_TUNNEL") {
            let val = val.to_lowercase();
            if val == "no" || val == "none" {
                anyhow::bail!("Tunnel is disabled via GOOSE_TUNNEL environment variable");
            }
        }

        let mut state = self.state.write().await;
        if *state != TunnelState::Idle {
            anyhow::bail!("Tunnel is already running or starting");
        }
        *state = TunnelState::Starting;
        drop(state);

        match self.start_tunnel_internal().await {
            Ok((info, mut restart_rx)) => {
                *self.state.write().await = TunnelState::Running;
                *self.info.write().await = Some(info.clone());
                let _ = self.update_config(|c| c.auto_start = true).await;

                let state = self.state.clone();
                let config = self.config.clone();
                let lapstone_handle = self.lapstone_handle.clone();
                let watchdog_handle_arc = self.watchdog_handle.clone();
                let manager = Arc::new(self.clone_for_watchdog());

                let watchdog = tokio::spawn(async move {
                    while restart_rx.recv().await.is_some() {
                        let auto_start = config.read().await.auto_start;
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
                *self.state.write().await = TunnelState::Error;
                Err(e)
            }
        }
    }

    fn clone_for_watchdog(&self) -> Self {
        TunnelManager {
            state: self.state.clone(),
            info: self.info.clone(),
            config: self.config.clone(),
            lapstone_handle: self.lapstone_handle.clone(),
            restart_tx: self.restart_tx.clone(),
            watchdog_handle: self.watchdog_handle.clone(),
        }
    }

    pub async fn stop(&self, clear_auto_start: bool) {
        if let Some(handle) = self.watchdog_handle.write().await.take() {
            handle.abort();
        }

        *self.restart_tx.write().await = None;

        lapstone::stop(self.lapstone_handle.clone()).await;

        *self.state.write().await = TunnelState::Idle;
        *self.info.write().await = None;

        if clear_auto_start {
            let _ = self.update_config(|c| c.auto_start = false).await;
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
