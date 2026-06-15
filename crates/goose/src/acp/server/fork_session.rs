use super::*;

impl GooseAcpAgent {
    #[allow(dead_code)]
    pub(super) async fn handle_fork_session(
        &self,
        cx: &ConnectionTo<Client>,
        args: ForkSessionRequest,
    ) -> Result<ForkSessionResponse, agent_client_protocol::Error> {
        validate_absolute_cwd(&args.cwd)?;
        let source_session_id = &*args.session_id.0;

        let source = self
            .session_manager
            .get_session(source_session_id, false)
            .await
            .internal_err()?;
        let fork_name = if source.name.trim().is_empty() {
            "(copy)".to_string()
        } else {
            format!("{} (copy)", source.name)
        };

        let new_session = self
            .session_manager
            .copy_session(source_session_id, fork_name)
            .await
            .internal_err()?;
        let new_session_id = new_session.id.clone();

        let goose_session = self
            .session_manager
            .get_session(&new_session_id, false)
            .await
            .internal_err()?;

        let goose_session = self
            .prepare_session_for_activation(
                goose_session,
                args.cwd.clone(),
                args.mcp_servers,
                false,
            )
            .await?;

        let (_agent, extension_results) = self
            .activate_acp_session(cx, &goose_session, HashMap::new())
            .await?;

        let acp_session_id = SessionId::new(new_session_id.clone());
        let mut meta = session_meta(&new_session);
        if let Ok(v) = serde_json::to_value(&extension_results) {
            meta.insert("extensionResults".to_string(), v);
        }

        let (mode_state, model_state, config_options) =
            build_session_setup_config(&self.provider_inventory, &goose_session).await?;

        let mut response = ForkSessionResponse::new(acp_session_id.clone())
            .modes(mode_state)
            .meta(meta);

        if let Some(ms) = model_state {
            response = response.models(ms);
        }
        if let Some(co) = config_options {
            response = response.config_options(co);
        }
        send_session_setup_notifications(
            cx,
            &goose_session,
            self.supports_goose_custom_notifications(),
        )?;
        Ok(response)
    }
}
