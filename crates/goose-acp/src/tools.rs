use std::{cell::LazyCell, fmt::Display, future::Future, path::PathBuf};

use agent_client_protocol_schema::{
    CreateTerminalRequest, ReadTextFileRequest, SessionId, SessionNotification, SessionUpdate,
    Terminal, TerminalOutputRequest, ToolCallContent, ToolCallUpdate, ToolCallUpdateFields,
    WaitForTerminalExitRequest, WriteTextFileRequest,
};
use async_trait::async_trait;
use goose::agents::mcp_client::McpClientTrait;
use goose_mcp::developer::text_editor::{text_editor_insert_inmem, text_editor_replace_inmem};
use rmcp::{
    model::{CallToolResult, Content, InitializeResult, JsonObject, ListToolsResult, Tool},
    ErrorData, ServiceError,
};
use sacp::{AgentToClient, JrConnectionCx};
use schemars::{schema_for, JsonSchema};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

pub struct AcpTools {
    cx: JrConnectionCx<AgentToClient>,
    session_id: sacp::schema::SessionId,
    working_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ReadParams {
    /// The path to the file to read.
    path: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WriteParams {
    /// The path to the file to be written.
    path: String,
    /// The text to be written to the file.
    text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct StrReplaceParams {
    /// The path to the file to be modified.
    path: String,
    /// The string to be replaced.
    old_str: String,
    /// The string to replace with.
    new_str: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct InsertParams {
    /// The path to the file to be modified.
    path: String,
    /// The string to be inserted.
    new_str: String,
    /// The position to insert the string at.
    position: i64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ShellParams {
    /// Command to be executed.
    command: String,
}

macro_rules! schema {
    ($t:ty) => {
        serde_json::to_value(schema_for!($t))
            .unwrap()
            .as_object()
            .unwrap()
            .clone()
    };
}

const TOOL_DEFS: LazyCell<Vec<Tool>> = LazyCell::new(|| {
    vec![
        Tool::new("read", "Read files", schema!(ReadParams)),
        Tool::new("write", "Write files", schema!(WriteParams)),
        Tool::new(
            "str_replace",
            "Replace strings in files",
            schema!(StrReplaceParams),
        ),
        Tool::new("insert", "Insert strings into files", schema!(InsertParams)),
        Tool::new("shell", "Execute shell commands", schema!(ShellParams)),
    ]
});

async fn handle_tool_call<Req, Fut>(
    mut func: impl FnMut(Req) -> Fut,
    request: JsonObject,
) -> Result<CallToolResult, ServiceError>
where
    Req: DeserializeOwned,
    Fut: Future<Output = Result<CallToolResult, ServiceError>>,
{
    let args = Req::deserialize(request).map_err(|error| {
        tracing::error!("failed to deserialize tool arguments: {}", error);
        invalid_request(error)
    })?;
    func(args).await
}

async fn read_file(
    path: &PathBuf,
    cx: &JrConnectionCx<AgentToClient>,
    session_id: SessionId,
) -> Result<String, ServiceError> {
    let res = cx
        .send_request(ReadTextFileRequest::new(session_id, path))
        .block_task()
        .await
        .map_err(|_| ServiceError::McpError(ErrorData::internal_error("failed to read", None)))?;
    Ok(res.content)
}

async fn write_file(
    path: &PathBuf,
    cx: &JrConnectionCx<AgentToClient>,
    session_id: SessionId,
    content: &str,
) -> Result<(), ServiceError> {
    cx.send_request(WriteTextFileRequest::new(session_id, path, content))
        .block_task()
        .await
        .map_err(|_| ServiceError::McpError(ErrorData::internal_error("failed to write", None)))?;
    Ok(())
}

impl AcpTools {
    pub fn new(
        cx: JrConnectionCx<AgentToClient>,
        session_id: sacp::schema::SessionId,
        working_dir: PathBuf,
    ) -> Self {
        AcpTools {
            cx,
            session_id,
            working_dir,
        }
    }

    async fn read(&self, params: ReadParams) -> Result<CallToolResult, ServiceError> {
        let path = self.working_dir.join(params.path);
        let content = read_file(&path, &self.cx, self.session_id.clone()).await?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    async fn write(&self, params: WriteParams) -> Result<CallToolResult, ServiceError> {
        let path = self.working_dir.join(params.path);
        write_file(
            &path,
            &self.cx,
            self.session_id.clone(),
            params.text.as_str(),
        )
        .await?;
        Ok(CallToolResult::success(vec![Content::text("done")]))
    }

    async fn str_replace(&self, params: StrReplaceParams) -> Result<CallToolResult, ServiceError> {
        let path = self.working_dir.join(params.path);
        let content = read_file(&path, &self.cx, self.session_id.clone()).await?;
        let (content, _) = text_editor_replace_inmem(
            &path,
            content.as_str(),
            params.old_str.as_str(),
            params.new_str.as_str(),
        )
        .map_err(|_| {
            ServiceError::McpError(ErrorData::internal_error("failed to replace", None))
        })?;
        write_file(&path, &self.cx, self.session_id.clone(), content.as_str()).await?;
        Ok(CallToolResult::success(vec![Content::text("done")]))
    }

    async fn insert(&self, params: InsertParams) -> Result<CallToolResult, ServiceError> {
        let path = self.working_dir.join(params.path);
        let content = read_file(&path, &self.cx, self.session_id.clone()).await?;
        let (content, _) = text_editor_insert_inmem(
            &path,
            content.as_str(),
            params.position,
            &params.new_str.as_str(),
        )
        .map_err(|_| ServiceError::McpError(ErrorData::internal_error("failed to insert", None)))?;
        write_file(&path, &self.cx, self.session_id.clone(), content.as_str()).await?;
        Ok(CallToolResult::success(vec![Content::text("done")]))
    }

    async fn shell(&self, params: ShellParams) -> Result<CallToolResult, ServiceError> {
        let res = self
            .cx
            .send_request(CreateTerminalRequest::new(
                self.session_id.clone(),
                params.command,
            ))
            .block_task()
            .await
            .map_err(|_| {
                ServiceError::McpError(ErrorData::internal_error("failed to spawn terminal", None))
            })?;
        let terminal_id = res.terminal_id;

        self.cx
            .send_notification(SessionNotification::new(
                self.session_id.clone(),
                SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
                    tool_call_id,
                    ToolCallUpdateFields::new().content(Some(vec![ToolCallContent::Terminal(
                        Terminal::new(terminal_id),
                    )])),
                )),
            ))
            .map_err(|_| {
                ServiceError::McpError(ErrorData::internal_error("failed to spawn terminal", None))
            })?;

        let _res = self
            .cx
            .send_request(WaitForTerminalExitRequest::new(
                self.session_id.clone(),
                terminal_id.clone(),
            ))
            .block_task()
            .await
            .map_err(|_| {
                ServiceError::McpError(ErrorData::internal_error(
                    "failed to wait for terminal exit",
                    None,
                ))
            })?;
        let output_res = self
            .cx
            .send_request(TerminalOutputRequest::new(
                self.session_id.clone(),
                terminal_id.clone(),
            ))
            .block_task()
            .await
            .map_err(|_| {
                ServiceError::McpError(ErrorData::internal_error(
                    "failed to wait for terminal exit",
                    None,
                ))
            })?;

        Ok(CallToolResult::success(vec![
            Content::text(format!(
                "exit code: {}",
                output_res
                    .exit_status
                    .and_then(|s| s.exit_code)
                    .unwrap_or_default()
            )),
            Content::text(output_res.output),
        ]))
    }
}

fn invalid_request(error: impl Display) -> ServiceError {
    ServiceError::McpError(ErrorData::invalid_request(format!("{}", error), None))
}

#[async_trait]
impl McpClientTrait for AcpTools {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, ServiceError> {
        Ok(ListToolsResult::with_all_items(TOOL_DEFS.clone()))
    }

    async fn call_tool(
        &self,
        _session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        _working_dir: Option<&str>,
        _cancel_token: CancellationToken,
    ) -> Result<CallToolResult, ServiceError> {
        let args = arguments.ok_or(invalid_request("missing arguments"))?;
        match name {
            "read" => handle_tool_call(|p| self.read(p), args).await,
            "write" => handle_tool_call(|p| self.write(p), args).await,
            "str_replace" => handle_tool_call(|p| self.str_replace(p), args).await,
            "insert" => handle_tool_call(|p| self.insert(p), args).await,
            "shell" => handle_tool_call(|p| self.shell(p), args).await,
            _ => Err(invalid_request("tool not found")),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        todo!()
    }
}
