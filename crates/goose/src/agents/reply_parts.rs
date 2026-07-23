use anyhow::Result;
use goose_providers::errors::ProviderError;
use regex::Regex;
use std::sync::Arc;

use async_stream::try_stream;
use futures::stream::StreamExt;
use serde_json::{json, Value};
use tracing::debug;

use super::super::agents::Agent;
#[cfg(feature = "code-mode")]
use crate::agents::platform_extensions::code_execution;
use crate::config::{Config, GooseMode};
use crate::conversation::message::{Message, MessageContent, MessageUsage, ToolRequest};
use crate::conversation::{fix_conversation, Conversation};
#[cfg(test)]
use crate::providers::base::stream_from_single_message;
use crate::providers::base::{MessageStream, Provider};
use crate::providers::toolshim::{
    augment_message_with_selected_tool_interpreter, convert_tool_messages_to_text,
    modify_system_prompt_for_tool_json, sanitize_residual_markers,
};
use goose_providers::conversation::token_usage::{CostSource, ProviderStats, ProviderUsage, Usage};
use goose_providers::model::ModelConfig;
use rmcp::model::Tool;
use tracing::warn;

async fn enhance_model_error(
    error: ProviderError,
    provider: &Arc<dyn Provider>,
    toolshim: bool,
) -> ProviderError {
    let ProviderError::RequestFailed(ref msg) = error else {
        return error;
    };

    let re = Regex::new(r"(?i)\b4\d{2}\b.*model|model.*\b4\d{2}\b").unwrap();
    if !re.is_match(msg) {
        return error;
    }

    let Ok(models) = provider.fetch_recommended_models(toolshim).await else {
        return error;
    };
    if models.is_empty() {
        return error;
    }

    ProviderError::RequestFailed(format!(
        "{}. Available models for this provider: {}",
        msg,
        models.join(", ")
    ))
}

fn coerce_value(s: &str, schema: &Value) -> Value {
    let type_str = schema.get("type");

    match type_str {
        Some(Value::String(t)) => match t.as_str() {
            "number" | "integer" => try_coerce_number(s),
            "boolean" => try_coerce_boolean(s),
            _ => Value::String(s.to_string()),
        },
        Some(Value::Array(types)) => {
            // Try each type in order
            for t in types {
                if let Value::String(type_name) = t {
                    match type_name.as_str() {
                        "number" | "integer" if s.parse::<f64>().is_ok() => {
                            return try_coerce_number(s)
                        }
                        "boolean" if matches!(s.to_lowercase().as_str(), "true" | "false") => {
                            return try_coerce_boolean(s)
                        }
                        _ => continue,
                    }
                }
            }
            Value::String(s.to_string())
        }
        _ => Value::String(s.to_string()),
    }
}

fn try_coerce_number(s: &str) -> Value {
    if let Ok(n) = s.parse::<f64>() {
        if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            json!(n as i64)
        } else {
            json!(n)
        }
    } else {
        Value::String(s.to_string())
    }
}

fn try_coerce_boolean(s: &str) -> Value {
    match s.to_lowercase().as_str() {
        "true" => json!(true),
        "false" => json!(false),
        _ => Value::String(s.to_string()),
    }
}

pub(crate) fn coerce_tool_arguments(
    arguments: Option<serde_json::Map<String, Value>>,
    tool_schema: &Value,
) -> Option<serde_json::Map<String, Value>> {
    let args = arguments?;

    let properties = tool_schema.get("properties").and_then(|p| p.as_object())?;

    let mut coerced = serde_json::Map::new();

    for (key, value) in args.iter() {
        let coerced_value =
            if let (Value::String(s), Some(prop_schema)) = (value, properties.get(key)) {
                coerce_value(s, prop_schema)
            } else {
                value.clone()
            };
        coerced.insert(key.clone(), coerced_value);
    }

    Some(coerced)
}

async fn toolshim_postprocess(
    response: Message,
    toolshim_tools: &[Tool],
) -> Result<Message, ProviderError> {
    match augment_message_with_selected_tool_interpreter(response.clone(), toolshim_tools).await {
        Ok(message) => Ok(message),
        Err(e) => {
            warn!(
                "Toolshim augmentation failed, skipping tool augmentation: {}",
                e
            );
            Ok(sanitize_residual_markers(response))
        }
    }
}

/// Fill `usage.stats` timing fields measured by the stream wrapper, keeping any
/// values the provider already reported (e.g. MLX's own `elapsed_ms`).
fn fill_stream_timing(
    usage: &mut ProviderUsage,
    request_started: std::time::Instant,
    first_content_at: Option<std::time::Instant>,
) {
    let stats = usage.stats.get_or_insert_with(ProviderStats::default);
    if stats.time_to_first_token_ms.is_none() {
        if let Some(first) = first_content_at {
            stats.time_to_first_token_ms = Some((first - request_started).as_millis() as u64);
        }
    }
    if stats.elapsed_ms.is_none() {
        stats.elapsed_ms = Some(request_started.elapsed().as_millis() as u64);
    }
}

fn message_has_timing_content(message: &Message) -> bool {
    message
        .content
        .iter()
        .any(|content| !matches!(content, MessageContent::SystemNotification(_)))
}

