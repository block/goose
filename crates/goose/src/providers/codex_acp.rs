use anyhow::Result;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::acp::{
    extension_configs_to_mcp_servers, AcpProvider, AcpProviderConfig, PermissionMapping,
    ACP_CURRENT_MODEL,
};
use crate::config::{Config, GooseMode};
use crate::model::ModelConfig;
use crate::providers::base::{ProviderDef, ProviderMetadata};

const CODEX_ACP_PROVIDER_NAME: &str = "codex-acp";
const CODEX_ACP_DOC_URL: &str = "https://github.com/zed-industries/codex-acp";

pub struct CodexAcpProvider;

impl ProviderDef for CodexAcpProvider {
    type Provider = AcpProvider;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            CODEX_ACP_PROVIDER_NAME,
            "Codex CLI",
            "Use goose with your ChatGPT Plus/Pro subscription via the codex-acp adapter.",
            ACP_CURRENT_MODEL,
            vec![],
            CODEX_ACP_DOC_URL,
            vec![],
        )
        .with_setup_steps(vec![
            "Install the ACP adapter: `npm install -g @zed-industries/codex-acp`",
            "Run `codex` once to authenticate with your OpenAI account",
            "Set in your goose config file (`~/.config/goose/config.yaml` on macOS/Linux):\n  GOOSE_PROVIDER: codex-acp\n  GOOSE_MODEL: current",
            "Restart goose for changes to take effect",
        ])
    }

    fn from_env(
        model: ModelConfig,
        extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<AcpProvider>> {
        Box::pin(async move {
            let config = Config::global();
            let work_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);
            let mcp_servers = extension_configs_to_mcp_servers(&extensions);

            // Seed read-only so modes() returns a preset match and session/new
            // includes modes. session_mode_id upgrades to the real mode.
            // https://github.com/zed-industries/codex-acp/issues/221
            let args = vec![
                "-c".to_string(),
                "sandbox_mode=read-only".to_string(),
                "-c".to_string(),
                "sandbox_workspace_write.network_access=true".to_string(),
            ];

            // codex-acp permission option_ids
            let permission_mapping = PermissionMapping {
                allow_option_id: Some("approved".to_string()),
                reject_option_id: Some("abort".to_string()),
                rejected_tool_status: sacp::schema::ToolCallStatus::Failed,
            };

            // Chat and Approve both map to "read-only".
            let mode_mapping = HashMap::from([
                (GooseMode::Auto, "full-access".to_string()),
                (GooseMode::Approve, "read-only".to_string()),
                (GooseMode::SmartApprove, "auto".to_string()),
                (GooseMode::Chat, "read-only".to_string()),
            ]);

            let provider_config = AcpProviderConfig {
                command: CODEX_ACP_PROVIDER_NAME.to_string(),
                args,
                env: vec![],
                env_remove: vec![],
                work_dir,
                mcp_servers,
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
