use anyhow::Result;
use futures::future::BoxFuture;
use std::path::PathBuf;

use crate::acp::{
    extension_configs_to_mcp_servers, AcpProvider, AcpProviderConfig, PermissionMapping,
};
use crate::config::search_path::SearchPaths;
use crate::config::{Config, GooseMode};
use crate::model::ModelConfig;
use crate::providers::base::{ProviderDef, ProviderMetadata};

const GEMINI_ACP_PROVIDER_NAME: &str = "gemini-acp";
pub const GEMINI_ACP_DEFAULT_MODEL: &str = "default";
const GEMINI_ACP_DOC_URL: &str = "https://github.com/google-gemini/gemini-cli";
const GEMINI_ACP_BINARY: &str = "gemini";

pub struct GeminiAcpProvider;

impl ProviderDef for GeminiAcpProvider {
    type Provider = AcpProvider;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            GEMINI_ACP_PROVIDER_NAME,
            "Gemini CLI (ACP)",
            "ACP provider for Google's Gemini CLI. Install: npm install -g @google/gemini-cli",
            GEMINI_ACP_DEFAULT_MODEL,
            vec![],
            GEMINI_ACP_DOC_URL,
            vec![],
        )
    }

    fn from_env(
        model: ModelConfig,
        extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<AcpProvider>> {
        Box::pin(async move {
            let config = Config::global();
            let resolved_command = SearchPaths::builder()
                .with_npm()
                .resolve(GEMINI_ACP_BINARY)?;
            let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);

            let permission_mapping = PermissionMapping {
                allow_option_id: Some("allow".to_string()),
                reject_option_id: Some("reject".to_string()),
                rejected_tool_status: sacp::schema::ToolCallStatus::Failed,
            };

            let provider_config = AcpProviderConfig {
                command: resolved_command,
                args: vec!["--acp".to_string()],
                env: vec![],
                env_remove: vec![],
                work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                mcp_servers: extension_configs_to_mcp_servers(&extensions),
                session_mode_id: Some(map_goose_mode(goose_mode)),
                permission_mapping,
                notification_callback: None,
            };

            let metadata = Self::metadata();
            AcpProvider::connect(metadata.name, model, goose_mode, provider_config).await
        })
    }
}

fn map_goose_mode(goose_mode: GooseMode) -> String {
    match goose_mode {
        GooseMode::Auto => "yolo".to_string(),
        GooseMode::Approve => "default".to_string(),
        GooseMode::SmartApprove => "auto_edit".to_string(),
        GooseMode::Chat => "plan".to_string(),
    }
}