impl Agent {
    pub async fn prepare_tools_and_prompt(
        &self,
        session_id: &str,
        working_dir: &std::path::Path,
    ) -> Result<(Vec<Tool>, Vec<Tool>, String, ModelConfig)> {
        let mut tools = self.list_tools(session_id, None).await;

        #[cfg(feature = "code-mode")]
        let code_execution_active = self
            .extension_manager
            .is_extension_enabled(code_execution::EXTENSION_NAME)
            .await;
        #[cfg(not(feature = "code-mode"))]
        let code_execution_active = false;
        #[cfg(feature = "code-mode")]
        if code_execution_active {
            let disclosure_style =
                crate::agents::platform_extensions::code_execution::get_tool_disclosure();

            tools = tools
                .into_iter()
                .filter_map(|mut t| match disclosure_style {
                    pctx_code_mode::config::ToolDisclosure::Catalog
                    | pctx_code_mode::config::ToolDisclosure::Filesystem => {
                        // in catalog & filesystem styles, progressive search is handled
                        // by pctx, so we want to omit all non-first-class extensions
                        // from the standard tool list
                        if crate::agents::extension_manager::get_tool_owner(&t).is_some_and(|o| {
                            crate::agents::extension_manager::is_first_class_extension(&o)
                        }) || crate::agents::extension_manager::get_tool_resource_uri(&t)
                            .is_some()
                        {
                            Some(t)
                        } else {
                            None
                        }
                    }
                    pctx_code_mode::config::ToolDisclosure::Sidecar => {
                        // in sidecar style there is no progressive search, just a way to chain tools
                        // together with typescript
                        // add output schema to description since many model providers drop the
                        // output schema when presenting tools to the model
                        let output_schema = t
                            .output_schema
                            .as_ref()
                            .map(|s| serde_json::json!(s).to_string())
                            .unwrap_or("unknown".to_string());
                        let description_extension = format!(
                            "The successful return schema of this tool is:\n{output_schema}"
                        );

                        t.description = Some(
                            t.description
                                .map(|t| format!("{t}\n{description_extension}"))
                                .unwrap_or(description_extension)
                                .into(),
                        );

                        Some(t)
                    }
                })
                .collect();
        }

        // Filter out tools not visible to the model per MCP Apps visibility spec.
        // Tools with `_meta.ui.visibility` that doesn't include "model" are app-only.
        tools.retain(is_tool_visible_to_model);

        // Stable tool ordering is important for multi session prompt caching.
        tools.sort_by(|a, b| a.name.cmp(&b.name));

        // Prepare system prompt
        let extensions_info = self
            .extension_manager
            .get_extensions_info(working_dir)
            .await;
        let (extension_count, tool_count) = self.total_extension_and_tool_counts(session_id).await;

        let model_config = self.model_config_for_session(session_id).await?;

        let goose_mode = *self.current_goose_mode.lock().await;

        if goose_mode == GooseMode::SmartApprove {
            self.tool_inspection_manager.apply_tool_annotations(&tools);
        }

        let prompt_manager = self.prompt_manager.lock().await;
        let mut system_prompt = prompt_manager
            .builder()
            .with_extensions(extensions_info.into_iter())
            .with_frontend_instructions(self.frontend_instructions.lock().await.clone())
            .with_extension_and_tool_counts(extension_count, tool_count)
            .with_code_execution_mode(code_execution_active)
            .with_hints(working_dir)
            .with_goose_mode(goose_mode)
            .build();

        // Handle toolshim if enabled
        let mut toolshim_tools = vec![];
        if model_config.toolshim {
            // If tool interpretation is enabled, modify the system prompt
            system_prompt = modify_system_prompt_for_tool_json(&system_prompt, &tools);
            // Make a copy of tools before emptying
            toolshim_tools = tools.clone();
            // Empty the tools vector for provider completion
            tools = vec![];
        }

        Ok((tools, toolshim_tools, system_prompt, model_config))
    }

