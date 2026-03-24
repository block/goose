use anyhow::Result;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::acp::{
    extension_configs_to_mcp_servers, AcpProvider, AcpProviderConfig, PermissionMapping,
    ACP_CURRENT_MODEL,
};
use crate::config::search_path::SearchPaths;
use crate::config::{Config, GooseMode};
use crate::model::ModelConfig;
use crate::providers::base::{ProviderDef, ProviderMetadata};

const GEMINI_ACP_PROVIDER_NAME: &str = "gemini-acp";
const GEMINI_ACP_DOC_URL: &str = "https://github.com/google-gemini/gemini-cli";

pub struct GeminiAcpProvider;

impl ProviderDef for GeminiAcpProvider {
    type Provider = AcpProvider;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            GEMINI_ACP_PROVIDER_NAME,
            "Gemini CLI (ACP)",
            "Use goose with your Google Gemini subscription via the Gemini CLI.",
            ACP_CURRENT_MODEL,
            vec![],
            GEMINI_ACP_DOC_URL,
            vec![],
        )
        .with_setup_steps(vec![
            "Install the Gemini CLI: `npm install -g @google/gemini-cli`",
            "Run `gemini` once to authenticate with your Google account",
            "Set in your goose config file (`~/.config/goose/config.yaml` on macOS/Linux):\n  GOOSE_PROVIDER: gemini-acp\n  GOOSE_MODEL: current",
            "Restart goose for changes to take effect",
        ])
    }

    fn from_env(
        model: ModelConfig,
        extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<AcpProvider>> {
        Box::pin(async move {
            let config = Config::global();
            let command_name: String = config.get_gemini_cli_command().unwrap_or_default().into();
            let resolved_command = SearchPaths::builder().with_npm().resolve(&command_name)?;
            let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);

            let permission_mapping = PermissionMapping {
                allow_option_id: Some("allow".to_string()),
                reject_option_id: Some("reject".to_string()),
                rejected_tool_status: sacp::schema::ToolCallStatus::Failed,
            };

            let mut args = vec!["--acp".to_string()];
            if model.model_name != ACP_CURRENT_MODEL {
                args.push("--model".to_string());
                args.push(model.model_name.clone());
            }

            let mode_mapping = HashMap::from([
                (GooseMode::Auto, "yolo".to_string()),
                (GooseMode::Approve, "default".to_string()),
                (GooseMode::SmartApprove, "auto_edit".to_string()),
                (GooseMode::Chat, "plan".to_string()),
            ]);

            let provider_config = AcpProviderConfig {
                command: resolved_command,
                args,
                env: vec![],
                env_remove: vec![],
                work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                mcp_servers: extension_configs_to_mcp_servers(&extensions),
                session_mode_id: Some(mode_mapping[&goose_mode].clone()),
                mode_mapping,
                permission_mapping,
                notification_callback: None,
            };

            let metadata = Self::metadata();
            AcpProvider::connect(metadata.name, model, goose_mode, provider_config).await
        })
    }
}
