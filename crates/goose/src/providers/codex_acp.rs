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

const CODEX_ACP_PROVIDER_NAME: &str = "codex-acp";
pub const CODEX_ACP_DEFAULT_MODEL: &str = "gpt-5.2-codex";
const CODEX_ACP_DOC_URL: &str = "https://github.com/zed-industries/codex-acp";

pub struct CodexAcpProvider;

impl ProviderDef for CodexAcpProvider {
    type Provider = AcpProvider;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            CODEX_ACP_PROVIDER_NAME,
            "Codex CLI",
            "ACP adapter for OpenAI's coding assistant. Install: npm install -g @zed-industries/codex-acp",
            CODEX_ACP_DEFAULT_MODEL,
            vec![],
            CODEX_ACP_DOC_URL,
            vec![],
        )
        .with_unlisted_models()
    }

    fn from_env(
        model: ModelConfig,
        extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<AcpProvider>> {
        Box::pin(async move {
            let config = Config::global();
            // Requires: npm install -g @zed-industries/codex-acp
            let resolved_command = SearchPaths::builder()
                .with_npm()
                .resolve(CODEX_ACP_PROVIDER_NAME)?;
            let work_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let env = vec![];
            let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);
            let mcp_servers = extension_configs_to_mcp_servers(&extensions);

            // Codex sandbox blocks network by default. Enable it when HTTP MCP
            // servers are configured so codex-acp can connect to them.
            let has_http_mcp = mcp_servers
                .iter()
                .any(|s| matches!(s, sacp::schema::McpServer::Http(_)));
            let mut args = vec![];
            if has_http_mcp {
                args.extend([
                    "-c".to_string(),
                    "sandbox_workspace_write.network_access=true".to_string(),
                ]);
            }

            // codex-acp permission option_ids
            let permission_mapping = PermissionMapping {
                allow_option_id: Some("approved".to_string()),
                reject_option_id: Some("abort".to_string()),
                rejected_tool_status: sacp::schema::ToolCallStatus::Failed,
            };

            let provider_config = AcpProviderConfig {
                command: resolved_command,
                args,
                env,
                env_remove: vec![],
                work_dir,
                mcp_servers,
                session_mode_id: Some(map_goose_mode(goose_mode)),
                permission_mapping,
            };

            let metadata = Self::metadata();
            AcpProvider::connect(metadata.name, model, goose_mode, provider_config).await
        })
    }
}

fn map_goose_mode(goose_mode: GooseMode) -> String {
    match goose_mode {
        GooseMode::Auto => "auto".to_string(),
        GooseMode::Approve => {
            // Best-fit: read-only requires approval for edits/commands, closest to manual mode.
            "read-only".to_string()
        }
        GooseMode::SmartApprove => {
            // Codex has no risk-based mode; read-only is the safest approximation.
            "read-only".to_string()
        }
        GooseMode::Chat => {
            // Codex lacks a no-tools mode; read-only is the closest available behavior.
            "read-only".to_string()
        }
    }
}
