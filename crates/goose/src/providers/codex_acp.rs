use anyhow::Result;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::acp::{
    extension_configs_to_mcp_servers, AcpProvider, AcpProviderConfig, ACP_CURRENT_MODEL,
};
use crate::config::search_path::SearchPaths;
use crate::config::{Config, GooseMode};
use crate::providers::base::{
    current_working_dir, ProviderDef, ProviderDescriptor, ProviderMetadata,
};

pub(crate) const CODEX_ACP_PROVIDER_NAME: &str = "codex-acp";
const CODEX_ACP_DOC_URL: &str = "https://github.com/agentclientprotocol/codex-acp";

pub struct CodexAcpProvider;

impl goose_providers::base::ProviderDescriptor for CodexAcpProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            CODEX_ACP_PROVIDER_NAME,
            "Codex CLI",
            "Use goose with ChatGPT Plus/Pro or OpenAI API credits via the codex-acp adapter.",
            ACP_CURRENT_MODEL,
            vec![],
            CODEX_ACP_DOC_URL,
            vec![],
        )
        .with_setup_steps(vec![
            "Verify `codex-acp --version` shows `@agentclientprotocol/codex-acp`",
            "If `--version` is rejected, remove `@zed-industries/codex-acp`: `npm uninstall -g @zed-industries/codex-acp`",
            "If `codex-acp` is missing or was removed, install `@agentclientprotocol/codex-acp`: `npm install -g @agentclientprotocol/codex-acp`",
            "Authenticate with OpenAI: run `codex` and follow the prompts",
            "Configure goose in `~/.config/goose/config.yaml`:\n  GOOSE_PROVIDER: codex-acp\n  GOOSE_MODEL: current",
            "Restart goose",
        ])
    }
}

impl ProviderDef for CodexAcpProvider {
    type Provider = AcpProvider;

    fn from_env(
        extensions: Vec<crate::config::ExtensionConfig>,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<AcpProvider>> {
        Self::from_env_with_working_dir(extensions, current_working_dir(), tls_config)
    }

    fn from_env_with_working_dir(
        extensions: Vec<crate::config::ExtensionConfig>,
        working_dir: PathBuf,
        _tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<AcpProvider>> {
        Box::pin(async move {
            let config = Config::global();
            // with_npm() includes npm global bin dir (desktop app PATH may not)
            let resolved_command = SearchPaths::builder()
                .with_npm()
                .resolve(CODEX_ACP_PROVIDER_NAME)?;
            let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);
            let mcp_servers = extension_configs_to_mcp_servers(&extensions);

            let mode_mapping = HashMap::from([
                (GooseMode::Auto, vec!["agent-full-access".to_string()]),
                (GooseMode::SmartApprove, vec!["agent".to_string()]),
                (GooseMode::Approve, vec!["read-only".to_string()]),
                (GooseMode::Chat, vec!["read-only".to_string()]),
            ]);

            let provider_config = AcpProviderConfig {
                command: resolved_command,
                args: vec![],
                env: vec![],
                env_remove: vec![],
                work_dir: working_dir,
                mcp_servers,
                session_mode_id: None,
                session_config_options: vec![],
                model_config_option_id: Some("model".to_string()),
                mode_mapping,
                notification_callback: None,
            };

            let metadata = Self::metadata();
            AcpProvider::connect(metadata.name, goose_mode, provider_config).await
        })
    }
}
