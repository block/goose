use super::*;

impl GooseAcpAgent {
    pub(super) async fn on_read_resource(
        &self,
        req: ReadResourceRequest,
    ) -> Result<ReadResourceResponse, agent_client_protocol::Error> {
        let session_id = &req.session_id;
        // A cached agent bypasses get_session_agent's activation guard; keep
        // resource reads consistent with the tool-call path when the stored
        // working dir vanished after activation.
        self.validate_session_working_dir(session_id).await?;
        let agent = self.get_session_agent(session_id).await?;
        let cancel_token = CancellationToken::new();
        let result = agent
            .extension_manager
            .read_resource(session_id, &req.uri, &req.extension_name, cancel_token)
            .await
            .internal_err()?;
        let result_json = serde_json::to_value(&result).internal_err()?;
        Ok(ReadResourceResponse {
            result: result_json,
        })
    }
}
