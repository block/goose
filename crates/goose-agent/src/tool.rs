use std::{collections::HashSet, future::Future, marker::PhantomData, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use goose_provider_types::conversation::{
    message::{Message, MessageContent, ToolRequest},
    Conversation,
};
use rmcp::{
    handler::server::router::tool::{AsyncTool, SyncTool, ToolBase},
    model::{CallToolResult, ErrorData, JsonObject, Tool},
};
use serde_json::{json, Value};

use crate::operation::{applied, not_applicable, Emitter, Operation, OperationResult};

fn empty_input_schema() -> Arc<JsonObject> {
    Arc::new(
        serde_json::from_value(json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }))
        .expect("empty tool input schema is an object"),
    )
}

fn definition<T: ToolBase>() -> Tool {
    let mut tool = Tool::new_with_raw(
        T::name(),
        T::description(),
        T::input_schema().unwrap_or_else(empty_input_schema),
    );
    if let Some(title) = T::title() {
        tool = tool.with_title(title);
    }
    if let Some(output_schema) = T::output_schema() {
        tool = tool.with_raw_output_schema(output_schema);
    }
    if let Some(annotations) = T::annotations() {
        tool = tool.with_annotations(annotations);
    }
    if let Some(icons) = T::icons() {
        tool = tool.with_icons(icons);
    }
    if let Some(meta) = T::meta() {
        tool = tool.with_meta(meta);
    }
    tool
}

fn pending_requests(conversation: &Conversation, tool_name: &str) -> Vec<ToolRequest> {
    let answered: HashSet<&str> = conversation
        .messages()
        .iter()
        .flat_map(Message::get_tool_response_ids)
        .collect();

    conversation
        .messages()
        .iter()
        .flat_map(|message| &message.content)
        .filter_map(|content| match content {
            MessageContent::ToolRequest(request)
                if !answered.contains(request.id.as_str())
                    && request
                        .tool_call
                        .as_ref()
                        .is_ok_and(|call| call.name == tool_name) =>
            {
                Some(request.clone())
            }
            _ => None,
        })
        .collect()
}

fn parameters<T: ToolBase>(request: &ToolRequest) -> Result<T::Parameter, ErrorData> {
    if T::input_schema().is_none() {
        return Ok(T::Parameter::default());
    }

    let arguments = request
        .tool_call
        .as_ref()
        .expect("matched requests have parsed tool calls")
        .arguments
        .clone()
        .unwrap_or_default();
    serde_json::from_value(Value::Object(arguments)).map_err(|error| {
        ErrorData::invalid_params(format!("failed to deserialize parameters: {error}"), None)
    })
}

fn result<T: ToolBase>(output: Result<T::Output, T::Error>) -> Result<CallToolResult, ErrorData> {
    let output = output.map_err(Into::into)?;
    let value = serde_json::to_value(output).map_err(|error| {
        ErrorData::internal_error(format!("failed to serialize tool output: {error}"), None)
    })?;
    Ok(CallToolResult::structured(value))
}

async fn response<E, F, Fut>(
    requests: Vec<ToolRequest>,
    emit: &Emitter,
    mut execute: F,
) -> Result<OperationResult<E>>
where
    E: From<Message>,
    F: FnMut(ToolRequest) -> Fut,
    Fut: Future<Output = Result<CallToolResult, ErrorData>>,
{
    if requests.is_empty() {
        return not_applicable();
    }

    let mut message = Message::user();
    for request in requests {
        let metadata = request.metadata.clone();
        let id = request.id.clone();
        let tool_result = execute(request).await;
        message.add_tool_response_with_metadata(id, tool_result, metadata.as_ref());
    }
    let message = emit.message(message).await;
    applied([E::from(message)])
}

pub struct SyncToolOperation<T> {
    definition: Tool,
    tool: PhantomData<fn() -> T>,
}

impl<T: ToolBase> SyncToolOperation<T> {
    pub fn new() -> Self {
        Self {
            definition: definition::<T>(),
            tool: PhantomData,
        }
    }
}

impl<T: ToolBase> Default for SyncToolOperation<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<S, T, E> Operation<S, E> for SyncToolOperation<T>
where
    S: Send + Sync + 'static,
    T: SyncTool<S> + Send + Sync + 'static,
    E: From<Message> + Send + 'static,
{
    fn name(&self) -> &'static str {
        "sync_tool"
    }

    async fn inference_tools(&self, _session: &S) -> Result<Vec<Tool>> {
        Ok(vec![self.definition.clone()])
    }

    async fn run(
        &self,
        session: &S,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<E>> {
        let requests = pending_requests(conversation, &self.definition.name);
        response(requests, emit, |request| async move {
            let parameters = parameters::<T>(&request)?;
            result::<T>(T::invoke(session, parameters))
        })
        .await
    }
}

pub struct AsyncToolOperation<T> {
    definition: Tool,
    tool: PhantomData<fn() -> T>,
}

impl<T: ToolBase> AsyncToolOperation<T> {
    pub fn new() -> Self {
        Self {
            definition: definition::<T>(),
            tool: PhantomData,
        }
    }
}

impl<T: ToolBase> Default for AsyncToolOperation<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<S, T, E> Operation<S, E> for AsyncToolOperation<T>
where
    S: Send + Sync + 'static,
    T: AsyncTool<S> + Send + Sync + 'static,
    E: From<Message> + Send + 'static,
{
    fn name(&self) -> &'static str {
        "async_tool"
    }

    async fn inference_tools(&self, _session: &S) -> Result<Vec<Tool>> {
        Ok(vec![self.definition.clone()])
    }

    async fn run(
        &self,
        session: &S,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<E>> {
        let requests = pending_requests(conversation, &self.definition.name);
        response(requests, emit, |request| async move {
            let parameters = parameters::<T>(&request)?;
            result::<T>(T::invoke(session, parameters).await)
        })
        .await
    }
}
