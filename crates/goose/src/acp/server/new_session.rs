use crate::acp::server::{meta_string, sid_short, validate_absolute_cwd, ResultExt};
use crate::config::{Config, GooseMode};
use crate::session::SessionType;

use super::GooseAcpAgent;
use agent_client_protocol::schema::{NewSessionRequest, NewSessionResponse, SessionId};
use agent_client_protocol::{Client, ConnectionTo};
use std::collections::HashMap;
use tracing::debug;

impl GooseAcpAgent {
    #[allow(dead_code)]
    pub(super) async fn handle_new_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: NewSessionRequest,
    ) -> Result<NewSessionResponse, agent_client_protocol::Error> {
        debug!(?args, "new session request");
        let t_start = std::time::Instant::now();
        validate_absolute_cwd(&args.cwd)?;
        let project_id = meta_string(args.meta.as_ref(), "projectId");
        let session_type = match meta_string(args.meta.as_ref(), "client") {
            Some(_) => SessionType::User,
            None => SessionType::Acp,
        };
        let config = Config::global();
        let (resolved_provider, resolved_model_config) =
            match meta_string(args.meta.as_ref(), "provider") {
                Some(provider) => {
                    let model_config =
                        super::resolve_provider_default_model_config(&provider).await?;
                    (provider, model_config)
                }
                None => super::resolve_default_provider_model_config(config)?,
            };
        let current_mode: GooseMode = config.get_goose_mode().unwrap_or_default();
        let t0 = std::time::Instant::now();
        let mut goose_session = self
            .session_manager
            .create_session(
                args.cwd.clone(),
                "New Chat".to_string(),
                session_type,
                current_mode,
            )
            .await
            .internal_err_ctx("Failed to create session")?;
        let mut builder = self.session_manager.update(&goose_session.id);
        let extension_data =
            self.build_enabled_extensions_data(config, &goose_session, args.mcp_servers)?;
        builder = builder
            .provider_name(resolved_provider)
            .model_config(resolved_model_config)
            .extension_data(extension_data);
        if let Some(pid) = project_id {
            builder = builder.project_id(Some(pid));
        }
        builder
            .apply()
            .await
            .internal_err_ctx("Failed to update session")?;

        goose_session = self
            .session_manager
            .get_session(&goose_session.id, false)
            .await
            .internal_err_ctx("Failed to reload session")?;
        let session_id_str = goose_session.id.clone();
        let sid = sid_short(&session_id_str);
        debug!(target: "perf", sid = %sid, ms = t0.elapsed().as_millis() as u64, "perf: new_session create_session");

        let (_agent, extension_results) = self
            .activate_acp_session(cx, &goose_session, HashMap::new())
            .await?;

        let goose_session = self
            .session_manager
            .get_session(&goose_session.id, false)
            .await
            .internal_err_ctx("Failed to reload session")?;

        let acp_session_id = SessionId::new(session_id_str.clone());

        let (mode_state, model_state, config_options) =
            super::build_session_setup_config(&self.provider_inventory, &goose_session).await?;

        let mut response = NewSessionResponse::new(acp_session_id.clone()).modes(mode_state);
        if let Some(ms) = model_state {
            response = response.models(ms);
        }
        if let Some(co) = config_options {
            response = response.config_options(co);
        }
        if let Ok(extension_results) = serde_json::to_value(&extension_results) {
            let mut meta = serde_json::Map::new();
            meta.insert("extensionResults".to_string(), extension_results);
            response = response.meta(meta);
        }
        super::send_session_setup_notifications(
            cx,
            &goose_session,
            self.supports_goose_custom_notifications(),
        )?;
        debug!(
            target: "perf",
            sid = %sid,
            ms = t_start.elapsed().as_millis() as u64,
            "perf: new_session done"
        );
        Ok(response)
    }
}
