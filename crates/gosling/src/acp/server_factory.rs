use crate::acp::server::{AcpProviderFactory, GoslingAcpAgent, GoslingAcpAgentOptions};
use crate::agents::GoslingPlatform;
use crate::source_roots::SourceRoot;
use anyhow::Result;
use std::sync::Arc;
use tracing::info;

pub struct AcpServerFactoryConfig {
    pub builtins: Vec<String>,
    pub data_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub gosling_platform: GoslingPlatform,
    pub additional_source_roots: Vec<SourceRoot>,
}

pub struct AcpServer {
    config: AcpServerFactoryConfig,
}

impl AcpServer {
    pub fn new(config: AcpServerFactoryConfig) -> Self {
        Self { config }
    }

    pub async fn create_agent(&self) -> Result<Arc<GoslingAcpAgent>> {
        let config = crate::config::Config::global();
        let disable_session_naming = config.get_gosling_disable_session_naming().unwrap_or(false);

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

        let agent = GoslingAcpAgent::new(GoslingAcpAgentOptions {
            provider_factory,
            builtins: self.config.builtins.clone(),
            data_dir: self.config.data_dir.clone(),
            config_dir: self.config.config_dir.clone(),
            disable_session_naming,
            gosling_platform: self.config.gosling_platform.clone(),
            additional_source_roots: self.config.additional_source_roots.clone(),
        })
        .await?;
        info!("Created new ACP agent");

        Ok(Arc::new(agent))
    }
}
