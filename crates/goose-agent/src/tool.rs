use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use goose_provider_types::conversation::{
    message::{Message, MessageContent, ToolRequest},
    Conversation,
};
use rmcp::{
    handler::server::router::tool::{AsyncTool, SyncTool, ToolBase},
    model::{CallToolRequestParams, CallToolResult, ErrorData, JsonObject, Tool},
};
use serde_json::{json, Value};

use crate::operation::{
    applied, messages_since_kickoff, not_applicable, Emitter, InferenceTools, Operation,
    OperationFuture, OperationResult,
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

fn pending_requests(messages: &[Message], tool_names: &HashSet<&str>) -> Vec<ToolRequest> {
    let answered: HashSet<&str> = messages
        .iter()
        .flat_map(Message::get_tool_response_ids)
        .collect();

    messages
        .iter()
        .flat_map(|message| &message.content)
        .filter_map(|content| match content {
            MessageContent::ToolRequest(request)
                if !request.was_executed_externally()
                    && !answered.contains(request.id.as_str())
                    && request
                        .tool_call
                        .as_ref()
                        .map_or(true, |call| tool_names.contains(call.name.as_ref())) =>
            {
                Some(request.clone())
            }
            _ => None,
        })
        .collect()
}

fn interrupted_result() -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::error(vec![
        rmcp::model::ContentBlock::text("Tool call was interrupted before completing"),
    ]))
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

/// Supplies tools whose definitions and implementations may vary by session.
#[async_trait]
pub trait ToolProvider<S>: Send + Sync {
    async fn tools(&self, session: &S) -> Result<Vec<Tool>>;

    async fn call(
        &self,
        session: &S,
        request_id: &str,
        call: CallToolRequestParams,
        emit: &Emitter,
    ) -> Result<CallToolResult, ErrorData>;
}

type ToolHandler<S> = dyn for<'a> Fn(&'a S, Option<JsonObject>) -> OperationFuture<'a, Result<CallToolResult, ErrorData>>
    + Send
    + Sync;

struct RegisteredTool<S> {
    definition: Tool,
    handler: Arc<ToolHandler<S>>,
}

struct RegisteredToolProvider<S> {
    tools: Vec<RegisteredTool<S>>,
}

#[async_trait]
impl<S> ToolProvider<S> for RegisteredToolProvider<S>
where
    S: Send + Sync + 'static,
{
    async fn tools(&self, _session: &S) -> Result<Vec<Tool>> {
        Ok(self
            .tools
            .iter()
            .map(|tool| tool.definition.clone())
            .collect())
    }

    async fn call(
        &self,
        session: &S,
        _request_id: &str,
        call: CallToolRequestParams,
        _emit: &Emitter,
    ) -> Result<CallToolResult, ErrorData> {
        let tool = self
            .tools
            .iter()
            .find(|tool| tool.definition.name == call.name)
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("unknown tool {}", call.name), None)
            })?;
        (tool.handler)(session, call.arguments).await
    }
}

/// An agent operation that advertises and dispatches tools.
///
/// Tools can be registered from rmcp's typed tool traits, or supplied at runtime
/// by a [`ToolProvider`] whose definitions may vary by session.
pub struct ToolOperation<S> {
    registered: RegisteredToolProvider<S>,
    providers: Vec<Arc<dyn ToolProvider<S>>>,
}