    #[tracing::instrument(
        skip(provider, model_config, session_id, system_prompt, messages, tools, toolshim_tools),
        fields(session.id = %session_id)
    )]
    pub(crate) async fn stream_response_from_provider(
        provider: Arc<dyn Provider>,
        model_config: ModelConfig,
        session_id: &str,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
        toolshim_tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let config = model_config.clone();

        let projected_messages =
            Conversation::new_unvalidated(messages.iter().cloned()).agent_visible_messages();
        let (filtered_messages, _) =
            fix_conversation(Conversation::new_unvalidated(projected_messages));

        // Convert tool messages to text if toolshim is enabled
        let messages_for_provider = if config.toolshim {
            convert_tool_messages_to_text(filtered_messages.messages())
        } else {
            filtered_messages
        };

        // Clone owned data to move into the async stream
        let system_prompt = system_prompt.to_owned();
        let tools = tools.to_owned();
        let toolshim_tools = toolshim_tools.to_owned();
        let provider = provider.clone();

        // Capture errors during stream creation and return them as part of the stream
        // so they can be handled by the existing error handling logic in the agent
        let model_config =
            model_config.with_default_thinking_effort(Config::global().get_goose_thinking_effort());
        let request_started = std::time::Instant::now();
        debug!("WAITING_LLM_STREAM_START");
        let stream_result = crate::session_context::with_session_id(
            Some(session_id.to_string()),
            provider.stream(
                &model_config,
                system_prompt.as_str(),
                messages_for_provider.messages(),
                &tools,
            ),
        )
        .await;
        debug!("WAITING_LLM_STREAM_END");

        // If there was an error creating the stream, return a stream that yields that error
        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                let enhanced_error = enhance_model_error(e, &provider, config.toolshim).await;
                // Return a stream that immediately yields the error
                // This allows the error to be caught by existing error handling in agent.rs
                return Ok(Box::pin(try_stream! {
                    yield Err(enhanced_error)?;
                }));
            }
        };

        Ok(Box::pin(try_stream! {
            if config.toolshim {
                // Toolshim mode: accumulate the full response before processing
                // so that tool-use markers spanning multiple chunks are detected
                // and stripped before any output reaches the UI.
                let mut accumulated_message: Option<Message> = None;
                let mut final_usage: Option<ProviderUsage> = None;
                let mut first_content_at: Option<std::time::Instant> = None;

                while let Some(result) = stream.next().await {
                    let (msg_opt, usage_opt) = result?;

                    if let Some(msg) = msg_opt {
                        if first_content_at.is_none() && message_has_timing_content(&msg) {
                            first_content_at = Some(std::time::Instant::now());
                        }
                        accumulated_message = Some(match accumulated_message {
                            Some(mut prev) => {
                                for new_content in msg.content {
                                    match (&mut prev.content.last_mut(), &new_content) {
                                        (
                                            Some(MessageContent::Text(last_text)),
                                            MessageContent::Text(new_text),
                                        ) if last_text.audience() == new_text.audience() => {
                                            last_text.text.push_str(&new_text.text);
                                        }
                                        _ => {
                                            prev.content.push(new_content);
                                        }
                                    }
                                }
                                prev
                            }
                            None => msg,
                        });
                    }

                    if let Some(usage) = usage_opt {
                        final_usage = Some(usage);
                    }

                    // Yield empty item so the agent loop can check cancellation
                    yield (None, None);
                }

                // The toolshim interpreter call below must not count toward elapsed time.
                if let Some(usage) = final_usage.as_mut() {
                    fill_stream_timing(usage, request_started, first_content_at);
                }

                if let Some(msg) = accumulated_message {
                    let processed = toolshim_postprocess(msg, &toolshim_tools).await?;
                    yield (Some(processed), final_usage);
                } else if final_usage.is_some() {
                    // Preserve usage-only responses (no message content)
                    yield (None, final_usage);
                }
            } else {
                let mut first_content_at: Option<std::time::Instant> = None;
                while let Some(result) = stream.next().await {
                    let (message, mut usage) = result?;

                    if first_content_at.is_none()
                        && message.as_ref().is_some_and(message_has_timing_content)
                    {
                        first_content_at = Some(std::time::Instant::now());
                    }
                    if let Some(usage) = usage.as_mut() {
                        fill_stream_timing(usage, request_started, first_content_at);
                    }

                    yield (message, usage);
                }
            }
        }))
    }

    /// Categorize tool requests from the response into different types
    /// Returns:
    /// - frontend_requests: Tool requests that should be handled by the frontend
    /// - other_requests: All other tool requests (including requests to enable extensions)
    /// - filtered_message: The original message with frontend tool requests removed
    pub(crate) async fn categorize_tool_requests(
        &self,
        response: &Message,
        tools: &[Tool],
        suppress_replayed_thinking: bool,
    ) -> (Vec<ToolRequest>, Vec<ToolRequest>, Message) {
        // First collect all tool requests with coercion applied
        let tool_requests: Vec<ToolRequest> = response
            .content
            .iter()
            .filter_map(|content| {
                if let MessageContent::ToolRequest(req) = content {
                    let mut coerced_req = req.clone();

                    if let Ok(ref mut tool_call) = coerced_req.tool_call {
                        if let Some(tool) = tools.iter().find(|t| t.name == tool_call.name) {
                            let schema_value = Value::Object(tool.input_schema.as_ref().clone());
                            tool_call.arguments =
                                coerce_tool_arguments(tool_call.arguments.clone(), &schema_value);

                            if let Some(ref meta) = tool.meta {
                                // Merge registry meta into existing tool_meta;
                                // existing keys win so provider markers (e.g.
                                // goose.external_dispatch) survive coercion.
                                let new_meta = serde_json::to_value(meta).ok();
                                coerced_req.tool_meta =
                                    match (coerced_req.tool_meta.take(), new_meta) {
                                        (
                                            Some(Value::Object(mut existing)),
                                            Some(Value::Object(new)),
                                        ) => {
                                            for (k, v) in new {
                                                existing.entry(k).or_insert(v);
                                            }
                                            Some(Value::Object(existing))
                                        }
                                        (None, new) => new,
                                        (existing, _) => existing,
                                    };
                            }
                        }
                    }

                    Some(coerced_req)
                } else {
                    None
                }
            })
            .collect();

        // Providers should emit unique tool-call ids within a turn, but a
        // malformed or malicious provider can repeat one. Keep only the first
        // occurrence of each id, in the order the provider sent them, so tools
        // aren't executed twice and duplicate tool_results don't pollute the
        // conversation history.
        let mut seen_ids = std::collections::HashSet::new();
        let tool_requests: Vec<ToolRequest> = tool_requests
            .into_iter()
            .filter(|req| seen_ids.insert(req.id.clone()))
            .collect();

        let has_tool_requests = !tool_requests.is_empty();
        let should_suppress_replayed_thinking = suppress_replayed_thinking && has_tool_requests;

        // Create a filtered message with frontend tool requests removed.
        // When a response contains tool calls, keep reasoning in the original
        // message for provider/state purposes but only suppress it from the
        // user-visible filtered message if the caller already surfaced
        // thinking earlier in this provider turn. That avoids replaying full
        // accumulated reasoning after streamed thought chunks while still
        // preserving final-only non-streaming thoughts.
        let mut filtered_content = Vec::new();
        let mut deduped_requests = tool_requests.iter();
        let mut next_request = deduped_requests.next();

        for content in &response.content {
            match content {
                MessageContent::ToolRequest(req) => {
                    // Drop content for requests removed during dedup so duplicate
                    // ids don't survive into the filtered (history) message.
                    let Some(coerced_req) = next_request.filter(|r| r.id == req.id) else {
                        continue;
                    };
                    next_request = deduped_requests.next();

                    // Always keep externally-dispatched requests visible, even if
                    // their name happens to overlap a registered frontend tool —
                    // they're observation-only and must not be removed from history.
                    let should_include = if coerced_req.was_executed_externally() {
                        true
                    } else if let Ok(tool_call) = &coerced_req.tool_call {
                        !self.is_frontend_tool(&tool_call.name).await
                    } else {
                        true
                    };

                    if should_include {
                        filtered_content.push(MessageContent::ToolRequest(coerced_req.clone()));
                    }
                }
                MessageContent::Thinking(_) | MessageContent::RedactedThinking(_)
                    if should_suppress_replayed_thinking => {}
                _ => {
                    if let Some(content) = user_visible_provider_content(content) {
                        filtered_content.push(content);
                    }
                }
            }
        }

        let mut filtered_message =
            Message::new(response.role.clone(), response.created, filtered_content);

        // Preserve the ID if it exists
        if let Some(id) = response.id.clone() {
            filtered_message = filtered_message.with_id(id);
        }

        // Categorize tool requests
        let mut frontend_requests = Vec::new();
        let mut other_requests = Vec::new();

        for request in tool_requests {
            // Skip externally-dispatched requests (e.g. claude-acp); the
            // provider already executed the tool. Stays in filtered_message.
            if request.was_executed_externally() {
                continue;
            }
            if let Ok(tool_call) = &request.tool_call {
                if self.is_frontend_tool(&tool_call.name).await {
                    frontend_requests.push(request);
                } else {
                    other_requests.push(request);
                }
            } else {
                // If there's an error in the tool call, add it to other_requests
                other_requests.push(request);
            }
        }

        (frontend_requests, other_requests, filtered_message)
    }

    /// `post_compaction_context_tokens` is `Some` when this usage came from a
    /// compaction call: the value (the retained summary size, not the billable
    /// output) becomes the session's new context baseline.
    pub(crate) async fn update_session_metrics(
        &self,
        session_id: &str,
        schedule_id: Option<String>,
        usage: &ProviderUsage,
        post_compaction_context_tokens: Option<i32>,
    ) -> Result<ProviderUsage> {
        let manager = self.config.session_manager.clone();
        let session = manager.get_session(session_id, false).await?;

        let (chunk_cost, cost_source) =
            self.resolve_chunk_cost(usage, session.provider_name.as_deref());

        let mut enriched = usage.clone();
        enriched.cost = chunk_cost;
        enriched.cost_source = cost_source;
        let ledger =
            MessageUsage::from_provider_usage(&enriched, post_compaction_context_tokens.is_some());

        let current_usage = match post_compaction_context_tokens {
            Some(retained) => Usage::new(Some(retained), None, Some(retained)),
            None => usage.usage,
        };

        manager
            .record_usage_metrics(
                session_id,
                schedule_id,
                current_usage,
                &usage.model,
                &ledger,
            )
            .await?;

        Ok(enriched)
    }

    fn resolve_chunk_cost(
        &self,
        usage: &ProviderUsage,
        provider_name: Option<&str>,
    ) -> (Option<f64>, Option<CostSource>) {
        if let Some(cost) = usage.cost {
            return (Some(cost), Some(CostSource::ProviderReported));
        }
        match provider_name
            .and_then(|pn| crate::providers::canonical::maybe_get_canonical_model(pn, &usage.model))
            .and_then(|canonical| canonical.cost.estimate_cost(&usage.usage))
        {
            Some(cost) => (Some(cost), Some(CostSource::Estimated)),
            None => (None, None),
        }
    }
}

