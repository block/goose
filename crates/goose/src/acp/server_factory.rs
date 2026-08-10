use crate::acp::server::{
    AcpBuiltinSelection, AcpProviderFactory, ActivePromptRun, GooseAcpAgent, GooseAcpAgentOptions,
};
use crate::agents::{AgentConfig, GoosePlatform};
use crate::config::PermissionManager;
use crate::execution::manager::AgentManager;
use crate::scheduler_trait::SchedulerTrait;
use crate::session::SessionManager;
use crate::source_roots::SourceRoot;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};
use tracing::info;

pub struct AcpServerFactoryConfig {
    pub builtins: AcpBuiltinSelection,
    pub data_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub goose_platform: GoosePlatform,
    pub additional_source_roots: Vec<SourceRoot>,
    pub enable_scheduler: bool,
}

struct SharedAgentState {
    session_manager: Arc<SessionManager>,
    permission_manager: Arc<PermissionManager>,
    agent_manager: Arc<AgentManager>,
    active_prompt_runs: Arc<std::sync::Mutex<HashMap<String, ActivePromptRun>>>,
    run_registration_lock: Arc<Mutex<()>>,
}

pub struct AcpServer {
    config: AcpServerFactoryConfig,
    scheduler: OnceCell<Arc<dyn SchedulerTrait>>,
    shared_state: OnceCell<Arc<SharedAgentState>>,
}

impl AcpServer {
    pub fn new(config: AcpServerFactoryConfig) -> Self {
        Self {
            config,
            scheduler: OnceCell::new(),
            shared_state: OnceCell::new(),
        }
    }

    /// Start the scheduler now instead of on first client connect, so a
    /// headless `goose serve` runs scheduled jobs; on failure `create_agent`
    /// retries. No-op when the scheduler is disabled.
    pub async fn start_scheduler(&self) -> Result<()> {
        self.scheduler().await.map(|_| ())
    }

    async fn scheduler(&self) -> Result<Option<Arc<dyn SchedulerTrait>>> {
        if !self.config.enable_scheduler {
            return Ok(None);
        }

        let data_dir = self.config.data_dir.clone();
        self.scheduler
            .get_or_try_init(|| async move {
                let session_manager = Arc::new(SessionManager::new(data_dir.clone()));
                let schedule_file_path = data_dir.join("schedule.json");
                let scheduler =
                    crate::scheduler::Scheduler::new(schedule_file_path, session_manager)
                        .await
                        .map(|scheduler| scheduler as Arc<dyn SchedulerTrait>)?;
                Ok(scheduler)
            })
            .await
            .cloned()
            .map(Some)
    }

    /// Agent state shared by every ACP connection of this process. Without
    /// sharing, each reconnect starts with an empty session cache, so resuming
    /// sessions re-initializes every extension of every session — a client
    /// stuck in a reconnect loop then floods its MCP servers with `initialize`
    /// requests (see issue #11095). The `SessionManager`/`PermissionManager`
    /// must be the same instances the `AgentManager` holds: the agent and the
    /// ACP layer share their in-memory state.
    async fn shared_state(
        &self,
        scheduler: Option<Arc<dyn SchedulerTrait>>,
        disable_session_naming: bool,
    ) -> Result<Arc<SharedAgentState>> {
        let data_dir = self.config.data_dir.clone();
        let config_dir = self.config.config_dir.clone();
        let goose_platform = self.config.goose_platform.clone();
        self.shared_state
            .get_or_try_init(|| async move {
                let session_manager = Arc::new(SessionManager::new(data_dir));
                let permission_manager = Arc::new(PermissionManager::new(config_dir));
                let agent_config = AgentConfig::new(
                    Arc::clone(&session_manager),
                    Arc::clone(&permission_manager),
                    scheduler,
                    crate::config::Config::global()
                        .get_goose_mode()
                        .unwrap_or_default(),
                    disable_session_naming,
                    goose_platform,
                );
                let agent_manager = AgentManager::new(agent_config, None).await.map(Arc::new)?;
                Ok(Arc::new(SharedAgentState {
                    session_manager,
                    permission_manager,
                    agent_manager,
                    active_prompt_runs: Arc::new(std::sync::Mutex::new(HashMap::new())),
                    run_registration_lock: Arc::new(Mutex::new(())),
                }))
            })
            .await
            .map(Arc::clone)
    }