impl<S> ToolOperation<S>
where
    S: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            registered: RegisteredToolProvider { tools: Vec::new() },
            providers: Vec::new(),
        }
    }

    fn register(&mut self, tool: RegisteredTool<S>) {
        if let Some(existing) = self
            .registered
            .tools
            .iter_mut()
            .find(|existing| existing.definition.name == tool.definition.name)
        {
            *existing = tool;
        } else {
            self.registered.tools.push(tool);
        }
    }

    pub fn with_provider(mut self, provider: Arc<dyn ToolProvider<S>>) -> Self {
        self.providers.push(provider);
        self
    }

    pub fn with_sync_tool<T>(mut self) -> Self
    where
        S: Clone,
        T: SyncTool<S> + Send + Sync + 'static,
    {
        self.register(RegisteredTool {
            definition: definition::<T>(),
            handler: Arc::new(|session, arguments| {
                let session = session.clone();
                Box::pin(async move {
                    let parameters = parameters::<T>(arguments)?;
                    tokio::task::spawn_blocking(move || {
                        result::<T>(T::invoke(&session, parameters))
                    })
                    .await
                    .map_err(|error| {
                        ErrorData::internal_error(
                            format!("synchronous tool task failed: {error}"),
                            None,
                        )
                    })?
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

    async fn available_tools(
        &self,
        session: &S,
        emit: &Emitter,
    ) -> Result<Vec<(Tool, &dyn ToolProvider<S>)>> {
        let mut available = self
            .registered
            .tools(session)
            .await?
            .into_iter()
            .map(|tool| (tool, &self.registered as &dyn ToolProvider<S>))
            .collect::<Vec<_>>();
        for provider in &self.providers {
            let tools = tokio::select! {
                biased;
                _ = emit.cancelled() => anyhow::bail!("tool discovery cancelled"),
                tools = provider.tools(session) => tools?,
            };
            for tool in tools {
                if available
                    .iter()
                    .any(|(available, _)| available.name == tool.name)
                {
                    anyhow::bail!("multiple tool providers registered '{}'", tool.name);
                }
                available.push((tool, provider.as_ref()));
            }
        }
        Ok(available)
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

    async fn inference_tools(
        &self,
        session: &S,
        _conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<InferenceTools> {
        let available = self.available_tools(session, emit).await?;
        let routes = available
            .iter()
            .map(|(tool, provider)| {
                let provider_index = self
                    .providers
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate.as_ref(), *provider))
                    .map_or(0, |index| index + 1);
                (tool.name.to_string(), serde_json::json!(provider_index))
            })
            .collect();
        Ok(InferenceTools {
            tools: available.into_iter().map(|(tool, _)| tool).collect(),
            message_notes: serde_json::Map::from_iter([(
                "routes".to_string(),
                Value::Object(routes),
            )]),
        })
    }

    async fn run(
        &self,
        session: &S,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<E>> {
        let advertised = messages_since_kickoff(conversation)?
            .iter()
            .rev()
            .find_map(|message| {
                (message.role == rmcp::model::Role::Assistant)
                    .then(|| message.metadata.operation_note("tools", "routes"))
                    .flatten()
            })
            .and_then(Value::as_object)
            .map(|routes| {
                routes
                    .iter()
                    .map(|(name, index)| {
                        let index = index.as_u64().ok_or_else(|| {
                            anyhow::anyhow!("invalid persisted provider route for '{name}'")
                        })?;
                        let index = usize::try_from(index)?;
                        if index > self.providers.len() {
                            anyhow::bail!("persisted provider route for '{name}' is unavailable");
                        }
                        Ok((name.clone(), index))
                    })
                    .collect::<Result<HashMap<_, _>>>()
            })
            .transpose()?;
        let available = if advertised.is_none() {
            Some(self.available_tools(session, emit).await?)
        } else {
            None
        };
        let tool_names = advertised
            .as_ref()
            .map(|providers| providers.keys().map(String::as_str).collect())
            .unwrap_or_else(|| {
                available
                    .as_ref()
                    .expect("tools are discovered when no advertised set exists")
                    .iter()
                    .map(|(tool, _)| tool.name.as_ref())
                    .collect()
            });
        let requests = pending_requests(messages_since_kickoff(conversation)?, &tool_names);
        if requests.is_empty() {
            return not_applicable();
        }

        let mut message = Message::user();
        let mut cancelled = false;
        for request in requests {
            let tool_result = if cancelled || emit.cancel_token().is_cancelled() {
                cancelled = true;
                interrupted_result()
            } else {
                match request.tool_call.as_ref() {
                    Err(error) => Err(error.clone()),
                    Ok(call) => {
                        let provider = advertised
                            .as_ref()
                            .and_then(|providers| providers.get(call.name.as_ref()))
                            .map(|index| {
                                if *index == 0 {
                                    &self.registered as &dyn ToolProvider<S>
                                } else {
                                    self.providers[*index - 1].as_ref()
                                }
                            })
                            .or_else(|| {
                                available
                                    .as_ref()
                                    .and_then(|tools| {
                                        tools.iter().find(|(tool, _)| tool.name == call.name)
                                    })
                                    .map(|(_, provider)| *provider)
                            })
                            .expect("matched requests reference an advertised tool");
                        tokio::select! {
                            biased;
                            _ = emit.cancelled() => {
                                cancelled = true;
                                interrupted_result()
                            },
                            result = provider.call(session, &request.id, call.clone(), emit) => result,
                        }
                    }
                }
            };
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
