//! The core agent loop, extracted as a free function.
//!
//! This module contains the main inference–tool-execution loop, parameterized
//! by a `LoopDriver` trait that provides tool dispatch, permissions, and other
//! Agent-specific capabilities. The `AgentHook` trait handles cross-cutting
//! concerns (compaction, prompt injection, etc.).
//!
//! The separation makes it possible to:
//! - Add new hooks without touching the loop
//! - Test the loop with mock drivers
//! - Eventually replace the driver with a simpler interface

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt, TryStreamExt};
use rmcp::model::{CallToolResult, Content, Tool};
use tracing::{error, warn};
use tracing_futures::Instrument;
use uuid::Uuid;

use super::agent::{frontend_tool_stream, tool_stream, AgentEvent, ToolStream, ToolStreamItem};
use super::hooks::{AgentHook, ErrorRecovery, ExitAction, ExitReason, LoopContext, LoopEvent};
use super::reply_parts::update_session_metrics_standalone;
use super::tool_execution::{ToolCallResult, CHAT_MODE_TOOL_SKIPPED_RESPONSE};
use crate::agents::platform_extensions::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;
use crate::agents::types::SessionConfig;
use crate::config::{Config, GooseMode};
use crate::conversation::message::{
    Message, MessageContent, ProviderMetadata, SystemNotificationType,
};
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::session::{Session, SessionManager};
use crate::utils::is_token_cancelled;
use rmcp::model::ErrorData;
use tokio_util::sync::CancellationToken;

const DEFAULT_MAX_TURNS: u32 = 1000;

/// Trait abstracting the Agent-specific capabilities the loop needs.
///
/// This is the seam between the generic loop and Agent's concrete implementation.
/// Each method corresponds to something the loop delegates to the Agent today.
/// Over time, more of these can be converted into hooks.
#[async_trait]
pub trait LoopDriver: Send + Sync {
    /// Get the LLM provider.
    fn provider(&self) -> Arc<dyn Provider>;

    /// Get the session manager.
    fn session_manager(&self) -> Arc<SessionManager>;

    /// Check if a tool is a frontend tool.
    async fn is_frontend_tool(&self, name: &str) -> bool;

    /// Dispatch a tool call and return the result stream.
    async fn dispatch_tool_call(
        &self,
        tool_call: rmcp::model::CallToolRequestParams,
        request_id: String,
        cancel_token: Option<CancellationToken>,
        session: &Session,
    ) -> (String, Result<ToolCallResult, ErrorData>);

    /// Inspect, permission-check, and dispatch backend tool requests.
    ///
    /// This consolidates the full tool gate flow:
    /// 1. Runs tool inspectors (adversary detection, permission checks, repetition)
    /// 2. Partitions into approved / needs_approval / denied
    /// 3. Dispatches approved tools → adds to `tool_futures`
    /// 4. Marks denied tools with DECLINED_RESPONSE in `request_to_response_map`
    /// 5. Streams action-required messages for tools needing user approval;
    ///    dispatches on confirmation → adds to `tool_futures`
    ///
    /// Returns a stream of action-required messages to yield to the UI.
    fn gate_and_dispatch_tools<'a>(
        &'a self,
        session_id: &'a str,
        requests: &'a [crate::conversation::message::ToolRequest],
        messages: &'a [Message],
        goose_mode: GooseMode,
        tool_futures: &'a mut Vec<(String, ToolStream)>,
        request_to_response_map: &'a mut HashMap<String, Message>,
        cancel_token: Option<CancellationToken>,
        session: &'a Session,
    ) -> BoxStream<'a, Result<Message>>;

    /// Drain pending elicitation messages for a session.
    async fn drain_elicitation_messages(&self, session_id: &str) -> Vec<Message>;

    /// Save extension state after runtime changes.
    async fn save_extension_state(&self, session_config: &SessionConfig) -> Result<()>;

    /// Refresh tools and system prompt (e.g. after extension install or hint reload).
    async fn prepare_tools_and_prompt(
        &self,
        session_id: &str,
        working_dir: &std::path::Path,
    ) -> Result<(Vec<Tool>, Vec<Tool>, String)>;

    /// Inject MOIM (model-oriented information messages) into the conversation.
    async fn inject_moim(
        &self,
        session_id: &str,
        conversation: Conversation,
        working_dir: &std::path::Path,
    ) -> Conversation;

    /// Stream a response from the provider.
    async fn stream_response(
        &self,
        session_id: &str,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
        toolshim_tools: &[Tool],
    ) -> Result<crate::providers::base::MessageStream, ProviderError>;

    /// Load subdirectory hints, returning true if new hints were found.
    async fn load_subdirectory_hints(&self, working_dir: &std::path::Path) -> bool;
}

