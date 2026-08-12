use crate::acp::server::{
    AcpBuiltinSelection, AcpProviderFactory, GooseAcpAgent, GooseAcpAgentOptions,
};
use crate::agents::GoosePlatform;
use crate::scheduler_trait::SchedulerTrait;
use crate::session::SessionManager;
use crate::source_roots::SourceRoot;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::info;

pub struct AcpServerFactoryConfig {
    pub builtins: AcpBuiltinSelection,
    pub data_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub goose_platform: GoosePlatform,
    pub additional_source_roots: Vec<SourceRoot>,
    pub enable_scheduler: bool,
}

pub struct AcpServer {
    config: AcpServerFactoryConfig,
    scheduler: OnceCell<Arc<dyn SchedulerTrait>>,
}

impl AcpServer {
    pub fn new(config: AcpServerFactoryConfig) -> Self {
        Self {
            config,
            scheduler: OnceCell::new(),
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

    pub async fn create_agent(&self) -> Result<Arc<GooseAcpAgent>> {
        let config = crate::config::Config::global();
        let disable_session_naming = config.get_goose_disable_session_naming().unwrap_or(false);
        let scheduler = self.scheduler().await?;
        if let Some(scheduler) = &scheduler {
            // Listing syncs from storage, registering jobs persisted by other processes.
            scheduler.list_scheduled_jobs().await;
        }

        let provider_factory: AcpProviderFactory =
            Arc::new(move |provider_name, extensions, working_dir| {
                Box::pin(async move {
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
                })
            });

        let agent = GooseAcpAgent::new(GooseAcpAgentOptions {
            provider_factory,
            builtin_selection: self.config.builtins.clone(),
            data_dir: self.config.data_dir.clone(),
            config_dir: self.config.config_dir.clone(),
            disable_session_naming,
            goose_platform: self.config.goose_platform.clone(),
            additional_source_roots: self.config.additional_source_roots.clone(),
            scheduler,
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
