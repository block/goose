use crate::acp::custom_requests::GooseExtension;
use crate::acp::server::{meta_string, validate_absolute_cwd, ResultExt};
use crate::agents::ExtensionLoadResult;
use crate::config::{Config, GooseMode};
use crate::recipe::{Recipe, Settings};
use crate::session::{ExtensionData, Session, SessionType};

use super::GooseAcpAgent;
use agent_client_protocol::schema::v1::{Meta, NewSessionRequest, NewSessionResponse, SessionId};
use agent_client_protocol::{Client, ConnectionTo};
use goose_providers::model::ModelConfig;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::warn;

struct InitialSessionConfig {
    provider: String,
    model_config: ModelConfig,
    extension_data: ExtensionData,
    recipe: Option<Recipe>,
    user_recipe_values: Option<HashMap<String, String>>,
    project_id: Option<String>,
}

impl GooseAcpAgent {
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
        let current_mode: GooseMode = config.get_goose_mode().unwrap_or_default();
        let recipe = self.resolve_recipe_from_meta(args.meta.as_ref()).await?;
        let session_name = match recipe.as_ref() {
            Some((recipe, _)) if !recipe.title.trim().is_empty() => recipe.title.clone(),
            _ => "New Chat".to_string(),
        };

        let session = self
            .session_manager
            .create_session(args.cwd.clone(), session_name, session_type, current_mode)
            .await
            .internal_err_ctx("Failed to create session")?;
        match self
            .finish_new_session_setup(cx, config, &session, args, recipe, project_id)
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
        recipe: Option<(Recipe, PathBuf)>,
        project_id: Option<String>,
    ) -> Result<NewSessionResponse, agent_client_protocol::Error> {
        let rendered_recipe = self
            .configure_new_session(cx, config, session, args, recipe, project_id)
            .await?;

        let reloaded_session = self.reload_session(&session.id).await?;
        let (agent, extension_results) = self
            .activate_acp_session(cx, &reloaded_session, HashMap::new())
            .await?;
        if let Some(recipe) = &rendered_recipe {
            self.apply_recipe(&agent, recipe).await;
        }

        let reloaded_session = self.reload_session(&session.id).await?;
        let response = self
            .build_new_session_response(&reloaded_session, &extension_results)
            .await?;
        super::send_session_setup_notifications(
            cx,
            &reloaded_session,
            self.supports_goose_custom_notifications(),
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
        cx: &ConnectionTo<Client>,
        config: &Config,
        session: &Session,
        args: NewSessionRequest,
        recipe: Option<(Recipe, PathBuf)>,
        project_id: Option<String>,
    ) -> Result<Option<Recipe>, agent_client_protocol::Error> {
        let (rendered, user_recipe_values) = self
            .render_recipe_for_session(cx, &session.id, recipe.as_ref())
            .await?;

        let recipe_settings = rendered.as_ref().and_then(|r| r.settings.as_ref());
        let (provider, model_config) = self
            .resolve_provider_and_model(config, args.meta.as_ref(), recipe_settings)
            .await?;

        let goose_extensions = meta_goose_extensions(args.meta.as_ref())?;
        let recipe_extensions = rendered.as_ref().and_then(|r| r.extensions.as_deref());
        let extension_data = self.build_enabled_extensions_data(
            config,
            session,
            args.mcp_servers,
            goose_extensions,
            recipe_extensions,
        )?;

        self.apply_initial_session_config(
            &session.id,
            InitialSessionConfig {
                provider,
                model_config,
                extension_data,
                recipe: recipe.map(|(recipe, _)| recipe),
                user_recipe_values,
                project_id,
            },
        )
        .await?;

        Ok(rendered)
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
        recipe_settings: Option<&Settings>,
    ) -> Result<(String, ModelConfig), agent_client_protocol::Error> {
        let recipe_provider = recipe_settings.and_then(|s| s.goose_provider.clone());
        let recipe_model = recipe_settings.and_then(|s| s.goose_model.clone());

        let provider = match recipe_provider {
            Some(provider) => provider,
            None => match meta_string(meta, "provider")? {
                Some(provider) => provider,
                None => {
                    if let Some(model) = recipe_model.as_deref() {
                        let provider = config.get_goose_provider().map_err(|error| {
                            agent_client_protocol::Error::internal_error()
                                .data(format!("Failed to resolve provider: {}", error))
                        })?;
                        let model_config = model_config_from_recipe_settings(&provider, model)?;
                        return Ok((provider, model_config));
                    }

                    return super::resolve_default_provider_model_config(config);
                }
            },
        };

        let model_config = match recipe_model {
            Some(model) => model_config_from_recipe_settings(&provider, &model)?,
            None => super::resolve_provider_default_model_config(&provider).await?,
        };

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
        if let Some(recipe) = config.recipe {
            builder = builder.recipe(Some(recipe));
        }
        if config.user_recipe_values.is_some() {
            builder = builder.user_recipe_values(config.user_recipe_values);
        }
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

fn model_config_from_recipe_settings(
    provider: &str,
    model: &str,
) -> Result<ModelConfig, agent_client_protocol::Error> {
    crate::model_config::model_config_from_user_config(provider, model)
        .internal_err_ctx("Failed to build model config from recipe settings")
}

fn meta_goose_extensions(
    meta: Option<&Meta>,
) -> Result<Option<Vec<GooseExtension>>, agent_client_protocol::Error> {
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
