use crate::error::Result;
use crate::types::requests::CallToolRequest;
use crate::types::responses::CallToolResponse;
use crate::GooseClient;
use goose::agents::extension::ToolInfo;

impl GooseClient {
    pub async fn list_tools(
        &self,
        session_id: impl AsRef<str>,
        extension_name: Option<&str>,
    ) -> Result<Vec<ToolInfo>> {
        let sid = session_id.as_ref();
        match extension_name {
            Some(ext) => {
                self.http
                    .get_with_query(
                        "/agent/tools",
                        &[("session_id", sid), ("extension_name", ext)],
                    )
                    .await
            }
            None => {
                self.http
                    .get_with_query("/agent/tools", &[("session_id", sid)])
                    .await
            }
        }
    }

    pub async fn call_tool(
        &self,
        session_id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Result<CallToolResponse> {
        self.http
            .post(
                "/agent/call_tool",
                &CallToolRequest {
                    session_id: session_id.into(),
                    name: name.into(),
                    arguments,
                },
            )
            .await
    }
}
