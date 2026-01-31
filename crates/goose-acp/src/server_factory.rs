use anyhow::Result;
use goose::config::paths::Paths;
use goose::config::Config;
use goose::model::ModelConfig;
use goose::providers::create;
use std::sync::Arc;
use tracing::info;

use crate::server::{AcpServerConfig, GooseAcpAgent};

pub struct AcpServerFactoryConfig {
    pub builtins: Vec<String>,
    pub data_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
}

impl Default for AcpServerFactoryConfig {
    fn default() -> Self {
        Self {
            builtins: vec!["developer".to_string()],
            data_dir: Paths::data_dir(),
            config_dir: Paths::config_dir(),
        }
    }
}

pub struct AcpServer {
    config: AcpServerFactoryConfig,
}

impl AcpServer {
    pub fn new(config: AcpServerFactoryConfig) -> Self {
        Self { config }
    }

    pub async fn create_agent(&self) -> Result<Arc<GooseAcpAgent>> {
        let global_config = Config::global();

        let provider_name: String = global_config
            .get_goose_provider()
            .map_err(|e| anyhow::anyhow!("No provider configured: {}", e))?;

        let model_name: String = global_config
            .get_goose_model()
            .map_err(|e| anyhow::anyhow!("No model configured: {}", e))?;

        let model_config = ModelConfig {
            request_params: None,
            model_name: model_name.clone(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            fast_model: None,
        };

        let provider = create(&provider_name, model_config).await?;
        let goose_mode = global_config
            .get_goose_mode()
            .unwrap_or(goose::config::GooseMode::Auto);

        let acp_config = AcpServerConfig {
            provider,
            builtins: self.config.builtins.clone(),
            data_dir: self.config.data_dir.clone(),
            config_dir: self.config.config_dir.clone(),
            goose_mode,
        };

        let agent = GooseAcpAgent::with_config(acp_config).await?;
        info!("Created new ACP agent");

        Ok(Arc::new(agent))
    }
}
