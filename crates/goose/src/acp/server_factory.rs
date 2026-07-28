use crate::acp::server::{AcpProviderFactory, GooseAcpAgent, GooseAcpAgentOptions};
use crate::agents::GoosePlatform;
use crate::scheduler::scheduler_disabled_by_env;
use crate::scheduler_trait::SchedulerTrait;
use crate::session::SessionManager;
use crate::source_roots::SourceRoot;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::info;

/// Re-exported so callers that reach the switch through the ACP surface keep
/// working; the switch itself is process-wide and lives with the scheduler.
pub use crate::scheduler::GOOSE_ACP_SCHEDULER_DISABLED_ENV;

pub struct AcpServerFactoryConfig {
    pub builtins: Vec<String>,
    pub data_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub goose_platform: GoosePlatform,
    pub additional_source_roots: Vec<SourceRoot>,
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

    /// Returns the scheduler for ACP agents, or `None` when scheduling is
    /// disabled for this runtime.
    ///
    /// When disabled the [`crate::scheduler::Scheduler`] is never constructed,
    /// so `schedule.json` is neither read nor written.
    async fn scheduler(&self) -> Result<Option<Arc<dyn SchedulerTrait>>> {
        if scheduler_disabled_by_env() {
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
            builtins: self.config.builtins.clone(),
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
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn scheduler_is_none_and_writes_nothing_when_disabled() {
        let data_dir = tempfile::tempdir().unwrap();
        let server = AcpServer::new(AcpServerFactoryConfig {
            builtins: Vec::new(),
            data_dir: data_dir.path().to_path_buf(),
            config_dir: data_dir.path().to_path_buf(),
            goose_platform: GoosePlatform::GooseCli,
            additional_source_roots: Vec::new(),
        });
        let _guard = env_lock::lock_env([(GOOSE_ACP_SCHEDULER_DISABLED_ENV, Some("true"))]);

        assert!(server.scheduler().await.unwrap().is_none());
        assert!(!data_dir.path().join("schedule.json").exists());
    }

    #[tokio::test]
    #[serial]
    async fn scheduler_is_built_when_enabled() {
        let data_dir = tempfile::tempdir().unwrap();
        let server = AcpServer::new(AcpServerFactoryConfig {
            builtins: Vec::new(),
            data_dir: data_dir.path().to_path_buf(),
            config_dir: data_dir.path().to_path_buf(),
            goose_platform: GoosePlatform::GooseCli,
            additional_source_roots: Vec::new(),
        });
        let _guard = env_lock::lock_env([(GOOSE_ACP_SCHEDULER_DISABLED_ENV, None::<&str>)]);

        assert!(server.scheduler().await.unwrap().is_some());
    }
}
