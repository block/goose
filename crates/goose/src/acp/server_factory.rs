use crate::acp::server::{AcpProviderFactory, GooseAcpAgent, GooseAcpAgentOptions};
use crate::agents::GoosePlatform;
use crate::scheduler_trait::SchedulerTrait;
use crate::session::SessionManager;
use crate::source_roots::SourceRoot;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::info;

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

    async fn scheduler(&self) -> Result<Arc<dyn SchedulerTrait>> {
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
