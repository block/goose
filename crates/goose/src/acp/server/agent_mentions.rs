use super::*;
use goose_sdk_types::custom_requests::{AgentMention, SourceType};
use std::path::PathBuf;

impl GooseAcpAgent {
    pub(super) async fn on_list_agent_mentions(
        &self,
        req: ListAgentMentionsRequest,
    ) -> Result<ListAgentMentionsResponse, agent_client_protocol::Error> {
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

        let agents = crate::agents::platform_extensions::summon::discover_filesystem_sources(&cwd)
            .into_iter()
            .filter(|source| {
                matches!(
                    source.source_type,
                    SourceType::Agent | SourceType::Recipe | SourceType::Subrecipe
                ) && !source.content.is_empty()
            })
            .map(|source| {
                let mention = format!("@{}", source.name);
                AgentMention {
                    name: source.name,
                    description: source.description,
                    source_type: source.source_type,
                    source_path: (!source.path.is_empty()).then_some(source.path),
                    mention,
                }
            })
            .collect();

        Ok(ListAgentMentionsResponse { agents })
    }
}
