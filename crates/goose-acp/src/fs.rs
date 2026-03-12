use async_trait::async_trait;
use fs_err as fs;
use goose::agents::mcp_client::{Error as McpError, McpClientTrait};
use goose::agents::platform_extensions::developer::edit::{
    resolve_path, string_replace, FileEditParams, FileReadParams, FileWriteParams,
};
use goose::agents::platform_extensions::developer::DeveloperClient;
use rmcp::model::{CallToolResult, Content as RmcpContent, Tool, ToolAnnotations};
use sacp::schema::{ReadTextFileRequest, SessionId, WriteTextFileRequest};
use sacp::{AgentToClient, JrConnectionCx};
use schemars::schema_for;
use std::path::Path;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

async fn acp_read_text_file(
    cx: &JrConnectionCx<AgentToClient>,
    session_id: &SessionId,
    path: &Path,
    line: Option<u32>,
    limit: Option<u32>,
) -> Result<String, String> {
    let mut request = ReadTextFileRequest::new(session_id.clone(), path.to_path_buf());
    if let Some(l) = line {
        request = request.line(l);
    }
    if let Some(l) = limit {
        request = request.limit(l);
    }
    let response = cx
        .send_request(request)
        .block_task()
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(response.content)
}

async fn acp_write_text_file(
    cx: &JrConnectionCx<AgentToClient>,
    session_id: &SessionId,
    path: &Path,
    content: &str,
) -> Result<(), String> {
    let request =
        WriteTextFileRequest::new(session_id.clone(), path.to_path_buf(), content.to_string());
    cx.send_request(request)
        .block_task()
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

pub(crate) struct AcpTools {
    pub(crate) inner: Arc<dyn McpClientTrait>,
    pub(crate) cx: JrConnectionCx<AgentToClient>,
    pub(crate) session_id: SessionId,
    pub(crate) fs_read: bool,
    pub(crate) fs_write: bool,
}

fn error_result(msg: impl std::fmt::Display) -> CallToolResult {
    CallToolResult::error(vec![RmcpContent::text(msg.to_string()).with_priority(0.0)])
}

fn fail(action: &str, path: &str, err: impl std::fmt::Display) -> CallToolResult {
    error_result(format!("Failed to {action} {path}: {err}"))
}

fn read_tool() -> Tool {
    let schema = serde_json::to_value(schema_for!(FileReadParams))
        .expect("schema serialization should succeed")
        .as_object()
        .expect("schema should serialize to an object")
        .clone();
    Tool::new("read", "Read a text file from disk.", schema).annotate(
        ToolAnnotations::with_title("Read")
            .read_only(true)
            .destructive(false)
            .idempotent(false)
            .open_world(false),
    )
}

pub(crate) fn with_location_meta(
    mut result: CallToolResult,
    path: &Path,
    line: Option<u32>,
) -> CallToolResult {
    let location = serde_json::json!({
        "tool_locations": [{"path": path.to_string_lossy(), "line": line}]
    });
    result.meta = Some(serde_json::from_value(location).unwrap());
    result
}

impl AcpTools {
    fn parse_args<T: serde::de::DeserializeOwned>(
        arguments: Option<rmcp::model::JsonObject>,
    ) -> Result<T, String> {
        DeveloperClient::parse_args(arguments).map_err(|e| format!("Error: {e}"))
    }

    async fn read_content(&self, path: &Path) -> Result<String, String> {
        if self.fs_read {
            acp_read_text_file(&self.cx, &self.session_id, path, None, None).await
        } else {
            fs::read_to_string(path).map_err(|e| e.to_string())
        }
    }

    async fn acp_read(
        &self,
        arguments: Option<rmcp::model::JsonObject>,
        ctx: &goose::agents::ToolCallContext,
    ) -> Result<CallToolResult, McpError> {
        let params: FileReadParams = match Self::parse_args(arguments) {
            Ok(p) => p,
            Err(e) => return Ok(error_result(e)),
        };
        let path = resolve_path(&params.path, ctx.working_dir.as_deref());
        match acp_read_text_file(&self.cx, &self.session_id, &path, params.line, params.limit).await
        {
            Ok(content) => Ok(with_location_meta(
                CallToolResult::success(vec![RmcpContent::text(content).with_priority(0.0)]),
                &path,
                params.line,
            )),
            Err(e) => Ok(fail("read", &params.path, e)),
        }
    }

    async fn acp_write(
        &self,
        arguments: Option<rmcp::model::JsonObject>,
        ctx: &goose::agents::ToolCallContext,
    ) -> Result<CallToolResult, McpError> {
        let params: FileWriteParams = match Self::parse_args(arguments) {
            Ok(p) => p,
            Err(e) => return Ok(error_result(e)),
        };
        let path = resolve_path(&params.path, ctx.working_dir.as_deref());
        match acp_write_text_file(&self.cx, &self.session_id, &path, &params.content).await {
            Ok(()) => {
                let line_count = params.content.lines().count();
                let action = if path.exists() { "Wrote" } else { "Created" };
                Ok(with_location_meta(
                    CallToolResult::success(vec![RmcpContent::text(format!(
                        "{action} {} ({line_count} lines)",
                        params.path
                    ))
                    .with_priority(0.0)]),
                    &path,
                    Some(1),
                ))
            }
            Err(e) => Ok(fail("write", &params.path, e)),
        }
    }

    async fn acp_edit(
        &self,
        arguments: Option<rmcp::model::JsonObject>,
        ctx: &goose::agents::ToolCallContext,
    ) -> Result<CallToolResult, McpError> {
        let params: FileEditParams = match Self::parse_args(arguments) {
            Ok(p) => p,
            Err(e) => return Ok(error_result(e)),
        };
        let path = resolve_path(&params.path, ctx.working_dir.as_deref());

        let content = match self.read_content(&path).await {
            Ok(c) => c,
            Err(e) => return Ok(fail("read", &params.path, e)),
        };

        let new_content = match string_replace(&content, &params.before, &params.after) {
            Ok(c) => c,
            Err(msg) => return Ok(error_result(msg)),
        };

        let write_result = if self.fs_write {
            acp_write_text_file(&self.cx, &self.session_id, &path, &new_content).await
        } else {
            fs::write(&path, &new_content).map_err(|e| e.to_string())
        };

        match write_result {
            Ok(()) => {
                let old_lines = params.before.lines().count();
                let new_lines = params.after.lines().count();
                Ok(with_location_meta(
                    CallToolResult::success(vec![RmcpContent::text(format!(
                        "Edited {} ({old_lines} lines -> {new_lines} lines)",
                        params.path
                    ))
                    .with_priority(0.0)]),
                    &path,
                    Some(1),
                ))
            }
            Err(e) => Ok(fail("write", &params.path, e)),
        }
    }
}

#[async_trait]
impl McpClientTrait for AcpTools {
    async fn list_tools(
        &self,
        session_id: &str,
        next_cursor: Option<String>,
        cancellation_token: CancellationToken,
    ) -> Result<rmcp::model::ListToolsResult, McpError> {
        let mut result = self
            .inner
            .list_tools(session_id, next_cursor, cancellation_token)
            .await?;
        if self.fs_read {
            result.tools.insert(0, read_tool());
        }
        Ok(result)
    }

    async fn call_tool(
        &self,
        ctx: &goose::agents::ToolCallContext,
        name: &str,
        arguments: Option<rmcp::model::JsonObject>,
        cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        match name {
            "read" if self.fs_read => self.acp_read(arguments, ctx).await,
            "write" if self.fs_write => self.acp_write(arguments, ctx).await,
            "edit" if self.fs_read && self.fs_write => self.acp_edit(arguments, ctx).await,
            _ => {
                self.inner
                    .call_tool(ctx, name, arguments, cancellation_token)
                    .await
            }
        }
    }

    fn get_info(&self) -> Option<&rmcp::model::InitializeResult> {
        self.inner.get_info()
    }
}
