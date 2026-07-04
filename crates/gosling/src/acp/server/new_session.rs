use crate::acp::custom_requests::GoslingExtension;
use crate::acp::server::{meta_string, validate_absolute_cwd, ResultExt};
use crate::agents::ExtensionLoadResult;
use crate::config::{Config, GoslingMode};
use crate::session::{ExtensionData, Session, SessionType};

use super::GoslingAcpAgent;
use agent_client_protocol::schema::v1::{Meta, NewSessionRequest, NewSessionResponse, SessionId};
use agent_client_protocol::{Client, ConnectionTo};
use gosling_providers::model::ModelConfig;
use std::collections::HashMap;
use tracing::warn;

struct InitialSessionConfig {
    provider: String,
    model_config: ModelConfig,
    extension_data: ExtensionData,
    project_id: Option<String>,
}

impl GoslingAcpAgent {
    pub(super) async fn handle_new_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: NewSessionRequest,
    ) -> Result<NewSessionResponse, agent_client_protocol::Error> {
        validate_absolute_cwd(&args.cwd)?;
        let config = Config::global();
        let project_id = meta_string(args.meta.as_ref(), "projectId")?;
        let session_type = match meta_string(args.meta.as_ref(), "client")? {
            Some(_) => SessionType::User,
            None => SessionType::Acp,
        };
        let current_mode: GoslingMode = config.get_gosling_mode().unwrap_or_default();

        let session = self
            .session_manager
            .create_session(
                args.cwd.clone(),
                "New Chat".to_string(),
                session_type,
                current_mode,
            )
            .await
            .internal_err_ctx("Failed to create session")?;
        match self
            .finish_new_session_setup(cx, config, &session, args, project_id)
            .await
        {
            Ok(response) => Ok(response),
            Err(error) => {
                self.cleanup_failed_new_session(&session.id).await;
                Err(error)
            }
        }
    }

    async fn finish_new_session_setup(
        &self,
        cx: &ConnectionTo<Client>,
        config: &Config,
        session: &Session,
        args: NewSessionRequest,
        project_id: Option<String>,
    ) -> Result<NewSessionResponse, agent_client_protocol::Error> {
        self.configure_new_session(config, session, args, project_id)
            .await?;

        let reloaded_session = self.reload_session(&session.id).await?;
        let (_agent, extension_results) = self
            .activate_acp_session(cx, &reloaded_session, HashMap::new())
            .await?;

        let reloaded_session = self.reload_session(&session.id).await?;
        let response = self
            .build_new_session_response(&reloaded_session, &extension_results)
            .await?;
        super::send_session_setup_notifications(
            cx,
            &reloaded_session,
            self.supports_gosling_custom_notifications(),
        )?;
        Ok(response)
    }

    async fn cleanup_failed_new_session(&self, session_id: &str) {
        if let Err(error) = self.session_manager.delete_session(session_id).await {
            warn!(
                session_id,
                %error,
                "Failed to delete session during new-session cleanup"
            );
        }
        self.sessions.lock().await.remove(session_id);
        if let Err(error) = self
            .agent_manager
            .remove_session_if_loaded(session_id)
            .await
        {
            warn!(
                session_id,
                %error,
                "Failed to remove in-memory agent during new-session cleanup"
            );
        }
    }

    async fn configure_new_session(
        &self,
        config: &Config,
        session: &Session,
        args: NewSessionRequest,
        project_id: Option<String>,
    ) -> Result<(), agent_client_protocol::Error> {
        let (provider, model_config) = self
            .resolve_provider_and_model(config, args.meta.as_ref())
            .await?;

        let gosling_extensions = meta_gosling_extensions(args.meta.as_ref())?;
        let extension_data = self.build_enabled_extensions_data(
            config,
            session,
            args.mcp_servers,
            gosling_extensions,
        )?;

        self.apply_initial_session_config(
            &session.id,
            InitialSessionConfig {
                provider,
                model_config,
                extension_data,
                project_id,
            },
        )
        .await?;

        Ok(())
    }

    async fn reload_session(
        &self,
        session_id: &str,
    ) -> Result<Session, agent_client_protocol::Error> {
        self.session_manager
            .get_session(session_id, false)
            .await
            .internal_err_ctx("Failed to reload session")
    }

    async fn resolve_provider_and_model(
        &self,
        config: &Config,
        meta: Option<&Meta>,
    ) -> Result<(String, ModelConfig), agent_client_protocol::Error> {
        let provider = match meta_string(meta, "provider")? {
            Some(provider) => provider,
            None => {
                return super::resolve_default_provider_model_config(config);
            }
        };

        let model_config = super::resolve_provider_default_model_config(&provider).await?;

        Ok((provider, model_config))
    }

    async fn apply_initial_session_config(
        &self,
        session_id: &str,
        config: InitialSessionConfig,
    ) -> Result<(), agent_client_protocol::Error> {
        let mut builder = self
            .session_manager
            .update(session_id)
            .provider_name(config.provider)
            .model_config(config.model_config)
            .extension_data(config.extension_data);
        if let Some(project_id) = config.project_id {
            builder = builder.project_id(Some(project_id));
        }
        builder
            .apply()
            .await
            .internal_err_ctx("Failed to update session")?;
        Ok(())
    }

    async fn build_new_session_response(
        &self,
        session: &Session,
        extension_results: &[ExtensionLoadResult],
    ) -> Result<NewSessionResponse, agent_client_protocol::Error> {
        let (mode_state, config_options) =
            super::build_session_setup_config(&self.provider_inventory, session).await?;

        let mut response =
            NewSessionResponse::new(SessionId::new(session.id.clone())).modes(mode_state);
        if let Some(co) = config_options {
            response = response.config_options(co);
        }
        response = response.meta(super::session_response_meta(session, extension_results));
        Ok(response)
    }
}

fn meta_gosling_extensions(
    meta: Option<&Meta>,
) -> Result<Option<Vec<GoslingExtension>>, agent_client_protocol::Error> {
    let Some(value) = meta.and_then(|m| m.get("enabledExtensions")) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|e| {
            agent_client_protocol::Error::invalid_params().data(format!("enabledExtensions: {e}"))
        })
}
