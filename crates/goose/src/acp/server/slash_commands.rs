use super::*;
use std::path::PathBuf;

impl GooseAcpAgent {
    pub(super) async fn on_list_slash_commands(
        &self,
        req: ListSlashCommandsRequest,
    ) -> Result<ListSlashCommandsResponse, agent_client_protocol::Error> {
        let cwd = if let Some(cwd) = req
            .cwd
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            PathBuf::from(cwd)
        } else if let Some(session_id) = req
            .session_id
            .as_deref()
            .map(str::trim)
            .filter(|session_id| !session_id.is_empty())
        {
            self.context
                .session_manager
                .get_session(session_id, false)
                .await
                .map_err(|e| agent_client_protocol::Error::invalid_params().data(e.to_string()))?
                .working_dir
        } else {
            return Err(agent_client_protocol::Error::invalid_params()
                .data("Either cwd or sessionId is required"));
        };

        Ok(ListSlashCommandsResponse {
            available_commands: crate::acp::response_builder::available_commands_for_working_dir(
                &cwd,
            ),
        })
    }
}
