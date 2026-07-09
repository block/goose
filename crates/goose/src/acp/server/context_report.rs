use super::*;
use crate::token_counter::create_token_counter;

impl GooseAcpAgent {
    pub(super) async fn on_get_context_report(
        &self,
        req: ContextReportRequest,
    ) -> Result<ContextReportResponse, agent_client_protocol::Error> {
        let agent = self.get_session_agent(&req.session_id).await?;
        let session = self
            .session_manager
            .get_session(&req.session_id, false)
            .await
            .internal_err()?;
        let token_counter = create_token_counter().await.internal_err()?;

        agent
            .build_context_report(&req.session_id, &session.working_dir, &token_counter)
            .await
            .internal_err()
    }
}
