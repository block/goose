use axum::http::StatusCode;
use goose::builtin_extension::register_builtin_extensions;
use goose::execution::manager::AgentManager;
use goose::l402::L402PaymentHandler;
use goose::scheduler_trait::SchedulerTrait;
use goose::session::SessionManager;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::tunnel::TunnelManager;
use crate::wallet::WalletManager;
use goose::agents::ExtensionLoadResult;
use goose::gateway::manager::GatewayManager;
use goose::providers::local_inference::InferenceRuntime;

/// L402 payment handler that delegates to the WalletManager.
struct WalletL402Handler {
    wallet: Arc<WalletManager>,
}

#[async_trait::async_trait]
impl L402PaymentHandler for WalletL402Handler {
    async fn pay_invoice(&self, bolt11: &str) -> anyhow::Result<String> {
        let (_, preimage_hex) = self.wallet.pay_invoice(bolt11).await?;
        Ok(preimage_hex)
    }
}

type ExtensionLoadingTasks =
    Arc<Mutex<HashMap<String, Arc<Mutex<Option<JoinHandle<Vec<ExtensionLoadResult>>>>>>>>;

#[derive(Clone)]
pub struct AppState {
    pub(crate) agent_manager: Arc<AgentManager>,
    pub recipe_file_hash_map: Arc<Mutex<HashMap<String, PathBuf>>>,
    recipe_session_tracker: Arc<Mutex<HashSet<String>>>,
    pub tunnel_manager: Arc<TunnelManager>,
    pub gateway_manager: Arc<GatewayManager>,
    pub extension_loading_tasks: ExtensionLoadingTasks,
    pub inference_runtime: Arc<InferenceRuntime>,
    pub wallet_manager: Arc<WalletManager>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Arc<AppState>> {
        register_builtin_extensions(goose_mcp::BUILTIN_EXTENSIONS.clone());

        let agent_manager = AgentManager::instance().await?;
        let tunnel_manager = Arc::new(TunnelManager::new());
        let gateway_manager = Arc::new(GatewayManager::new(agent_manager.clone())?);
        let wallet_manager = Arc::new(WalletManager::new());

        // Register the L402 payment handler so ApiClient can auto-pay L402 challenges.
        let l402_handler = Arc::new(WalletL402Handler {
            wallet: Arc::clone(&wallet_manager),
        });
        let _ = goose::l402::set_l402_handler(l402_handler);

        // Register the L402 pay callback for MCP tools (developer server's pay_l402_invoice tool).
        let wallet_for_mcp = Arc::clone(&wallet_manager);
        goose_mcp::l402::set_l402_pay_fn(std::sync::Arc::new(move |bolt11: String| {
            let w = Arc::clone(&wallet_for_mcp);
            Box::pin(async move {
                let (_, preimage) = w.pay_invoice(&bolt11).await?;
                Ok(preimage)
            })
        }));

        Ok(Arc::new(Self {
            agent_manager,
            recipe_file_hash_map: Arc::new(Mutex::new(HashMap::new())),
            recipe_session_tracker: Arc::new(Mutex::new(HashSet::new())),
            tunnel_manager,
            gateway_manager,
            extension_loading_tasks: Arc::new(Mutex::new(HashMap::new())),
            inference_runtime: InferenceRuntime::get_or_init(),
            wallet_manager,
        }))
    }

    pub async fn set_extension_loading_task(
        &self,
        session_id: String,
        task: JoinHandle<Vec<ExtensionLoadResult>>,
    ) {
        let mut tasks = self.extension_loading_tasks.lock().await;
        tasks.insert(session_id, Arc::new(Mutex::new(Some(task))));
    }

    pub async fn take_extension_loading_task(
        &self,
        session_id: &str,
    ) -> Option<Vec<ExtensionLoadResult>> {
        let task_holder = {
            let tasks = self.extension_loading_tasks.lock().await;
            tasks.get(session_id).cloned()
        };

        if let Some(holder) = task_holder {
            let task = holder.lock().await.take();
            if let Some(handle) = task {
                match handle.await {
                    Ok(results) => return Some(results),
                    Err(e) => {
                        tracing::warn!("Background extension loading task failed: {}", e);
                    }
                }
            }
        }
        None
    }

    pub async fn remove_extension_loading_task(&self, session_id: &str) {
        let mut tasks = self.extension_loading_tasks.lock().await;
        tasks.remove(session_id);
    }

    pub fn scheduler(&self) -> Arc<dyn SchedulerTrait> {
        self.agent_manager.scheduler()
    }

    pub fn session_manager(&self) -> &SessionManager {
        self.agent_manager.session_manager()
    }

    pub async fn set_recipe_file_hash_map(&self, hash_map: HashMap<String, PathBuf>) {
        let mut map = self.recipe_file_hash_map.lock().await;
        *map = hash_map;
    }

    pub async fn mark_recipe_run_if_absent(&self, session_id: &str) -> bool {
        let mut sessions = self.recipe_session_tracker.lock().await;
        if sessions.contains(session_id) {
            false
        } else {
            sessions.insert(session_id.to_string());
            true
        }
    }

    pub async fn get_agent(&self, session_id: String) -> anyhow::Result<Arc<goose::agents::Agent>> {
        self.agent_manager.get_or_create_agent(session_id).await
    }

    pub async fn get_agent_for_route(
        &self,
        session_id: String,
    ) -> Result<Arc<goose::agents::Agent>, StatusCode> {
        self.get_agent(session_id).await.map_err(|e| {
            tracing::error!("Failed to get agent: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
    }
}
