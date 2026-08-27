use std::{collections::HashSet, sync::Arc};

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

use crate::operation::{
    applied, not_applicable, Emitter, Operation, OperationFuture, OperationResult,
};

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

fn pending_requests(conversation: &Conversation, tool_names: &HashSet<&str>) -> Vec<ToolRequest> {
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
                        .is_ok_and(|call| tool_names.contains(call.name.as_ref())) =>
            {
                Some(request.clone())
            }
            _ => None,
        })
        .collect()
}

fn parameters<T: ToolBase>(arguments: Option<JsonObject>) -> Result<T::Parameter, ErrorData> {
    if T::input_schema().is_none() {
        return Ok(T::Parameter::default());
    }

    serde_json::from_value(Value::Object(arguments.unwrap_or_default())).map_err(|error| {
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

type ToolHandler<S> = dyn for<'a> Fn(&'a S, Option<JsonObject>) -> OperationFuture<'a, Result<CallToolResult, ErrorData>>
    + Send
    + Sync;

struct RegisteredTool<S> {
    definition: Tool,
    handler: Arc<ToolHandler<S>>,
}

/// An agent operation containing tools defined with rmcp's tool traits.
///
/// Callers register any number of their own [`SyncTool`] and [`AsyncTool`]
/// implementations with the builder methods. The operation advertises every
/// registered definition during inference and dispatches matching tool calls.
pub struct ToolOperation<S> {
    tools: Vec<RegisteredTool<S>>,
}

impl<S> ToolOperation<S>
where
    S: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    fn register(&mut self, tool: RegisteredTool<S>) {
        if let Some(existing) = self
            .tools
            .iter_mut()
            .find(|existing| existing.definition.name == tool.definition.name)
        {
            *existing = tool;
        } else {
            self.tools.push(tool);
        }
    }

    pub fn with_sync_tool<T>(mut self) -> Self
    where
        T: SyncTool<S> + Send + Sync + 'static,
    {
        self.register(RegisteredTool {
            definition: definition::<T>(),
            handler: Arc::new(|session, arguments| {
                Box::pin(async move {
                    let parameters = parameters::<T>(arguments)?;
                    result::<T>(T::invoke(session, parameters))
                })
            }),
        });
        self
    }

    pub fn with_async_tool<T>(mut self) -> Self
    where
        T: AsyncTool<S> + Send + Sync + 'static,
    {
        self.register(RegisteredTool {
            definition: definition::<T>(),
            handler: Arc::new(|session, arguments| {
                Box::pin(async move {
                    let parameters = parameters::<T>(arguments)?;
                    result::<T>(T::invoke(session, parameters).await)
                })
            }),
        });
        self
    }
}

impl<S> Default for ToolOperation<S>
where
    S: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<S, E> Operation<S, E> for ToolOperation<S>
where
    S: Send + Sync + 'static,
    E: From<Message> + Send + 'static,
{
    fn name(&self) -> &'static str {
        "tools"
    }

    async fn inference_tools(&self, _session: &S) -> Result<Vec<Tool>> {
        Ok(self
            .tools
            .iter()
            .map(|tool| tool.definition.clone())
            .collect())
    }

    async fn run(
        &self,
        session: &S,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<E>> {
        let tool_names = self
            .tools
            .iter()
            .map(|tool| tool.definition.name.as_ref())
            .collect();
        let requests = pending_requests(conversation, &tool_names);
        if requests.is_empty() {
            return not_applicable();
        }

        let mut message = Message::user();
        for request in requests {
            let call = request
                .tool_call
                .as_ref()
                .expect("matched requests have parsed tool calls");
            let tool = self
                .tools
                .iter()
                .find(|tool| tool.definition.name == call.name)
                .expect("matched requests reference registered tools");
            let tool_result = (tool.handler)(session, call.arguments.clone()).await;
            message.add_tool_response_with_metadata(
                request.id,
                tool_result,
                request.metadata.as_ref(),
            );
        }
        let message = emit.message(message).await;
        applied([E::from(message)])
    }
}
