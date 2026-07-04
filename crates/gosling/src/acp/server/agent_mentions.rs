use super::*;
use gosling_sdk_types::custom_requests::{AgentMention, SourceType};
use std::collections::HashSet;
use std::path::PathBuf;

impl GoslingAcpAgent {
    pub(super) async fn on_list_agent_mentions(
        &self,
        req: ListAgentMentionsRequest,
    ) -> Result<ListAgentMentionsResponse, agent_client_protocol::Error> {
        let session = if let Some(session_id) = req
            .session_id
            .as_deref()
            .map(str::trim)
            .filter(|session_id| !session_id.is_empty())
        {
            Some(
                self.session_manager
                    .get_session(session_id, false)
                    .await
                    .map_err(|_| {
                        agent_client_protocol::Error::resource_not_found(Some(
                            session_id.to_string(),
                        ))
                        .data(format!("Session not found: {}", session_id))
                    })?,
            )
        } else {
            None
        };

        let cwd = if let Some(cwd) = req
            .cwd
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            PathBuf::from(cwd)
        } else if let Some(session) = &session {
            session.working_dir.clone()
        } else {
            return Err(agent_client_protocol::Error::invalid_params()
                .data("Either cwd or sessionId is required"));
        };

        let filesystem_sources =
            crate::agents::platform_extensions::summon::discover_filesystem_sources(&cwd);
        let mut sources = Vec::new();
        let mut seen = HashSet::new();

        for source in filesystem_sources {
            if seen.insert(source.name.clone()) {
                sources.push(source);
            }
        }

        let agents = sources
            .into_iter()
            .filter(|source| {
                matches!(source.source_type, SourceType::Agent) && !source.content.is_empty()
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
