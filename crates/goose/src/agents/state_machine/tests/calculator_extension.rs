use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, Content, ErrorData, Implementation, InitializeResult, JsonObject,
    ListToolsResult, ServerCapabilities, ServerNotification, Tool,
};
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::action_required_manager::{ActionRequiredManager, ElicitationOutcome};
use crate::agents::mcp_client::{Error as McpError, McpClientTrait};
use crate::agents::tool_execution::ToolCallContext;

pub(super) const ADD: &str = "calculator__add";
pub(super) const DIVIDE: &str = "calculator__divide";
pub(super) const REQUEST_VALUE: &str = "calculator__request_value";

pub(super) fn value(value: i64) -> Value {
    json!({ "value": value })
}

#[derive(Deserialize, JsonSchema)]
struct ValueParams {
    value: i64,
}

#[derive(JsonSchema)]
struct RequestValueParams {}

pub(super) struct CalculatorExtension {
    info: InitializeResult,
    action_required: Arc<ActionRequiredManager>,
    total: Mutex<i64>,
    barrier: Mutex<Option<Arc<tokio::sync::Barrier>>>,
    notification_senders: Mutex<Vec<mpsc::Sender<ServerNotification>>>,
}

impl CalculatorExtension {
    pub(super) fn new(action_required: Arc<ActionRequiredManager>) -> Self {
        Self {
            info: InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
                .with_server_info(Implementation::new(
                    "calculator".to_string(),
                    "test".to_string(),
                )),
            action_required,
            total: Mutex::new(0),
            barrier: Mutex::new(None),
            notification_senders: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn total(&self) -> i64 {
        *self.total.lock().unwrap()
    }

    pub(super) fn synchronize(&self, calls: usize) {
        *self.barrier.lock().unwrap() = Some(Arc::new(tokio::sync::Barrier::new(calls)));
    }
}

#[async_trait]
impl McpClientTrait for CalculatorExtension {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, McpError> {
        let schema = serde_json::to_value(schema_for!(ValueParams))
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        let request_value_schema = serde_json::to_value(schema_for!(RequestValueParams))
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        Ok(ListToolsResult {
            tools: vec![
                Tool::new(
                    "add",
                    "Add a value to the running total",
                    Arc::new(schema.clone()),
                ),
                Tool::new(
                    "divide",
                    "Divide the running total by a value",
                    Arc::new(schema.clone()),
                ),
                Tool::new(
                    "multiply",
                    "Multiply the running total by a value",
                    Arc::new(schema.clone()),
                ),
                Tool::new(
                    "subtract",
                    "Subtract a value from the running total",
                    Arc::new(schema),
                ),
                Tool::new(
                    "request_value",
                    "Ask the user for a value",
                    Arc::new(request_value_schema),
                ),
            ],
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<JsonObject>,
        _cancel_token: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        if name == "request_value" {
            let tool_call_request_id = ctx.tool_call_request_id.clone().ok_or_else(|| {
                McpError::McpError(ErrorData::invalid_params(
                    "request_value requires a tool call id",
                    None,
                ))
            })?;
            let schema = serde_json::to_value(schema_for!(ValueParams)).unwrap();
            let outcome = self
                .action_required
                .request_and_wait(
                    ctx.session_id.clone(),
                    tool_call_request_id,
                    "What value should I use?".to_string(),
                    schema,
                    std::time::Duration::from_secs(10),
                )
                .await
                .map_err(|error| {
                    McpError::McpError(ErrorData::internal_error(error.to_string(), None))
                })?;
            return Ok(match outcome {
                ElicitationOutcome::Accept(value) => {
                    let params: ValueParams = serde_json::from_value(value).map_err(|error| {
                        McpError::McpError(ErrorData::invalid_params(error.to_string(), None))
                    })?;
                    *self.total.lock().unwrap() = params.value;
                    CallToolResult::success(vec![Content::text(format!(
                        "result: {}",
                        params.value
                    ))])
                }
                ElicitationOutcome::Decline => {
                    CallToolResult::error(vec![Content::text("elicitation declined")])
                }
                ElicitationOutcome::Cancel => {
                    CallToolResult::error(vec![Content::text("elicitation cancelled")])
                }
            });
        }
        let calculate: fn(i64, i64) -> Option<i64> = match name {
            "add" => i64::checked_add,
            "divide" => i64::checked_div,
            "multiply" => i64::checked_mul,
            "subtract" => i64::checked_sub,
            _ => {
                return Err(McpError::McpError(ErrorData::invalid_params(
                    format!("unknown calculator tool {name:?}"),
                    None,
                )));
            }
        };
        let params: ValueParams = serde_json::from_value(Value::Object(
            arguments.unwrap_or_default(),
        ))
        .map_err(|error| McpError::McpError(ErrorData::invalid_params(error.to_string(), None)))?;
        let barrier = self.barrier.lock().unwrap().clone();
        if let Some(barrier) = barrier {
            let result = tokio::time::timeout(std::time::Duration::from_secs(2), barrier.wait())
                .await
                .map_err(|_| {
                    McpError::McpError(ErrorData::internal_error(
                        "calculator calls did not execute concurrently",
                        None,
                    ))
                })?;
            if result.is_leader() {
                *self.barrier.lock().unwrap() = None;
            }
        }
        let mut total = self.total.lock().unwrap();
        *total = calculate(*total, params.value).ok_or_else(|| {
            McpError::McpError(ErrorData::invalid_params(
                "calculator operation failed",
                None,
            ))
        })?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "result: {}",
            *total
        ))]))
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        let (tx, rx) = mpsc::channel(32);
        self.notification_senders.lock().unwrap().push(tx);
        rx
    }
}
