use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use utoipa::ToSchema;

use goose::config::paths::Paths;
use goose::execution::manager::AgentManager;

use super::handler::GatewayHandler;
use super::pairing::PairingStore;
use super::{Gateway, GatewayConfig, PairingState, PlatformUser};

pub struct GatewayInstance {
    pub config: GatewayConfig,
    pub gateway: Arc<dyn Gateway>,
    pub cancel: CancellationToken,
    pub handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PairedUserInfo {
    pub platform: String,
    pub user_id: String,
    pub display_name: Option<String>,
    pub session_id: String,
    pub paired_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GatewayStatus {
    pub gateway_type: String,
    pub running: bool,
    pub paired_users: Vec<PairedUserInfo>,
}

pub struct GatewayManager {
    gateways: RwLock<HashMap<String, GatewayInstance>>,
    pairing_store: Arc<PairingStore>,
    agent_manager: Arc<AgentManager>,
}

impl GatewayManager {
    pub fn new(agent_manager: Arc<AgentManager>) -> Self {
        let db_path = Paths::data_dir().join("gateway").join("pairings.db");
        let pairing_store = Arc::new(PairingStore::new(&db_path));

        Self {
            gateways: RwLock::new(HashMap::new()),
            pairing_store,
            agent_manager,
        }
    }

    pub fn pairing_store(&self) -> &Arc<PairingStore> {
        &self.pairing_store
    }

    pub async fn start_gateway(
        &self,
        config: GatewayConfig,
        gateway: Arc<dyn Gateway>,
    ) -> anyhow::Result<()> {
        let gw_type = config.gateway_type.clone();

        if self.gateways.read().await.contains_key(&gw_type) {
            anyhow::bail!("Gateway '{}' is already running", gw_type);
        }

        gateway.validate_config().await?;

        let cancel = CancellationToken::new();
        let handler = GatewayHandler::new(
            self.agent_manager.clone(),
            self.pairing_store.clone(),
            gateway.clone(),
            config.clone(),
        );

        let gateway_clone = gateway.clone();
        let cancel_clone = cancel.clone();
        let gateway_type_for_task = gw_type.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = gateway_clone.start(handler, cancel_clone).await {
                tracing::error!(gateway = %gateway_type_for_task, error = %e, "gateway stopped with error");
            }
        });

        let instance = GatewayInstance {
            config,
            gateway,
            cancel,
            handle,
        };

        self.gateways.write().await.insert(gw_type, instance);

        Ok(())
    }

    pub async fn stop_gateway(&self, gateway_type: &str) -> anyhow::Result<()> {
        let instance = self
            .gateways
            .write()
            .await
            .remove(gateway_type)
            .ok_or_else(|| anyhow::anyhow!("Gateway '{}' is not running", gateway_type))?;

        instance.cancel.cancel();
        let _ = instance.handle.await;
        tracing::info!(gateway = %gateway_type, "gateway stopped");
        Ok(())
    }

    pub async fn stop_all(&self) {
        let instances: Vec<(String, GatewayInstance)> =
            self.gateways.write().await.drain().collect();
        for (gateway_type, instance) in instances {
            instance.cancel.cancel();
            let _ = instance.handle.await;
            tracing::info!(gateway = %gateway_type, "gateway stopped");
        }
    }

    pub async fn is_running(&self, gateway_type: &str) -> bool {
        self.gateways.read().await.contains_key(gateway_type)
    }

    pub async fn list_running(&self) -> Vec<String> {
        self.gateways.read().await.keys().cloned().collect()
    }

    pub async fn status(&self) -> Vec<GatewayStatus> {
        let running = self.gateways.read().await;
        let mut statuses = Vec::new();

        for (gw_type, _instance) in running.iter() {
            let paired_users = self
                .pairing_store
                .list_paired_users(gw_type)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|(user, session_id, paired_at)| PairedUserInfo {
                    platform: user.platform,
                    user_id: user.user_id,
                    display_name: user.display_name,
                    session_id,
                    paired_at,
                })
                .collect();

            statuses.push(GatewayStatus {
                gateway_type: gw_type.clone(),
                running: true,
                paired_users,
            });
        }

        statuses.sort_by(|a, b| a.gateway_type.cmp(&b.gateway_type));
        statuses
    }

    pub async fn unpair_user(&self, platform: &str, user_id: &str) -> anyhow::Result<bool> {
        let user = PlatformUser {
            platform: platform.to_string(),
            user_id: user_id.to_string(),
            display_name: None,
        };
        let state = self.pairing_store.get(&user).await?;
        if matches!(state, PairingState::Paired { .. }) {
            self.pairing_store.remove(&user).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn generate_pairing_code(&self, gateway_type: &str) -> anyhow::Result<(String, i64)> {
        let code = PairingStore::generate_code();
        let expires_at = chrono::Utc::now().timestamp() + 300;
        self.pairing_store
            .store_pending_code(&code, gateway_type, expires_at)
            .await?;
        Ok((code, expires_at))
    }
}
