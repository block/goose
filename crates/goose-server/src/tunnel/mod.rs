pub mod config;
pub mod lapstone;

use crate::configuration::Settings;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

fn get_server_port() -> anyhow::Result<u16> {
    let settings = Settings::new()?;
    Ok(settings.port)
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInfo {
    pub url: String,
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
    config: Arc<RwLock<Option<TunnelConfig>>>,
    config_initialized: Arc<RwLock<bool>>,
    lapstone_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    restart_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
    watchdog_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl TunnelManager {
    pub fn new(config: TunnelConfig) -> Self {
        TunnelManager {
            state: Arc::new(RwLock::new(TunnelState::Idle)),
            info: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(Some(config))),
            config_initialized: Arc::new(RwLock::new(true)),
            lapstone_handle: Arc::new(RwLock::new(None)),
            restart_tx: Arc::new(RwLock::new(None)),
            watchdog_handle: Arc::new(RwLock::new(None)),
        }
    }

    pub fn new_uninitialized() -> Self {
        TunnelManager {
            state: Arc::new(RwLock::new(TunnelState::Idle)),
            info: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(None)),
            config_initialized: Arc::new(RwLock::new(false)),
            lapstone_handle: Arc::new(RwLock::new(None)),
            restart_tx: Arc::new(RwLock::new(None)),
            watchdog_handle: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn ensure_initialized(&self) {
        let initialized = *self.config_initialized.read().await;
        if !initialized {
            let loaded_config = config::load_config().await;
            *self.config.write().await = Some(loaded_config);
            *self.config_initialized.write().await = true;
        }
    }

    pub async fn get_status(&self) -> TunnelStatus {
        self.ensure_initialized().await;
        let state = self.state.read().await.clone();
        let info = self.info.read().await.clone();
        let auto_start = self
            .config
            .read()
            .await
            .as_ref()
            .map(|c| c.auto_start)
            .unwrap_or(false);
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
        self.ensure_initialized().await;
        let mut cfg_opt = self.config.write().await;
        if let Some(cfg) = cfg_opt.as_mut() {
            f(cfg);
            if let Err(e) = config::save_config(cfg).await {
                tracing::error!("Failed to save tunnel config: {}", e);
            }
        }
    }

    pub async fn start(&self) -> anyhow::Result<TunnelInfo> {
        self.ensure_initialized().await;

        let mut state = self.state.write().await;
        if *state != TunnelState::Idle {
            anyhow::bail!("Tunnel is already running or starting");
        }
        *state = TunnelState::Starting;
        drop(state);

        let config = self.config.read().await.clone().unwrap_or_default();
        let server_port = get_server_port()?;

        let tunnel_secret = config.secret.clone().unwrap_or_else(generate_secret);
        let server_secret =
            std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());
        let agent_id = config.agent_id.clone().unwrap_or_else(generate_agent_id);

        self.update_config(|c| {
            c.secret = Some(tunnel_secret.clone());
            c.agent_id = Some(agent_id.clone());
        })
        .await;

        let (restart_tx, mut restart_rx) = mpsc::channel::<()>(1);
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
            Ok(info) => {
                *self.state.write().await = TunnelState::Running;
                *self.info.write().await = Some(info.clone());
                self.update_config(|c| c.auto_start = true).await;

                let state = self.state.clone();
                let config = self.config.clone();
                let lapstone_handle = self.lapstone_handle.clone();
                let restart_tx_clone = self.restart_tx.clone();
                let watchdog_handle_arc = self.watchdog_handle.clone();

                let watchdog = tokio::spawn(async move {
                    while restart_rx.recv().await.is_some() {
                        let auto_start = config
                            .read()
                            .await
                            .as_ref()
                            .map(|c| c.auto_start)
                            .unwrap_or(false);
                        if auto_start {
                            tracing::warn!("Tunnel connection lost, initiating restart...");

                            lapstone::stop(lapstone_handle.clone()).await;
                            *state.write().await = TunnelState::Idle;

                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                            *state.write().await = TunnelState::Starting;
                            let cfg = config.read().await.clone().unwrap_or_default();
                            let tunnel_secret = cfg.secret.clone().unwrap_or_else(generate_secret);
                            let server_secret = std::env::var("GOOSE_SERVER__SECRET_KEY")
                                .unwrap_or_else(|_| "test".to_string());
                            let agent_id = cfg.agent_id.clone().unwrap_or_else(generate_agent_id);

                            let (new_restart_tx, new_restart_rx) = mpsc::channel::<()>(1);
                            *restart_tx_clone.write().await = Some(new_restart_tx.clone());

                            match lapstone::start(
                                server_port,
                                tunnel_secret,
                                server_secret,
                                agent_id,
                                lapstone_handle.clone(),
                                new_restart_tx,
                            )
                            .await
                            {
                                Ok(_) => {
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
                        } else {
                            tracing::info!("Tunnel connection lost but auto_start is disabled");
                            break;
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

    pub async fn stop(&self, clear_auto_start: bool) {
        if let Some(handle) = self.watchdog_handle.write().await.take() {
            handle.abort();
        }

        *self.restart_tx.write().await = None;

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
