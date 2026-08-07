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

pub(crate) const CURSOR_ACP_PROVIDER_NAME: &str = "cursor-acp";
const CURSOR_ACP_DOC_URL: &str = "https://cursor.com/docs/cli/acp";
pub(crate) const CURSOR_ACP_BINARY: &str = "cursor-agent";

pub struct CursorAcpProvider;

impl goose_providers::base::ProviderDescriptor for CursorAcpProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            CURSOR_ACP_PROVIDER_NAME,
            "Cursor (ACP)",
            "Use goose with your Cursor subscription via the cursor-agent ACP adapter.",
            ACP_CURRENT_MODEL,
            vec![],
            CURSOR_ACP_DOC_URL,
            vec![],
        )
        .with_setup_steps(vec![
            "Install the Cursor agent CLI (download from https://cursor.com)",
            "Ensure your Cursor CLI is authenticated (run `cursor-agent login` to verify)",
            "Add to your goose config file (`~/.config/goose/config.yaml` on macOS/Linux):\n  GOOSE_PROVIDER: cursor-acp\n  GOOSE_MODEL: current\n  cursor-acp_configured: true",
            "Restart goose for changes to take effect",
        ])
        .with_model_selection_hint("Use the Cursor model picker or set GOOSE_MODEL explicitly")
    }
}

impl ProviderDef for CursorAcpProvider {
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
            let resolved_command = SearchPaths::builder()
                .with_npm()
                .resolve(CURSOR_ACP_BINARY)?;
            let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);
            let model = config
                .get_goose_model()
                .unwrap_or_else(|_| ACP_CURRENT_MODEL.to_string());

            let session_config_options = if model == ACP_CURRENT_MODEL {
                vec![]
            } else {
                vec![("model".to_string(), model)]
            };

            let mode_mapping = HashMap::from([
                (GooseMode::Auto, vec!["agent".to_string()]),
                (GooseMode::Approve, vec!["ask".to_string()]),
                (GooseMode::SmartApprove, vec!["agent".to_string()]),
                (GooseMode::Chat, vec!["plan".to_string()]),
            ]);

            let provider_config = AcpProviderConfig {
                command: resolved_command,
                args: vec!["acp".to_string()],
                env: vec![],
                env_remove: vec![],
                work_dir: working_dir,
                mcp_servers: extension_configs_to_mcp_servers(&extensions),
                session_mode_id: mode_mapping[&goose_mode].first().cloned(),
                session_config_options,
                model_config_option_id: Some("model".to_string()),
                mode_mapping,
                notification_callback: None,
            };

            let metadata = Self::metadata();
            AcpProvider::connect(metadata.name, goose_mode, provider_config).await
        })
    }
}
