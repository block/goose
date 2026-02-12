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

const CLAUDE_CODE_ACP_PROVIDER_NAME: &str = "claude-code-acp";
pub const CLAUDE_CODE_ACP_DEFAULT_MODEL: &str = "default";
const CLAUDE_CODE_ACP_DOC_URL: &str = "https://github.com/zed-industries/claude-code-acp";

pub struct ClaudeCodeAcpProvider;

impl ProviderDef for ClaudeCodeAcpProvider {
    type Provider = AcpProvider;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            CLAUDE_CODE_ACP_PROVIDER_NAME,
            "Claude Code ACP",
            "Use the Claude Code ACP agent over ACP.",
            CLAUDE_CODE_ACP_DEFAULT_MODEL,
            vec![],
            CLAUDE_CODE_ACP_DOC_URL,
            vec![],
        )
    }

    fn from_env(
        model: ModelConfig,
        extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<AcpProvider>> {
        Box::pin(async move {
            let config = Config::global();
            // Requires: npm install -g @zed-industries/claude-code-acp
            let resolved_command = SearchPaths::builder()
                .with_npm()
                .resolve(CLAUDE_CODE_ACP_PROVIDER_NAME)?;
            let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);

            let permission_mapping = PermissionMapping {
                allow_option_id: Some("allow".to_string()),
                reject_option_id: Some("reject".to_string()),
                rejected_tool_status: sacp::schema::ToolCallStatus::Failed,
            };

            let provider_config = AcpProviderConfig {
                command: resolved_command,
                args: vec![],
                env: vec![],
                work_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                mcp_servers: extension_configs_to_mcp_servers(&extensions),
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
        GooseMode::Auto => {
            // Closest to "autonomous": Claude Code's bypassPermissions skips confirmations.
            "bypassPermissions".to_string()
        }
        GooseMode::Approve => {
            // Claude Code's default matches "ask before risky actions".
            "default".to_string()
        }
        GooseMode::SmartApprove => {
            // Best-effort: acceptEdits auto-accepts file edits but still prompts for risky ops.
            "acceptEdits".to_string()
        }
        GooseMode::Chat => {
            // Plan mode disables tool execution, aligning with chat-only intent.
            "plan".to_string()
        }
    }
}