    pub async fn create_agent(&self) -> Result<Arc<GooseAcpAgent>> {
        let config = crate::config::Config::global();
        let disable_session_naming = config.get_goose_disable_session_naming().unwrap_or(false);
        let scheduler = self.scheduler().await?;
        if let Some(scheduler) = &scheduler {
            // Listing syncs from storage, registering jobs persisted by other processes.
            scheduler.list_scheduled_jobs().await;
        }
        let shared = self
            .shared_state(scheduler.clone(), disable_session_naming)
            .await?;

        let provider_factory: AcpProviderFactory = Arc::new(
            move |provider_name, extensions, working_dir, use_default_model| {
                Box::pin(async move {
                    if use_default_model {
                        crate::providers::create_with_default_model(&provider_name, extensions)
                            .await
                    } else {
                        match working_dir {
                            Some(working_dir) => {
                                crate::providers::create_with_working_dir(
                                    &provider_name,
                                    extensions,
                                    working_dir,
                                )
                                .await
                            }
                            None => crate::providers::create(&provider_name, extensions).await,
                        }
                    }
                })
            },
        );

        let agent = GooseAcpAgent::new(GooseAcpAgentOptions {
            provider_factory,
            builtin_selection: self.config.builtins.clone(),
            config_dir: self.config.config_dir.clone(),
            disable_session_naming,
            goose_platform: self.config.goose_platform.clone(),
            additional_source_roots: self.config.additional_source_roots.clone(),
            agent_manager: Arc::clone(&shared.agent_manager),
            session_manager: Arc::clone(&shared.session_manager),
            permission_manager: Arc::clone(&shared.permission_manager),
            active_prompt_runs: Arc::clone(&shared.active_prompt_runs),
            run_registration_lock: Arc::clone(&shared.run_registration_lock),
        })
        .await?;
        info!("Created new ACP agent");

        Ok(Arc::new(agent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(data_dir: std::path::PathBuf, enable_scheduler: bool) -> AcpServer {
        AcpServer::new(AcpServerFactoryConfig {
            builtins: AcpBuiltinSelection::default(),
            config_dir: data_dir.clone(),
            data_dir,
            goose_platform: GoosePlatform::GooseCli,
            additional_source_roots: Vec::new(),
            enable_scheduler,
        })
    }

    #[tokio::test]
    async fn agent_manager_is_shared_across_connections() {
        let root = tempfile::tempdir().unwrap();
        let server = server(root.path().to_path_buf(), false);

        let first = server.create_agent().await.unwrap();
        let second = server.create_agent().await.unwrap();

        assert!(Arc::ptr_eq(&first.agent_manager(), &second.agent_manager()));
    }

    #[tokio::test]
    async fn disabled_server_does_not_construct_scheduler() {
        let root = tempfile::tempdir().unwrap();
        let server = server(root.path().to_path_buf(), false);

        assert!(server.scheduler().await.unwrap().is_none());
        assert!(!root.path().join("schedule.json").exists());
    }

    #[tokio::test]
    async fn automatic_server_constructs_scheduler() {
        let root = tempfile::tempdir().unwrap();
        let server = server(root.path().to_path_buf(), true);

        assert!(server.scheduler().await.unwrap().is_some());
    }

    #[tokio::test]
    async fn start_scheduler_initializes_before_any_client_connects() {
        let root = tempfile::tempdir().unwrap();
        let server = server(root.path().to_path_buf(), true);

        assert!(!server.scheduler.initialized());
        server.start_scheduler().await.unwrap();
        assert!(server.scheduler.initialized());
    }

    #[tokio::test]
    async fn start_scheduler_is_idempotent() {
        let root = tempfile::tempdir().unwrap();
        let server = server(root.path().to_path_buf(), true);

        server.start_scheduler().await.unwrap();
        server.start_scheduler().await.unwrap();
    }

    #[tokio::test]
    async fn start_scheduler_does_not_construct_one_when_disabled() {
        let root = tempfile::tempdir().unwrap();
        let server = server(root.path().to_path_buf(), false);

        server.start_scheduler().await.unwrap();
        assert!(!server.scheduler.initialized());
    }
}
