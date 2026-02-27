use crate::error::Result;
use crate::types::requests::ConfirmToolActionRequest;
use crate::GooseClient;
use goose::permission::permission_confirmation::Permission;

impl GooseClient {
    /// Confirm or deny a pending tool action request.
    ///
    /// `id` is the tool request ID received in the SSE stream's `MessageEvent::Message`
    /// content as a `ToolRequest`. `action` is typically `Permission::AllowOnce`,
    /// `Permission::DenyOnce`, or `Permission::AlwaysAllow`.
    pub async fn confirm_tool_action(
        &self,
        id: impl Into<String>,
        action: Permission,
        session_id: impl Into<String>,
    ) -> Result<()> {
        self.http
            .post_empty(
                "/action-required/tool-confirmation",
                &ConfirmToolActionRequest::new(id, action, session_id),
            )
            .await
    }
}
