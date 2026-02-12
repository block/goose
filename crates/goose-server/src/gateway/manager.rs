use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use goose::config::paths::Paths;
use goose::execution::manager::AgentManager;

use super::handler::GatewayHandler;
use super::pairing::PairingStore;
use super::{Gateway, GatewayConfig, PlatformUser};

pub struct GatewayInstance {
    pub config: GatewayConfig,
    pub gateway: Arc<dyn Gateway>,
    pub cancel: CancellationToken,
    pub handle: tokio::task::JoinHandle<()>,
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
        gateway.validate_config().await?;

        let cancel = CancellationToken::new();
        let handler = GatewayHandler::new(
            self.agent_manager.clone(),
            self.pairing_store.clone(),
            config.clone(),
        );

        let gateway_clone = gateway.clone();
        let cancel_clone = cancel.clone();
        let gateway_type = config.gateway_type.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = gateway_clone.start(handler, cancel_clone).await {
                tracing::error!(gateway = %gateway_type, error = %e, "gateway stopped with error");
            }
        });

        let instance = GatewayInstance {
            config: config.clone(),
            gateway,
            cancel,
            handle,
        };

        self.gateways
            .write()
            .await
            .insert(config.gateway_type.clone(), instance);

        Ok(())
    }

    pub async fn stop_gateway(&self, gateway_type: &str) -> anyhow::Result<()> {
        let instance = self.gateways.write().await.remove(gateway_type);
        if let Some(instance) = instance {
            instance.cancel.cancel();
            let _ = instance.handle.await;
            tracing::info!(gateway = %gateway_type, "gateway stopped");
        }
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

    pub async fn list_paired_users(
        &self,
        gateway_type: &str,
    ) -> anyhow::Result<Vec<(PlatformUser, String)>> {
        self.pairing_store.list_paired_users(gateway_type).await
    }

    pub async fn generate_pairing_code(&self, gateway_type: &str) -> anyhow::Result<(String, i64)> {
        let code = PairingStore::generate_code();
        let expires_at = chrono::Utc::now().timestamp() + 300; // 5 minutes
        self.pairing_store
            .store_pending_code(&code, gateway_type, expires_at)
            .await?;
        Ok((code, expires_at))
    }
}