fn user_visible_provider_content(content: &MessageContent) -> Option<MessageContent> {
    content.user_visible_content()
}

/// Check whether a tool should be callable by an app based on MCP Apps visibility metadata.
///
/// Per the MCP Apps spec (2026-01-26), if `_meta.ui.visibility` is present and does not
/// include `"app"`, the tool is model-only and must not be callable by app UIs.
/// If the field is absent, the tool defaults to visible to both model and app.
pub fn is_tool_visible_to_app(tool: &Tool) -> bool {
    let Some(meta) = &tool.meta else {
        return true;
    };
    let Some(ui) = meta.0.get("ui") else {
        return true;
    };
    let Some(visibility) = ui.get("visibility") else {
        return true;
    };
    let Some(arr) = visibility.as_array() else {
        return true;
    };
    arr.iter().any(|v| v.as_str() == Some("app"))
}

/// Check whether a tool should be visible to the model based on MCP Apps visibility metadata.
///
/// Per the MCP Apps spec (2026-01-26), tools may declare `_meta.ui.visibility` as an array
/// of `"model"` and/or `"app"`. If the field is absent, the tool defaults to visible to both.
/// If present and does not include `"model"`, the tool is app-only and must not be sent to the LLM.
pub fn is_tool_visible_to_model(tool: &Tool) -> bool {
    let Some(meta) = &tool.meta else {
        return true;
    };
    let Some(ui) = meta.0.get("ui") else {
        return true;
    };
    let Some(visibility) = ui.get("visibility") else {
        return true;
    };
    let Some(arr) = visibility.as_array() else {
        return true;
    };
    arr.iter().any(|v| v.as_str() == Some("model"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{AgentConfig, GoosePlatform};
    use crate::config::permission::PermissionLevel;
    use crate::config::{GooseMode, PermissionManager};
    use crate::conversation::message::{Message, SystemNotificationType};
    use crate::providers::base::Provider;
    use crate::session::{SessionManager, SessionType};
    use async_trait::async_trait;
    use goose_providers::conversation::token_usage::{ProviderStats, ProviderUsage, Usage};
    use goose_providers::model::ModelConfig;
    use rmcp::model::{AnnotateAble, RawTextContent, Role, ToolAnnotations};
    use rmcp::object;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    #[derive(Clone)]
    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        fn get_name(&self) -> &str {
            "mock"
        }

        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<MessageStream, ProviderError> {
            let message = Message::assistant().with_text("ok");
            let usage = ProviderUsage::new("mock".to_string(), Usage::default());
            Ok(stream_from_single_message(message, usage))
        }
    }

    #[derive(Clone)]
    struct CapturingProvider {
        messages: Arc<Mutex<Vec<Message>>>,
    }

    #[async_trait]
    impl Provider for CapturingProvider {
        fn get_name(&self) -> &str {
            "capturing"
        }

        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            messages: &[Message],
            _tools: &[Tool],
        ) -> Result<MessageStream, ProviderError> {
            *self.messages.lock().unwrap() = messages.to_vec();
            let message = Message::assistant().with_text("ok");
            let usage = ProviderUsage::new("capturing".to_string(), Usage::default());
            Ok(stream_from_single_message(message, usage))
        }
    }

    #[tokio::test]
    async fn provider_input_drops_rows_empty_after_agent_projection() {
        let user_only = RawTextContent {
            text: "user-only ACP output".to_string(),
            meta: None,
        }
        .no_annotation()
        .with_audience(vec![Role::User]);
        let messages = vec![
            Message::assistant().with_content(MessageContent::Text(user_only)),
            Message::user().with_text("current request"),
        ];
        let captured = Arc::new(Mutex::new(Vec::new()));
        let provider = Arc::new(CapturingProvider {
            messages: captured.clone(),
        });

        let _stream = crate::agents::Agent::stream_response_from_provider(
            provider,
            ModelConfig::new("test-model"),
            "test-session",
            "system",
            &messages,
            &[],
            &[],
        )
        .await
        .unwrap();

        let captured = captured.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].role, Role::User);
        assert_eq!(captured[0].as_concat_text(), "current request");
    }

    #[tokio::test]
    async fn provider_input_refixes_roles_after_agent_projection() {
        let user_only = RawTextContent {
            text: "hidden separator".to_string(),
            meta: None,
        }
        .no_annotation()
        .with_audience(vec![Role::User]);
        let messages = vec![
            Message::user().with_text("first request"),
            Message::assistant().with_content(MessageContent::Text(user_only)),
            Message::user().with_text("second request"),
        ];
        let captured = Arc::new(Mutex::new(Vec::new()));
        let provider = Arc::new(CapturingProvider {
            messages: captured.clone(),
        });

        let _stream = crate::agents::Agent::stream_response_from_provider(
            provider,
            ModelConfig::new("test-model"),
            "test-session",
            "system",
            &messages,
            &[],
            &[],
        )
        .await
        .unwrap();

        let captured = captured.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].role, Role::User);
        assert_eq!(
            captured[0].as_concat_text(),
            "first request\nsecond request"
        );
        assert!(!captured[0].as_concat_text().contains("hidden separator"));
    }

    #[tokio::test]
    async fn provider_input_refixes_tool_result_emptied_by_agent_projection() {
        let user_only_result =
            rmcp::model::Content::text("hidden result").with_audience(vec![Role::User]);
        let messages = vec![
            Message::user().with_text("run the tool"),
            Message::assistant().with_tool_request(
                "tool-1",
                Ok(rmcp::model::CallToolRequestParams::new("test_tool")),
            ),
            Message::user().with_tool_response(
                "tool-1",
                Ok(rmcp::model::CallToolResult::success(vec![user_only_result])),
            ),
        ];
        let captured = Arc::new(Mutex::new(Vec::new()));
        let provider = Arc::new(CapturingProvider {
            messages: captured.clone(),
        });

        let _stream = crate::agents::Agent::stream_response_from_provider(
            provider,
            ModelConfig::new("test-model"),
            "test-session",
            "system",
            &messages,
            &[],
            &[],
        )
        .await
        .unwrap();

        let captured = captured.lock().unwrap();
        let tool_response = captured
            .iter()
            .flat_map(|message| &message.content)
            .find_map(|content| match content {
                MessageContent::ToolResponse(response) => Some(response),
                _ => None,
            })
            .expect("projected tool response should remain paired");
        let result = tool_response
            .tool_result
            .as_ref()
            .expect("tool response should remain successful");
        assert_eq!(result.content.len(), 1);
        assert_eq!(
            result.content[0]
                .as_text()
                .expect("placeholder should be text")
                .text,
            "(empty result)"
        );
    }

    #[tokio::test]
    async fn prepare_tools_returns_sorted_tools_including_frontend() -> anyhow::Result<()> {
        let data_dir = tempfile::tempdir()?;
        let data_path = data_dir.path().to_path_buf();
        let session_manager = std::sync::Arc::new(SessionManager::new(data_path.clone()));
        let agent = Agent::with_config(AgentConfig::new(
            std::sync::Arc::clone(&session_manager),
            std::sync::Arc::new(PermissionManager::new(data_path)),
            None,
            GooseMode::default(),
            false,
            GoosePlatform::GooseCli,
        ));

        let session = session_manager
            .create_session(
                std::env::current_dir().unwrap(),
                "test-prepare-tools".to_string(),
                SessionType::Hidden,
                GooseMode::default(),
            )
            .await?;

        let model_config = ModelConfig::new("test-model");
        let provider = std::sync::Arc::new(MockProvider);
        agent
            .update_provider(provider, model_config, &session.id)
            .await?;

        // Add unsorted frontend tools
        let frontend_tools = vec![
            Tool::new(
                "frontend__z_tool".to_string(),
                "Z tool".to_string(),
                object!({ "type": "object", "properties": { } }),
            ),
            Tool::new(
                "frontend__a_tool".to_string(),
                "A tool".to_string(),
                object!({ "type": "object", "properties": { } }),
            ),
        ];

        agent
            .add_extension(
                crate::agents::extension::ExtensionConfig::Frontend {
                    name: "frontend".to_string(),
                    description: "desc".to_string(),
                    tools: frontend_tools,
                    instructions: None,
                    bundled: None,
                    available_tools: vec![],
                },
                &session.id,
            )
            .await
            .unwrap();

        let (tools, _toolshim_tools, _system_prompt, _model_config) = agent
            .prepare_tools_and_prompt(&session.id, session.working_dir.as_path())
            .await?;

        let names: Vec<String> = tools.iter().map(|t| t.name.clone().into_owned()).collect();
        assert!(names.iter().any(|n| n == "frontend__a_tool"));
        assert!(names.iter().any(|n| n == "frontend__z_tool"));

        // Verify the names are sorted ascending
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);

        Ok(())
    }

    #[tokio::test]
    async fn prepare_toolshim_tools_applies_writable_annotations() -> anyhow::Result<()> {
        let data_dir = tempfile::tempdir()?;
        let data_path = data_dir.path().to_path_buf();
        let session_manager = Arc::new(SessionManager::new(data_path.clone()));
        let permission_manager = Arc::new(PermissionManager::new(data_path));
        permission_manager
            .update_smart_approve_permission("frontend__write_tool", PermissionLevel::AlwaysAllow);
        let agent = Agent::with_config(AgentConfig::new(
            Arc::clone(&session_manager),
            Arc::clone(&permission_manager),
            None,
            GooseMode::SmartApprove,
            false,
            GoosePlatform::GooseCli,
        ));
        let session = session_manager
            .create_session(
                std::env::current_dir()?,
                "test-toolshim-annotations".to_string(),
                SessionType::Hidden,
                GooseMode::SmartApprove,
            )
            .await?;
        let model_config = ModelConfig::new("test-model").with_toolshim(true);
        agent
            .update_provider(Arc::new(MockProvider), model_config, &session.id)
            .await?;
        agent
            .add_extension(
                crate::agents::extension::ExtensionConfig::Frontend {
                    name: "frontend".to_string(),
                    description: "desc".to_string(),
                    tools: vec![Tool::new(
                        "frontend__write_tool",
                        "Write tool",
                        object!({ "type": "object", "properties": { } }),
                    )
                    .annotate(ToolAnnotations::new().read_only(false))],
                    instructions: None,
                    bundled: None,
                    available_tools: vec![],
                },
                &session.id,
            )
            .await?;

        let (tools, toolshim_tools, _, _) = agent
            .prepare_tools_and_prompt(&session.id, session.working_dir.as_path())
            .await?;

        assert!(tools.is_empty());
        assert!(toolshim_tools
            .iter()
            .any(|tool| tool.name == "frontend__write_tool"));
        assert_eq!(
            permission_manager.get_smart_approve_permission("frontend__write_tool"),
            Some(PermissionLevel::AskBefore)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_stream_error_propagation() {
        use futures::StreamExt;

        type StreamItem = Result<(Option<Message>, Option<ProviderUsage>), ProviderError>;
        let stream = futures::stream::iter(vec![
            Ok((Some(Message::assistant().with_text("chunk1")), None)),
            Ok((Some(Message::assistant().with_text("chunk2")), None)),
            Err(ProviderError::RequestFailed(
                "simulated stream error".to_string(),
            )),
        ] as Vec<StreamItem>);

        let mut pinned = Box::pin(stream);
        let mut results = Vec::new();
        let mut error_seen = false;

        while let Some(result) = pinned.next().await {
            match result {
                Ok((message, _usage)) => {
                    if let Some(msg) = message {
                        results.push(msg.as_concat_text());
                    }
                }
                Err(_e) => {
                    error_seen = true;
                    break;
                }
            }
        }

        assert_eq!(results.len(), 2);
        assert_eq!(results[0], "chunk1");
        assert_eq!(results[1], "chunk2");
        assert!(
            error_seen,
            "Error should have been propagated, not silently ignored"
        );
    }

    #[tokio::test]
    async fn categorize_tool_requests_keeps_thinking_when_not_previously_streamed() {
        let agent = crate::agents::Agent::new();
        let response = Message::assistant()
            .with_thinking("final-only reasoning", "")
            .with_tool_request(
                "tool-1",
                Ok(rmcp::model::CallToolRequestParams::new("test_tool")),
            );

        let (_frontend_requests, other_requests, filtered_message) =
            agent.categorize_tool_requests(&response, &[], false).await;

        assert_eq!(other_requests.len(), 1);
        assert_eq!(filtered_message.content.len(), 2);
        assert!(matches!(
            filtered_message.content[0],
            MessageContent::Thinking(_)
        ));
        assert!(matches!(
            filtered_message.content[1],
            MessageContent::ToolRequest(_)
        ));
    }

    #[tokio::test]
    async fn categorize_tool_requests_drops_replayed_thinking_after_streaming() {
        let agent = crate::agents::Agent::new();
        let response = Message::assistant()
            .with_thinking("replayed reasoning", "")
            .with_tool_request(
                "tool-1",
                Ok(rmcp::model::CallToolRequestParams::new("test_tool")),
            );

        let (_frontend_requests, other_requests, filtered_message) =
            agent.categorize_tool_requests(&response, &[], true).await;

        assert_eq!(other_requests.len(), 1);
        assert_eq!(filtered_message.content.len(), 1);
        assert!(matches!(
            filtered_message.content[0],
            MessageContent::ToolRequest(_)
        ));
    }

    #[tokio::test]
    async fn categorize_tool_requests_excludes_assistant_only_text_from_user_events() {
        let agent = crate::agents::Agent::new();
        let assistant_only = RawTextContent {
            text: "assistant-only".to_string(),
            meta: None,
        }
        .no_annotation()
        .with_audience(vec![Role::Assistant]);
        let response = Message::assistant()
            .with_content(MessageContent::Text(assistant_only))
            .with_text("user-visible")
            .with_thinking("visible reasoning", "");

        let (_frontend_requests, _other_requests, filtered_message) =
            agent.categorize_tool_requests(&response, &[], false).await;

        assert_eq!(response.as_concat_text(), "assistant-only\nuser-visible");
        assert_eq!(filtered_message.as_concat_text(), "user-visible");
        assert!(filtered_message
            .content
            .iter()
            .any(|content| matches!(content, MessageContent::Thinking(_))));
    }

    #[tokio::test]
    async fn categorize_tool_requests_skips_externally_dispatched_and_preserves_marker() {
        // External requests must (1) survive coercion with goose.external_dispatch
        // intact, (2) be excluded from both dispatch buckets, (3) stay in
        // filtered_message.
        use crate::conversation::message::TOOL_META_EXTERNAL_DISPATCH_KEY;

        let agent = crate::agents::Agent::new();

        let registry_tool = Tool::new("test_tool", "a test tool", object!({ "type": "object" }))
            .with_meta(rmcp::model::Meta(
                serde_json::json!({ "ui": { "visibility": ["model"] } })
                    .as_object()
                    .unwrap()
                    .clone(),
            ));

        let response = Message::assistant().with_tool_request_with_metadata(
            "tool-1",
            Ok(rmcp::model::CallToolRequestParams::new("test_tool")),
            None,
            Some(serde_json::json!({ TOOL_META_EXTERNAL_DISPATCH_KEY: true })),
        );

        let (frontend_requests, other_requests, filtered_message) = agent
            .categorize_tool_requests(&response, &[registry_tool], false)
            .await;

        assert!(
            frontend_requests.is_empty(),
            "external request leaked into frontend_requests: {frontend_requests:?}"
        );
        assert!(
            other_requests.is_empty(),
            "external request leaked into other_requests: {other_requests:?}"
        );
        assert_eq!(filtered_message.content.len(), 1);
        let tool_req = match &filtered_message.content[0] {
            MessageContent::ToolRequest(req) => req,
            other => panic!("expected ToolRequest, got {other:?}"),
        };
        assert!(
            tool_req.was_executed_externally(),
            "goose.external_dispatch marker was clobbered by coercion; merged tool_meta = {:?}",
            tool_req.tool_meta
        );
        let merged = tool_req
            .tool_meta
            .as_ref()
            .and_then(|v| v.as_object())
            .expect("tool_meta should be an object after merge");
        assert!(
            merged.contains_key("ui"),
            "registry tool meta keys were dropped; merged tool_meta = {merged:?}"
        );
    }

    #[tokio::test]
    async fn categorize_tool_requests_dedups_duplicate_ids_in_provider_order() {
        // A malformed provider repeats id "dup". The first occurrence wins, the
        // later duplicate is dropped from both the dispatch bucket and the
        // filtered (history) message, and unique ids are kept.
        let agent = crate::agents::Agent::new();

        let response = Message::assistant()
            .with_tool_request(
                "dup",
                Ok(rmcp::model::CallToolRequestParams::new("first_tool")),
            )
            .with_tool_request(
                "dup",
                Ok(rmcp::model::CallToolRequestParams::new("second_tool")),
            )
            .with_tool_request(
                "unique",
                Ok(rmcp::model::CallToolRequestParams::new("third_tool")),
            );

        let (_frontend_requests, other_requests, filtered_message) =
            agent.categorize_tool_requests(&response, &[], false).await;

        let kept: Vec<(&str, &str)> = other_requests
            .iter()
            .map(|r| (r.id.as_str(), r.tool_call.as_ref().unwrap().name.as_ref()))
            .collect();
        assert_eq!(kept, vec![("dup", "first_tool"), ("unique", "third_tool")]);

        let filtered_ids: Vec<&str> = filtered_message
            .content
            .iter()
            .filter_map(|c| match c {
                MessageContent::ToolRequest(req) => Some(req.id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(filtered_ids, vec!["dup", "unique"]);
    }

    fn make_tool_with_meta(meta_json: Option<serde_json::Value>) -> Tool {
        let mut tool = Tool::new("test_tool", "a test tool", object!({ "type": "object" }));
        if let Some(v) = meta_json {
            let obj = v.as_object().unwrap().clone();
            tool = tool.with_meta(rmcp::model::Meta(obj));
        }
        tool
    }

    #[test]
    fn test_tool_visible_when_no_meta() {
        let tool = make_tool_with_meta(None);
        assert!(is_tool_visible_to_model(&tool));
    }

    #[test]
    fn test_tool_visible_when_meta_has_no_ui() {
        let tool = make_tool_with_meta(Some(serde_json::json!({"other": "stuff"})));
        assert!(is_tool_visible_to_model(&tool));
    }

    #[test]
    fn test_tool_visible_when_ui_has_no_visibility() {
        let tool = make_tool_with_meta(Some(
            serde_json::json!({"ui": {"resourceUri": "ui://foo/bar"}}),
        ));
        assert!(is_tool_visible_to_model(&tool));
    }

    #[test]
    fn test_tool_visible_when_visibility_includes_model() {
        let tool = make_tool_with_meta(Some(
            serde_json::json!({"ui": {"visibility": ["model", "app"]}}),
        ));
        assert!(is_tool_visible_to_model(&tool));
    }

    #[test]
    fn test_tool_visible_when_visibility_is_model_only() {
        let tool = make_tool_with_meta(Some(serde_json::json!({"ui": {"visibility": ["model"]}})));
        assert!(is_tool_visible_to_model(&tool));
    }

    #[test]
    fn test_tool_hidden_when_visibility_is_app_only() {
        let tool = make_tool_with_meta(Some(serde_json::json!({"ui": {"visibility": ["app"]}})));
        assert!(!is_tool_visible_to_model(&tool));
    }

    #[test]
    fn test_tool_hidden_when_visibility_is_empty() {
        let tool = make_tool_with_meta(Some(serde_json::json!({"ui": {"visibility": []}})));
        assert!(!is_tool_visible_to_model(&tool));
    }

    #[test]
    fn test_tool_visible_when_visibility_is_not_array() {
        let tool = make_tool_with_meta(Some(serde_json::json!({"ui": {"visibility": "model"}})));
        assert!(is_tool_visible_to_model(&tool));
    }

    #[test]
    fn test_app_visible_when_no_meta() {
        let tool = make_tool_with_meta(None);
        assert!(is_tool_visible_to_app(&tool));
    }

    #[test]
    fn test_app_visible_when_visibility_includes_app() {
        let tool = make_tool_with_meta(Some(
            serde_json::json!({"ui": {"visibility": ["model", "app"]}}),
        ));
        assert!(is_tool_visible_to_app(&tool));
    }

    #[test]
    fn test_app_visible_when_visibility_is_app_only() {
        let tool = make_tool_with_meta(Some(serde_json::json!({"ui": {"visibility": ["app"]}})));
        assert!(is_tool_visible_to_app(&tool));
    }

    #[test]
    fn test_app_hidden_when_visibility_is_model_only() {
        let tool = make_tool_with_meta(Some(serde_json::json!({"ui": {"visibility": ["model"]}})));
        assert!(!is_tool_visible_to_app(&tool));
    }

    #[test]
    fn test_app_hidden_when_visibility_is_empty() {
        let tool = make_tool_with_meta(Some(serde_json::json!({"ui": {"visibility": []}})));
        assert!(!is_tool_visible_to_app(&tool));
    }

    fn usage_with_stats(stats: Option<ProviderStats>) -> ProviderUsage {
        let mut usage = ProviderUsage::new("mock".to_string(), Usage::default());
        usage.stats = stats;
        usage
    }

    #[test]
    fn message_has_timing_content_ignores_system_notification_only_messages() {
        let message = Message::assistant().with_system_notification(
            SystemNotificationType::ProgressMessage,
            "Loading local model test-model...",
        );

        assert!(!message_has_timing_content(&message));
    }

    #[test]
    fn message_has_timing_content_counts_user_visible_messages() {
        let text_message = Message::assistant().with_text("hello");
        let mixed_message = Message::assistant()
            .with_system_notification(SystemNotificationType::ProgressMessage, "Loading...")
            .with_text("ready");

        assert!(message_has_timing_content(&text_message));
        assert!(message_has_timing_content(&mixed_message));
    }

    #[test]
    fn fill_stream_timing_fills_both_fields_when_stats_absent() {
        let request_started = Instant::now() - Duration::from_millis(100);
        let first_content_at = Some(request_started + Duration::from_millis(40));
        let mut usage = usage_with_stats(None);

        fill_stream_timing(&mut usage, request_started, first_content_at);

        let stats = usage.stats.expect("stats must be created when absent");
        assert_eq!(stats.time_to_first_token_ms, Some(40));
        let elapsed = stats.elapsed_ms.expect("elapsed_ms must be filled");
        assert!(
            elapsed >= 100,
            "elapsed_ms ({elapsed}) must cover the full request duration"
        );
        assert!(stats.time_to_first_token_ms.unwrap() <= elapsed);
    }

    #[test]
    fn fill_stream_timing_preserves_provider_reported_values() {
        let request_started = Instant::now() - Duration::from_millis(100);
        let first_content_at = Some(request_started + Duration::from_millis(25));
        let mut usage = usage_with_stats(Some(ProviderStats {
            elapsed_ms: Some(7),
            time_to_first_token_ms: Some(3),
            output_tokens: Some(42),
            ..Default::default()
        }));

        fill_stream_timing(&mut usage, request_started, first_content_at);

        let stats = usage.stats.expect("stats must survive");
        assert_eq!(
            stats.elapsed_ms,
            Some(7),
            "provider-reported elapsed_ms (e.g. MLX) must not be overwritten"
        );
        assert_eq!(
            stats.time_to_first_token_ms,
            Some(3),
            "provider-reported TTFT must not be overwritten"
        );
        assert_eq!(
            stats.output_tokens,
            Some(42),
            "unrelated provider stats must survive the fill"
        );
    }

    #[test]
    fn fill_stream_timing_without_first_content_leaves_ttft_unset() {
        let request_started = Instant::now() - Duration::from_millis(100);
        let mut usage = usage_with_stats(None);

        fill_stream_timing(&mut usage, request_started, None);

        let stats = usage.stats.expect("stats must be created when absent");
        assert_eq!(
            stats.time_to_first_token_ms, None,
            "no content chunk observed means no TTFT"
        );
        assert!(stats.elapsed_ms.expect("elapsed_ms must be filled") >= 100);
    }
}
