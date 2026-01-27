use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::base::CodexAcpCommand;
use crate::config::{Config, GooseMode};
use crate::model::ModelConfig;
use crate::providers::acp_agent::AcpProviderCore;
use crate::providers::base::{
    ConfigKey, PermissionRouting, Provider, ProviderMetadata, ProviderUsage,
};
use crate::providers::errors::ProviderError;
use crate::session::{Session, SessionManager, SessionType};
use goose_acp_provider::binary_store::BinaryStore;
use goose_acp_provider::{
    schema::ToolCallStatus, AcpClient, AcpProviderConfig, AcpSessionConfig, PermissionMapping,
};
use rmcp::model::Tool;
use tokio::sync::Mutex;

pub const CODEX_ACP_DEFAULT_MODEL: &str = "default";
pub const CODEX_ACP_DOC_URL: &str = "https://developers.openai.com/codex/cli";
const CODEX_ACP_REPO: &str = "zed-industries/codex-acp";

#[derive(Debug)]
pub struct CodexAcpProvider {
    core: AcpProviderCore,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
}

impl CodexAcpProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = Config::global();
        let store = BinaryStore::new()?;
        let resolved_command = store
            .ensure_github_release_binary(CODEX_ACP_REPO, "codex-acp")
            .await?;
        let args = vec![];
        let work_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let env = vec![];
        let goose_mode = config.get_goose_mode().unwrap_or(GooseMode::Auto);

        let permission_mapping = PermissionMapping {
            allow_option_id: Some("approved".to_string()),
            reject_option_id: Some("abort".to_string()),
            rejected_tool_status: ToolCallStatus::Failed,
        };

        let client_config = AcpProviderConfig {
            session: AcpSessionConfig {
                work_dir,
                ..Default::default()
            },
            command: resolved_command,
            args,
            env,
            session_mode_id: Some(map_goose_mode(goose_mode)),
            permission_mapping,
        };

        let client = AcpClient::connect(client_config).await?;
        Ok(Self {
            core: AcpProviderCore::with_client(
                Self::metadata().name,
                model,
                Arc::new(client),
                goose_mode,
            ),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    async fn ensure_session<'a>(
        &self,
        session_id: Option<&'a str>,
    ) -> Result<&'a str, ProviderError> {
        let session_id = session_id.ok_or_else(|| {
            ProviderError::RequestFailed("ACP session_id is required".to_string())
        })?;

        let has_session = self.sessions.lock().await.contains_key(session_id);
        if has_session {
            Ok(session_id)
        } else {
            Err(ProviderError::RequestFailed(format!(
                "ACP session '{}' not found; resume is not supported",
                session_id
            )))
        }
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

#[async_trait]
impl Provider for CodexAcpProvider {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        ProviderMetadata::new(
            "codex-acp",
            "Codex ACP",
            "Use the Codex ACP agent over ACP.",
            CODEX_ACP_DEFAULT_MODEL,
            vec![],
            CODEX_ACP_DOC_URL,
            vec![ConfigKey::from_value_type::<CodexAcpCommand>(true, false)],
        )
    }

    fn get_name(&self) -> &str {
        self.core.name()
    }

    fn get_model_config(&self) -> ModelConfig {
        self.core.model()
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn permission_routing(&self) -> PermissionRouting {
        PermissionRouting::ActionRequired
    }

    async fn handle_permission_confirmation(
        &self,
        request_id: &str,
        confirmation: &crate::permission::PermissionConfirmation,
    ) -> bool {
        self.core
            .handle_permission_confirmation(request_id, confirmation)
            .await
    }

    async fn complete_with_model(
        &self,
        session_id: Option<&str>,
        model_config: &ModelConfig,
        system: &str,
        messages: &[crate::conversation::message::Message],
        tools: &[Tool],
    ) -> Result<(crate::conversation::message::Message, ProviderUsage), ProviderError> {
        let session_id = self.ensure_session(session_id).await?;
        self.core
            .complete_with_model(session_id, model_config, system, messages, tools)
            .await
    }

    async fn stream(
        &self,
        session_id: &str,
        system: &str,
        messages: &[crate::conversation::message::Message],
        tools: &[Tool],
    ) -> Result<crate::providers::base::MessageStream, ProviderError> {
        let session_id = self.ensure_session(Some(session_id)).await?;
        self.core.stream(session_id, system, messages, tools).await
    }

    async fn create_session(
        &self,
        session_manager: &SessionManager,
        working_dir: PathBuf,
        name: String,
        session_type: SessionType,
        _session_id: Option<String>,
    ) -> Result<Session> {
        let acp_session_id = self.core.new_session().await?;
        let session = session_manager
            .create_session_with_id(
                acp_session_id.0.to_string(),
                working_dir,
                name,
                session_type,
            )
            .await?;
        self.sessions
            .lock()
            .await
            .insert(session.id.clone(), session.clone());
        Ok(session)
    }
}