/// Configuration for the agent loop.
pub struct LoopConfig {
    pub session_config: SessionConfig,
    pub session: Session,
    pub cancel_token: Option<CancellationToken>,
    /// Initial conversation state.
    pub conversation: Conversation,
    /// Initial tools list.
    pub tools: Vec<Tool>,
    /// Toolshim tools (provider-specific).
    pub toolshim_tools: Vec<Tool>,
    /// System prompt.
    pub system_prompt: String,
    /// Current goose mode.
    pub goose_mode: GooseMode,
    /// Tool call cutoff for summarization.
    pub tool_call_cut_off: usize,
}

/// Run the main agent loop.
///
/// This is the core inference–tool-execution loop, parameterized by:
/// - `driver`: provides tool dispatch, permissions, and Agent-specific capabilities
/// - `hooks`: composable lifecycle hooks (compaction, MOIM injection, etc.)
/// - `config`: loop configuration (conversation, tools, session, etc.)
///
/// Returns a stream of `AgentEvent`s.
pub fn run_agent_loop<'a>(
    driver: &'a (dyn LoopDriver + 'a),
    hooks: Vec<Box<dyn AgentHook>>,
    config: LoopConfig,
) -> BoxStream<'a, Result<AgentEvent>> {
    let LoopConfig {
        session_config,
        session,
        cancel_token,
        conversation,
        mut tools,
        mut toolshim_tools,
        mut system_prompt,
        goose_mode,
        tool_call_cut_off,
    } = config;

    let session_manager = driver.session_manager();
    let mut conversation = conversation;
    let working_dir = session.working_dir.clone();

    let pre_turn_tool_count = conversation
        .messages()
        .iter()
        .flat_map(|m| m.content.iter())
        .filter(|c| matches!(c, MessageContent::ToolRequest(_)))
        .count();

    let reply_stream_span = tracing::info_span!(
        target: "goose::agents::agent",
        "reply_stream",
        session.id = %session_config.id
    );

    let inner = Box::pin(
        async_stream::try_stream! {
            let mut turns_taken = 0u32;
            let max_turns = session_config.max_turns.unwrap_or_else(|| {
                Config::global()
                    .get_param::<u32>("GOOSE_MAX_TURNS")
                    .unwrap_or(DEFAULT_MAX_TURNS)
            });
            let mut last_assistant_text = String::new();

            loop {
                if is_token_cancelled(&cancel_token) {
                    break;
                }

                turns_taken += 1;
                if turns_taken > max_turns {
                    yield AgentEvent::Message(
                        Message::assistant().with_text(
                            "I've reached the maximum number of actions I can do without user input. Would you like me to continue?"
                        )
                    );
                    break;
                }

                // --- pre_inference hooks ---
                // Runs compaction, final output check, etc.
                {
                    let (mut hook_ctx, mut hook_event_rx) = LoopContext::new(
                        conversation, system_prompt, tools, toolshim_tools,
                        driver.provider(), session_config.id.clone(),
                        session_config.schedule_id.clone(), session_manager.clone(),
                    );

                    for hook in &hooks {
                        hook.pre_inference(&mut hook_ctx).await?;
                        if hook_ctx.should_exit { break; }
                    }

                    let should_exit = hook_ctx.should_exit;
                    conversation = hook_ctx.conversation;
                    system_prompt = hook_ctx.system_prompt;
                    tools = hook_ctx.tools;
                    toolshim_tools = hook_ctx.toolshim_tools;

                    while let Ok(event) = hook_event_rx.try_recv() {
                        match event {
                            LoopEvent::Message(msg) => yield AgentEvent::Message(msg),
                            LoopEvent::HistoryReplaced(conv) => yield AgentEvent::HistoryReplaced(conv),
                        }
                    }

                    if should_exit { break; }
                }

                let conversation_with_moim = driver.inject_moim(
                    &session_config.id,
                    conversation.clone(),
                    &working_dir,
                ).await;

                let mut stream = driver.stream_response(
                    &session_config.id,
                    &system_prompt,
                    conversation_with_moim.messages(),
                    &tools,
                    &toolshim_tools,
                ).await?;

                let current_turn_tool_count = conversation.messages().iter()
                    .flat_map(|m| m.content.iter())
                    .filter(|c| matches!(c, MessageContent::ToolRequest(_)))
                    .count()
                    .saturating_sub(pre_turn_tool_count);

                let tool_pair_summarization_task = crate::context_mgmt::maybe_summarize_tool_pairs(
                    driver.provider(),
                    session_config.id.clone(),
                    conversation.clone(),
                    tool_call_cut_off,
                    current_turn_tool_count,
                );

                let mut no_tools_called = true;
                let mut messages_to_add = Conversation::default();
                let mut tools_updated = false;
                let mut did_recovery_compact_this_iteration = false;
                let mut exit_chat = false;

                while let Some(next) = stream.next().await {
                    if is_token_cancelled(&cancel_token) || exit_chat {
                        break;
                    }

                    match next {
                        Ok((response, usage)) => {
                            // --- post_inference hooks ---
                            // Run after successful LLM response, before tool execution.
                            // Currently used by CompactionHook to reset recovery counters.
                            if let Some(ref resp) = response {
                                let (mut hook_ctx, mut hook_event_rx) = LoopContext::new(
                                    conversation.clone(), system_prompt.clone(),
                                    tools.clone(), toolshim_tools.clone(),
                                    driver.provider(), session_config.id.clone(),
                                    session_config.schedule_id.clone(), session_manager.clone(),
                                );
                                for hook in &hooks {
                                    hook.post_inference(resp, &mut hook_ctx).await?;
                                }
                                while let Ok(event) = hook_event_rx.try_recv() {
                                    match event {
                                        LoopEvent::Message(msg) => yield AgentEvent::Message(msg),
                                        LoopEvent::HistoryReplaced(conv) => yield AgentEvent::HistoryReplaced(conv),
                                    }
                                }
                            }

                            if let Some(ref usage) = usage {
                                update_session_metrics_standalone(
                                    &session_manager,
                                    &session_config.id,
                                    session_config.schedule_id.clone(),
                                    usage,
                                    false,
                                ).await?;
                            }

                            if let Some(response) = response {
                                let (all_requests, coerced_response) =
                                    crate::agents::Agent::prepare_tool_requests(&response, &tools);

                                yield AgentEvent::Message(coerced_response.clone());
                                tokio::task::yield_now().await;

                                if all_requests.is_empty() {
                                    let text = coerced_response.as_concat_text();
                                    if !text.is_empty() {
                                        last_assistant_text = text;
                                    }
                                    messages_to_add.push(response);
                                    continue;
                                }

                                // --- Tool execution ---
                                let (mut response_map, metadata_map) = prepare_tool_maps(&all_requests);
                                let (frontend_requests, backend_requests) =
                                    partition_requests(driver, &all_requests).await;

                                let mut tool_futures =
                                    dispatch_frontend_tools(driver, &frontend_requests, cancel_token.clone(), &session).await;

                                let extension_install_ids = find_extension_install_ids(&backend_requests);

                                if goose_mode == GooseMode::Chat {
                                    skip_backend_tools_chat_mode(&backend_requests, &mut response_map);
                                } else {
                                    let mut approval_stream = driver.gate_and_dispatch_tools(
                                        &session_config.id, &backend_requests,
                                        conversation.messages(), goose_mode,
                                        &mut tool_futures, &mut response_map,
                                        cancel_token.clone(), &session,
                                    );
                                    while let Some(msg) = approval_stream.try_next().await? {
                                        yield AgentEvent::Message(msg);
                                    }
                                }

                                // --- Drain tool streams ---
                                {
                                    let with_id = tool_futures.into_iter()
                                        .map(|(id, s)| s.map(move |item| (id.clone(), item)))
                                        .collect::<Vec<_>>();
                                    let mut combined = stream::select_all(with_id);
                                    let mut all_install_successful = true;

                                    loop {
                                        if is_token_cancelled(&cancel_token) { break; }
                                        for msg in driver.drain_elicitation_messages(&session_config.id).await {
                                            yield AgentEvent::Message(msg);
                                        }
                                        tokio::select! {
                                            biased;
                                            tool_item = combined.next() => {
                                                match tool_item {
                                                    Some((request_id, item)) => {
                                                        for event in handle_tool_stream_item(
                                                            request_id, item,
                                                            &mut response_map, &metadata_map,
                                                            &extension_install_ids, &mut all_install_successful,
                                                        ) {
                                                            yield event;
                                                        }
                                                    }
                                                    None => break,
                                                }
                                            }
                                            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
                                        }
                                    }

                                    for msg in driver.drain_elicitation_messages(&session_config.id).await {
                                        yield AgentEvent::Message(msg);
                                    }
                                    if all_install_successful && !extension_install_ids.is_empty() {
                                        if let Err(e) = driver.save_extension_state(&session_config).await {
                                            warn!("Failed to save extension state after runtime changes: {}", e);
                                        }
                                        tools_updated = true;
                                    }
                                }

                                // --- Build conversation messages from results ---
                                let pair_result = build_tool_pair_messages(
                                    &response, &all_requests, &mut response_map,
                                );
                                for msg in &pair_result.messages {
                                    messages_to_add.push(msg.clone());
                                }
                                for event in pair_result.events {
                                    yield event;
                                }
                                if pair_result.exit_chat {
                                    exit_chat = true;
                                    break;
                                }

                                no_tools_called = false;
                            }
                        }
                        Err(ref provider_err) => {
                            #[cfg(feature = "telemetry")]
                            crate::posthog::emit_error(provider_err.telemetry_type(), &provider_err.to_string());

                            // --- on_error hooks ---
                            // Give hooks a chance to handle the error (e.g. compaction on ContextLengthExceeded).
                            let (mut hook_ctx, mut hook_event_rx) = LoopContext::new(
                                conversation.clone(), system_prompt.clone(),
                                tools.clone(), toolshim_tools.clone(),
                                driver.provider(), session_config.id.clone(),
                                session_config.schedule_id.clone(), session_manager.clone(),
                            );

                            let mut recovery = ErrorRecovery::Propagate;
                            for hook in &hooks {
                                let result = hook.on_error(provider_err, &mut hook_ctx).await?;
                                // Drain events emitted by this hook
                                while let Ok(event) = hook_event_rx.try_recv() {
                                    match event {
                                        LoopEvent::Message(msg) => yield AgentEvent::Message(msg),
                                        LoopEvent::HistoryReplaced(conv) => yield AgentEvent::HistoryReplaced(conv),
                                    }
                                }
                                if matches!(result, ErrorRecovery::Retry) {
                                    recovery = ErrorRecovery::Retry;
                                    break;
                                }
                            }

                            match recovery {
                                ErrorRecovery::Retry => {
                                    conversation = hook_ctx.conversation;
                                    did_recovery_compact_this_iteration = true;
                                    break;
                                }
                                ErrorRecovery::Propagate => {
                                    // No hook handled the error — display to user
                                    error!("Error: {}", provider_err);
                                    match provider_err {
                                        ProviderError::CreditsExhausted { top_up_url, .. } => {
                                            let user_msg = if top_up_url.is_some() {
                                                "Please add credits to your account, then resend your message to continue.".to_string()
                                            } else {
                                                "Please check your account with your provider to add more credits, then resend your message to continue.".to_string()
                                            };
                                            let notification_data = serde_json::json!({
                                                "top_up_url": top_up_url,
                                            });
                                            yield AgentEvent::Message(
                                                Message::assistant().with_system_notification_with_data(
                                                    SystemNotificationType::CreditsExhausted,
                                                    user_msg,
                                                    notification_data,
                                                )
                                            );
                                        }
                                        ProviderError::NetworkError(_) => {
                                            yield AgentEvent::Message(
                                                Message::assistant().with_text(
                                                    format!("{provider_err}\n\nPlease resend your message to try again.")
                                                )
                                            );
                                        }
                                        _ => {
                                            yield AgentEvent::Message(
                                                Message::assistant().with_text(
                                                    format!("Ran into this error: {provider_err}.\n\nPlease retry if you think this is a transient or recoverable error.")
                                                )
                                            );
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }

                if tools_updated {
                    (tools, toolshim_tools, system_prompt) =
                        driver.prepare_tools_and_prompt(&session_config.id, &session.working_dir).await?;
                }

                {
                    let has_new_hints = driver.load_subdirectory_hints(&working_dir).await;
                    if has_new_hints && !tools_updated {
                        (tools, toolshim_tools, system_prompt) =
                            driver.prepare_tools_and_prompt(&session_config.id, &session.working_dir).await?;
                    }
                }

                // --- on_loop_exit hooks ---
                // When no tools were called and we didn't just recover from compaction,
                // run on_loop_exit hooks (final output check, retry logic, etc.).
                if no_tools_called && !did_recovery_compact_this_iteration {
                    let (mut hook_ctx, mut hook_event_rx) = LoopContext::new(
                        conversation, system_prompt, tools, toolshim_tools,
                        driver.provider(), session_config.id.clone(),
                        session_config.schedule_id.clone(), session_manager.clone(),
                    );

                    let mut action = ExitAction::Defer;
                    for hook in &hooks {
                        action = hook.on_loop_exit(&ExitReason::NoToolCalls, &mut hook_ctx).await?;
                        // Drain events — persist messages, yield to UI
                        let mut history_replaced = false;
                        while let Ok(event) = hook_event_rx.try_recv() {
                            match event {
                                LoopEvent::Message(msg) => {
                                    messages_to_add.push(msg.clone());
                                    yield AgentEvent::Message(msg);
                                }
                                LoopEvent::HistoryReplaced(conv) => {
                                    history_replaced = true;
                                    yield AgentEvent::HistoryReplaced(conv);
                                }
                            }
                        }
                        if history_replaced {
                            // Conversation was replaced (e.g. retry) — clear accumulated messages
                            messages_to_add = Conversation::default();
                        }
                        match action {
                            ExitAction::Continue | ExitAction::Exit => break,
                            ExitAction::Defer => continue,
                        }
                    }

                    // Move state back from hook context
                    conversation = hook_ctx.conversation;
                    system_prompt = hook_ctx.system_prompt;
                    tools = hook_ctx.tools;
                    toolshim_tools = hook_ctx.toolshim_tools;

                    match action {
                        ExitAction::Exit | ExitAction::Defer => {
                            exit_chat = true;
                        }
                        ExitAction::Continue => {
                            // A hook wants to keep going (retry, final output nudge)
                        }
                    }
                }

                if is_token_cancelled(&cancel_token) {
                    tool_pair_summarization_task.abort();
                }

                if let Ok(summaries) = tool_pair_summarization_task.await {
                    let mut updated_messages = conversation.messages().clone();

                    for (summary_msg, tool_id) in summaries {
                        let matching: Vec<&mut Message> = updated_messages
                            .iter_mut()
                            .filter(|msg| {
                                msg.id.is_some() && msg.content.iter().any(|c| match c {
                                    MessageContent::ToolRequest(req) => req.id == tool_id,
                                    MessageContent::ToolResponse(resp) => resp.id == tool_id,
                                    _ => false,
                                })
                            })
                            .collect();

                        if matching.len() == 2 {
                            for msg in matching {
                                let id = msg.id.as_ref().unwrap();
                                msg.metadata = msg.metadata.with_agent_invisible();
                                SessionManager::update_message_metadata(&session_config.id, id, |metadata| {
                                    metadata.with_agent_invisible()
                                }).await?;
                            }
                            messages_to_add.push(summary_msg);
                        } else {
                            warn!("Expected a tool request/reply pair, but found {} matching messages",
                                matching.len());
                        }
                    }
                    conversation = Conversation::new_unvalidated(updated_messages);
                }

                for msg in &messages_to_add {
                    session_manager.add_message(&session_config.id, msg).await?;
                }
                conversation.extend(messages_to_add);
                if exit_chat {
                    break;
                }

                tokio::task::yield_now().await;
            }

            if !last_assistant_text.is_empty() {
                tracing::info!(target: "goose::agents::agent", trace_output = last_assistant_text.as_str());
            }
        }
        .instrument(reply_stream_span),
    );
    inner
}

// ---------------------------------------------------------------------------
// Helper functions — extracted from the tool execution block to reduce nesting
// ---------------------------------------------------------------------------

use crate::conversation::message::ToolRequest;
use crate::mcp_utils::ToolResult;

/// Build the request→response and request→metadata maps for a batch of tool requests.
fn prepare_tool_maps(
    requests: &[ToolRequest],
) -> (
    HashMap<String, Message>,
    HashMap<String, Option<ProviderMetadata>>,
) {
    let mut response_map = HashMap::new();
    let mut metadata_map = HashMap::new();
    for request in requests {
        response_map.insert(request.id.clone(), Message::user().with_generated_id());
        metadata_map.insert(request.id.clone(), request.metadata.clone());
    }
    (response_map, metadata_map)
}

/// Partition tool requests into (frontend, backend) based on `driver.is_frontend_tool`.
async fn partition_requests(
    driver: &(dyn LoopDriver + '_),
    requests: &[ToolRequest],
) -> (Vec<ToolRequest>, Vec<ToolRequest>) {
    let mut frontend = Vec::new();
    let mut backend = Vec::new();
    for request in requests {
        if let Ok(tool_call) = &request.tool_call {
            if driver.is_frontend_tool(&tool_call.name).await {
                frontend.push(request.clone());
                continue;
            }
        }
        backend.push(request.clone());
    }
    (frontend, backend)
}

/// Dispatch frontend tool calls, returning their streams.
async fn dispatch_frontend_tools(
    driver: &(dyn LoopDriver + '_),
    requests: &[ToolRequest],
    cancel_token: Option<CancellationToken>,
    session: &Session,
) -> Vec<(String, ToolStream)> {
    let mut futures = Vec::new();
    for request in requests {
        if let Ok(tool_call) = request.tool_call.clone() {
            let frontend_msg = Message::assistant().with_frontend_tool_request(
                request.id.clone(),
                Ok(tool_call.clone()),
            );
            let (req_id, tool_result) = driver
                .dispatch_tool_call(tool_call, request.id.clone(), cancel_token.clone(), session)
                .await;
            futures.push((
                req_id,
                match tool_result {
                    Ok(result) => frontend_tool_stream(frontend_msg, result.result),
                    Err(e) => {
                        tool_stream(Box::new(stream::empty()), futures::future::ready(Err(e)))
                    }
                },
            ));
        }
    }
    futures
}

/// Find request IDs for extension-install tool calls.
fn find_extension_install_ids(requests: &[ToolRequest]) -> Vec<String> {
    requests
        .iter()
        .filter_map(|r| {
            r.tool_call
                .as_ref()
                .ok()
                .filter(|tc| tc.name == MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE)
                .map(|_| r.id.clone())
        })
        .collect()
}

/// In chat mode, fill all backend tool responses with a "skipped" message.
fn skip_backend_tools_chat_mode(
    requests: &[ToolRequest],
    response_map: &mut HashMap<String, Message>,
) {
    for request in requests {
        if let Some(response) = response_map.get_mut(&request.id) {
            response.add_tool_response_with_metadata(
                request.id.clone(),
                Ok(CallToolResult::success(vec![Content::text(
                    CHAT_MODE_TOOL_SKIPPED_RESPONSE,
                )])),
                request.metadata.as_ref(),
            );
        }
    }
}

/// Extract a platform notification from a tool call result, if present.
fn extract_platform_notification(
    request_id: &str,
    call_result: &CallToolResult,
) -> Option<AgentEvent> {
    let meta = call_result.meta.as_ref()?;
    let notification_data = meta.0.get("platform_notification")?;
    let method = notification_data.get("method")?.as_str()?;
    let params = notification_data.get("params").cloned();
    let notification = rmcp::model::CustomNotification::new(method.to_string(), params);
    Some(AgentEvent::McpNotification((
        request_id.to_string(),
        rmcp::model::ServerNotification::CustomNotification(notification),
    )))
}

/// Handle a single item from the tool stream drain loop.
///
/// Updates `response_map` and `all_install_successful` as side effects.
/// Returns events to yield to the UI (0, 1, or 2 — e.g. a platform notification + result).
fn handle_tool_stream_item(
    request_id: String,
    item: ToolStreamItem<ToolResult<CallToolResult>>,
    response_map: &mut HashMap<String, Message>,
    metadata_map: &HashMap<String, Option<ProviderMetadata>>,
    extension_install_ids: &[String],
    all_install_successful: &mut bool,
) -> Vec<AgentEvent> {
    match item {
        ToolStreamItem::Result(output) => {
            let mut events = Vec::new();
            // Extract platform notification before consuming the output
            if let Ok(ref call_result) = output {
                if let Some(event) = extract_platform_notification(&request_id, call_result) {
                    events.push(event);
                }
            }
            // Track extension install failure
            if extension_install_ids.contains(&request_id) && output.is_err() {
                *all_install_successful = false;
            }
            // Record result in response map
            if let Some(response) = response_map.get_mut(&request_id) {
                let metadata = metadata_map.get(&request_id).and_then(|m| m.as_ref());
                response.add_tool_response_with_metadata(request_id, output, metadata);
            }
            events
        }
        ToolStreamItem::Notification(msg) => {
            vec![AgentEvent::McpNotification((request_id, msg))]
        }
        ToolStreamItem::AgentMessage(msg) => vec![AgentEvent::Message(msg)],
    }
}

/// Result of assembling tool-pair messages from a completed tool execution round.
struct ToolPairMessages {
    /// Messages to add to the conversation (thinking + request/response pairs).
    messages: Vec<Message>,
    /// Events to yield to the UI (tool response messages + error messages).
    events: Vec<AgentEvent>,
    /// Whether the loop should exit due to an unparseable tool call.
    exit_chat: bool,
}

/// Build conversation messages from tool execution results.
///
/// Preserves thinking/reasoning content, pairs each tool request with its response,
/// and handles unparseable tool calls.
fn build_tool_pair_messages(
    response: &Message,
    all_requests: &[ToolRequest],
    response_map: &mut HashMap<String, Message>,
) -> ToolPairMessages {
    let mut messages = Vec::new();
    let mut events = Vec::new();
    let mut exit_chat = false;

    // Preserve thinking content as a separate message
    let thinking_content: Vec<MessageContent> = response
        .content
        .iter()
        .filter(|c| matches!(c, MessageContent::Thinking(_)))
        .cloned()
        .collect();
    if !thinking_content.is_empty() {
        let thinking_msg = Message::new(response.role.clone(), response.created, thinking_content)
            .with_id(format!("msg_{}", Uuid::new_v4()));
        messages.push(thinking_msg);
    }

    // Reasoning content attached to each tool request message
    let reasoning_content: Vec<MessageContent> = response
        .content
        .iter()
        .filter(|c| matches!(c, MessageContent::Thinking(_)))
        .cloned()
        .collect();

    for request in all_requests {
        if request.tool_call.is_ok() {
            let mut request_msg = Message::assistant().with_id(format!("msg_{}", Uuid::new_v4()));
            for rc in &reasoning_content {
                request_msg = request_msg.with_content(rc.clone());
            }
            request_msg = request_msg.with_tool_request_with_metadata(
                request.id.clone(),
                request.tool_call.clone(),
                request.metadata.as_ref(),
                request.tool_meta.clone(),
            );
            messages.push(request_msg);

            let final_response = response_map
                .remove(&request.id)
                .unwrap_or_else(|| Message::user().with_generated_id());
            events.push(AgentEvent::Message(final_response.clone()));
            messages.push(final_response);
        } else {
            error!(
                "Tool call could not be parsed: {}",
                request.tool_call.as_ref().unwrap_err(),
            );
            events.push(AgentEvent::Message(
                Message::assistant().with_text(
                    "A tool call could not be parsed — the response may have been truncated. Try breaking the task into smaller steps or resending your message."
                ),
            ));
            exit_chat = true;
            break;
        }
    }

    ToolPairMessages {
        messages,
        events,
        exit_chat,
    }
}
