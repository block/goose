use anyhow::Result;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::acp::{
    extension_configs_to_mcp_servers, AcpProvider, AcpProviderConfig, ACP_CURRENT_MODEL,
};
use crate::config::search_path::SearchPaths;
use crate::config::{Config, GoslingMode};
use crate::providers::base::{
    current_working_dir, ProviderDef, ProviderDescriptor, ProviderMetadata,
};

pub(crate) const AMP_ACP_PROVIDER_NAME: &str = "amp-acp";
const AMP_ACP_DOC_URL: &str = "https://ampcode.com";
pub(crate) const AMP_ACP_BINARY: &str = "amp-acp";

pub struct AmpAcpProvider;

impl gosling_providers::base::ProviderDescriptor for AmpAcpProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            AMP_ACP_PROVIDER_NAME,
            "Amp",
            "Use gosling with your Amp subscription via the amp-acp adapter.",
            ACP_CURRENT_MODEL,
            vec![],
            AMP_ACP_DOC_URL,
            vec![],
        )
        .with_setup_steps(vec![
            "Install the Amp CLI: `curl -fsSL https://ampcode.com/install.sh | bash`",
            "Install the ACP adapter: `npm install -g amp-acp`",
            "Ensure your Amp CLI is authenticated (run `amp` to verify)",
            "Add to your gosling config file (`~/.config/gosling/config.yaml` on macOS/Linux):\n  GOSLING_PROVIDER: amp-acp\n  GOSLING_MODEL: current\n  amp-acp_configured: true",
            "Restart gosling for changes to take effect",
        ])
        .with_model_selection_hint("Use the Amp CLI to configure models")
    }
}

impl ProviderDef for AmpAcpProvider {
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
            let resolved_command = SearchPaths::builder().with_npm().resolve(AMP_ACP_BINARY)?;
            let gosling_mode = config.get_gosling_mode().unwrap_or(GoslingMode::Auto);

            let mode_mapping = HashMap::from([
                // "bypass" skips confirmations, closest to autonomous mode.
                (GoslingMode::Auto, "bypass".to_string()),
                // "default" prompts before risky actions.
                (GoslingMode::Approve, "default".to_string()),
                (GoslingMode::SmartApprove, "default".to_string()),
                (GoslingMode::Chat, "default".to_string()),
            ]);

            let provider_config = AcpProviderConfig {
                command: resolved_command,
                args: vec![],
                env: vec![],
                env_remove: vec![],
                work_dir: working_dir,
                mcp_servers: extension_configs_to_mcp_servers(&extensions),
                session_mode_id: Some(mode_mapping[&gosling_mode].clone()),
                session_config_options: vec![],
                model_config_option_id: None,
                mode_mapping,
                notification_callback: None,
            };

            let metadata = Self::metadata();
            AcpProvider::connect(metadata.name, gosling_mode, provider_config).await
        })
    }
}
