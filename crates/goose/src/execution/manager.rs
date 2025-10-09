use crate::agents::extension::PlatformExtensionContext;
use crate::agents::Agent;
use crate::config::paths::Paths;
use crate::model::ModelConfig;
use crate::providers::create;
use crate::scheduler_factory::SchedulerFactory;
use crate::scheduler_trait::SchedulerTrait;
use anyhow::Result;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};
use tracing::{debug, info, warn};

const DEFAULT_MAX_SESSION: usize = 100;

static AGENT_MANAGER: OnceCell<Arc<AgentManager>> = OnceCell::const_new();

pub struct AgentManager {
    sessions: Arc<RwLock<LruCache<String, Arc<Agent>>>>,
    scheduler: Arc<dyn SchedulerTrait>,
    default_provider: Arc<RwLock<Option<Arc<dyn crate::providers::base::Provider>>>>,
}

impl AgentManager {
    /// Reset the global singleton - ONLY for testing
    pub fn reset_for_test() {
        unsafe {
            // Cast away the const to get mutable access
            // This is safe in test context where we control execution with #[serial]
            let cell_ptr = &AGENT_MANAGER as *const OnceCell<Arc<AgentManager>>
                as *mut OnceCell<Arc<AgentManager>>;
            let _ = (*cell_ptr).take();
        }
    }

    // Private constructor - prevents direct instantiation in production
    async fn new(max_sessions: Option<usize>) -> Result<Self> {
        let schedule_file_path = Paths::data_dir().join("schedule.json");

        let scheduler = SchedulerFactory::create(schedule_file_path).await?;

        let capacity = NonZeroUsize::new(max_sessions.unwrap_or(DEFAULT_MAX_SESSION))
            .unwrap_or_else(|| NonZeroUsize::new(100).unwrap());

        let manager = Self {
            sessions: Arc::new(RwLock::new(LruCache::new(capacity))),
            scheduler,
            default_provider: Arc::new(RwLock::new(None)),
        };

        let _ = manager.configure_default_provider().await;

        Ok(manager)
    }

    pub async fn instance() -> Result<Arc<Self>> {
        AGENT_MANAGER
            .get_or_try_init(|| async {
                let manager = Self::new(Some(DEFAULT_MAX_SESSION)).await?;
                Ok(Arc::new(manager))
            })
            .await
            .cloned()
    }

    pub async fn scheduler(&self) -> Result<Arc<dyn SchedulerTrait>> {
        Ok(Arc::clone(&self.scheduler))
    }

    pub async fn set_default_provider(&self, provider: Arc<dyn crate::providers::base::Provider>) {
        debug!("Setting default provider on AgentManager");
        *self.default_provider.write().await = Some(provider);
    }

    pub async fn configure_default_provider(&self) -> Result<()> {
        let provider_name = std::env::var("GOOSE_DEFAULT_PROVIDER")
            .or_else(|_| std::env::var("GOOSE_PROVIDER__TYPE"))
            .ok();

        let model_name = std::env::var("GOOSE_DEFAULT_MODEL")
            .or_else(|_| std::env::var("GOOSE_PROVIDER__MODEL"))
            .ok();

        if provider_name.is_none() || model_name.is_none() {
            return Ok(());
        }

        if let (Some(provider_name), Some(model_name)) = (provider_name, model_name) {
            match ModelConfig::new(&model_name) {
                Ok(model_config) => match create(&provider_name, model_config) {
                    Ok(provider) => {
                        self.set_default_provider(provider).await;
                        info!(
                            "Configured default provider: {} with model: {}",
                            provider_name, model_name
                        );
                    }
                    Err(e) => {
                        warn!("Failed to create default provider {}: {}", provider_name, e)
                    }
                },
                Err(e) => warn!("Failed to create model config for {}: {}", model_name, e),
            }
        }
        Ok(())
    }

    pub async fn get_or_create_agent(&self, session_id: String) -> Result<Arc<Agent>> {
        {
            let mut sessions = self.sessions.write().await;
            if let Some(existing) = sessions.get(&session_id) {
                return Ok(Arc::clone(existing));
            }
        }

        let agent = Arc::new(Agent::new());
        agent.set_scheduler(Arc::clone(&self.scheduler)).await;
        agent
            .extension_manager
            .set_context(PlatformExtensionContext {
                session_id: Some(session_id.clone()),
            })
            .await;
        if let Some(provider) = &*self.default_provider.read().await {
            agent.update_provider(Arc::clone(provider)).await?;
        }

        let mut sessions = self.sessions.write().await;
        if let Some(existing) = sessions.get(&session_id) {
            Ok(Arc::clone(existing))
        } else {
            sessions.put(session_id, agent.clone());
            Ok(agent)
        }
    }

    pub async fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions
            .pop(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;
        info!("Removed session {}", session_id);
        Ok(())
    }

    pub async fn has_session(&self, session_id: &str) -> bool {
        self.sessions.read().await.contains(session_id)
    }

    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}
