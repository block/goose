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
use tracing::{error, info, warn};
use tracing_futures::Instrument;
use uuid::Uuid;

use super::agent::{frontend_tool_stream, tool_stream, AgentEvent, ToolStream, ToolStreamItem};
use super::hooks::{AgentHook, ErrorRecovery, LoopContext, LoopEvent};
use super::reply_parts::update_session_metrics_standalone;
use super::tool_execution::{ToolCallResult, CHAT_MODE_TOOL_SKIPPED_RESPONSE};
use crate::agents::final_output_tool::FINAL_OUTPUT_CONTINUATION_MESSAGE;
use crate::agents::platform_extensions::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;
use crate::agents::types::SessionConfig;
use crate::config::{Config, GooseMode};
use crate::conversation::message::{
    Message, MessageContent, ProviderMetadata, SystemNotificationType,
};
use crate::conversation::Conversation;
use crate::permission::permission_judge::PermissionCheckResult;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::session::{Session, SessionManager};
use crate::tool_inspection::InspectionResult;
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

    /// Run tool inspectors on a batch of requests.
    async fn inspect_tools(
        &self,
        session_id: &str,
        requests: &[crate::conversation::message::ToolRequest],
        messages: &[Message],
        goose_mode: GooseMode,
    ) -> Result<Vec<InspectionResult>>;

    /// Process inspection results through the permission inspector.
    fn process_permissions(
        &self,
        requests: &[crate::conversation::message::ToolRequest],
        inspection_results: &[InspectionResult],
    ) -> Option<PermissionCheckResult>;

    /// Handle approved and denied tools (dispatch approved, mark denied).
    async fn handle_approved_and_denied_tools(
        &self,
        permission_check_result: &PermissionCheckResult,
        request_to_response_map: &mut HashMap<String, Message>,
        cancel_token: Option<CancellationToken>,
        session: &Session,
    ) -> Result<Vec<(String, ToolStream)>>;

    /// Handle tools that need user approval (interactive).
    /// Returns a stream of messages (action-required prompts) and populates tool_futures.
    fn handle_approval_tool_requests<'a>(
        &'a self,
        tool_requests: &'a [crate::conversation::message::ToolRequest],
        tool_futures: &'a mut Vec<(String, ToolStream)>,
        request_to_response_map: &'a mut HashMap<String, Message>,
        cancel_token: Option<CancellationToken>,
        session: &'a Session,
        inspection_results: &'a [InspectionResult],
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

    /// Check if the final output tool has produced output.
    async fn check_final_output(&self) -> Option<Option<String>>;

    /// Handle retry logic when no tools were called.
    /// Returns true if the loop should retry.
    async fn handle_retry_logic(
        &self,
        conversation: &mut Conversation,
        session_config: &SessionConfig,
        initial_messages: &[Message],
    ) -> Result<bool>;

    /// Reset retry attempt counter at the start of a reply.
    async fn reset_retry_attempts(&self);

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
    /// Initial messages (for retry logic).
    pub initial_messages: Vec<Message>,
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
        initial_messages,
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

                // Check for final output
                if let Some(ref output) = driver.check_final_output().await {
                    if let Some(text) = output {
                        yield AgentEvent::Message(Message::assistant().with_text(text));
                    }
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
                {
                    let (hook_event_tx, mut hook_event_rx) = tokio::sync::mpsc::unbounded_channel();
                    let mut hook_ctx = LoopContext {
                        conversation,
                        system_prompt,
                        tools,
                        toolshim_tools,
                        provider: driver.provider(),
                        session_id: session_config.id.clone(),
                        schedule_id: session_config.schedule_id.clone(),
                        session_manager: session_manager.clone(),
                        event_tx: hook_event_tx,
                    };

                    for hook in &hooks {
                        hook.pre_inference(&mut hook_ctx).await?;
                    }

                    // Move state back (hooks may have modified conversation, prompt, tools)
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
                                let (hook_event_tx, mut hook_event_rx) = tokio::sync::mpsc::unbounded_channel();
                                let mut hook_ctx = LoopContext {
                                    conversation: conversation.clone(),
                                    system_prompt: system_prompt.clone(),
                                    tools: tools.clone(),
                                    toolshim_tools: toolshim_tools.clone(),
                                    provider: driver.provider(),
                                    session_id: session_config.id.clone(),
                                    schedule_id: session_config.schedule_id.clone(),
                                    session_manager: session_manager.clone(),
                                    event_tx: hook_event_tx,
                                };
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

                                let mut request_to_response_map = HashMap::new();
                                let mut request_metadata: HashMap<String, Option<ProviderMetadata>> = HashMap::new();
                                for request in all_requests.iter() {
                                    request_to_response_map.insert(request.id.clone(), Message::user().with_generated_id());
                                    request_metadata.insert(request.id.clone(), request.metadata.clone());
                                }

                                // Partition into frontend and backend requests
                                let mut frontend_requests = Vec::new();
                                let mut backend_requests = Vec::new();
                                for request in &all_requests {
                                    if let Ok(tool_call) = &request.tool_call {
                                        if driver.is_frontend_tool(&tool_call.name).await {
                                            frontend_requests.push(request.clone());
                                            continue;
                                        }
                                    }
                                    backend_requests.push(request.clone());
                                }

                                // Dispatch frontend tools through unified dispatch path
                                let mut tool_futures: Vec<(String, ToolStream)> = Vec::new();
                                for request in &frontend_requests {
                                    if let Ok(tool_call) = request.tool_call.clone() {
                                        let frontend_msg = Message::assistant().with_frontend_tool_request(
                                            request.id.clone(),
                                            Ok(tool_call.clone()),
                                        );
                                        let (req_id, tool_result) = driver.dispatch_tool_call(
                                            tool_call,
                                            request.id.clone(),
                                            cancel_token.clone(),
                                            &session,
                                        ).await;
                                        tool_futures.push((req_id, match tool_result {
                                            Ok(result) => frontend_tool_stream(
                                                frontend_msg,
                                                result.result,
                                            ),
                                            Err(e) => {
                                                tool_stream(Box::new(stream::empty()), futures::future::ready(Err(e)))
                                            }
                                        }));
                                    }
                                }

                                // Track extension install requests
                                let mut enable_extension_request_ids: Vec<String> = vec![];

                                if goose_mode == GooseMode::Chat {
                                    for request in backend_requests.iter() {
                                        if let Some(response) = request_to_response_map.get_mut(&request.id) {
                                            response.add_tool_response_with_metadata(
                                                request.id.clone(),
                                                Ok(CallToolResult::success(vec![Content::text(CHAT_MODE_TOOL_SKIPPED_RESPONSE)])),
                                                request.metadata.as_ref(),
                                            );
                                        }
                                    }
                                } else {
                                    let inspection_results = driver.inspect_tools(
                                        &session_config.id,
                                        &backend_requests,
                                        conversation.messages(),
                                        goose_mode,
                                    ).await?;

                                    let permission_check_result = driver.process_permissions(
                                        &backend_requests,
                                        &inspection_results,
                                    ).unwrap_or_else(|| {
                                        let mut result = PermissionCheckResult {
                                            approved: vec![],
                                            needs_approval: vec![],
                                            denied: vec![],
                                        };
                                        result.needs_approval.extend(backend_requests.iter().cloned());
                                        result
                                    });

                                    for request in &backend_requests {
                                        if let Ok(tool_call) = &request.tool_call {
                                            if tool_call.name == MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE {
                                                enable_extension_request_ids.push(request.id.clone());
                                            }
                                        }
                                    }

                                    let mut backend_futures = driver.handle_approved_and_denied_tools(
                                        &permission_check_result,
                                        &mut request_to_response_map,
                                        cancel_token.clone(),
                                        &session,
                                    ).await?;
                                    tool_futures.append(&mut backend_futures);

                                    {
                                        let mut tool_approval_stream = driver.handle_approval_tool_requests(
                                            &permission_check_result.needs_approval,
                                            &mut tool_futures,
                                            &mut request_to_response_map,
                                            cancel_token.clone(),
                                            &session,
                                            &inspection_results,
                                        );

                                        while let Some(msg) = tool_approval_stream.try_next().await? {
                                            yield AgentEvent::Message(msg);
                                        }
                                    }
                                }

                                // Drain all tool streams through one unified loop
                                {
                                    let with_id = tool_futures
                                        .into_iter()
                                        .map(|(request_id, stream)| {
                                            stream.map(move |item| (request_id.clone(), item))
                                        })
                                        .collect::<Vec<_>>();

                                    let mut combined = stream::select_all(with_id);
                                    let mut all_install_successful = true;

                                    loop {
                                        if is_token_cancelled(&cancel_token) {
                                            break;
                                        }

                                        for msg in driver.drain_elicitation_messages(&session_config.id).await {
                                            yield AgentEvent::Message(msg);
                                        }

                                        tokio::select! {
                                            biased;

                                            tool_item = combined.next() => {
                                                match tool_item {
                                                    Some((request_id, item)) => {
                                                        match item {
                                                            ToolStreamItem::Result(output) => {
                                                                if let Ok(ref call_result) = output {
                                                                    if let Some(ref meta) = call_result.meta {
                                                                        if let Some(notification_data) = meta.0.get("platform_notification") {
                                                                            if let Some(method) = notification_data.get("method").and_then(|v| v.as_str()) {
                                                                                let params = notification_data.get("params").cloned();
                                                                                let custom_notification = rmcp::model::CustomNotification::new(
                                                                                    method.to_string(),
                                                                                    params,
                                                                                );

                                                                                let server_notification = rmcp::model::ServerNotification::CustomNotification(custom_notification);
                                                                                yield AgentEvent::McpNotification((request_id.clone(), server_notification));
                                                                            }
                                                                        }
                                                                    }
                                                                }

                                                                if enable_extension_request_ids.contains(&request_id)
                                                                    && output.is_err()
                                                                {
                                                                    all_install_successful = false;
                                                                }
                                                                if let Some(response) = request_to_response_map.get_mut(&request_id) {
                                                                    let metadata = request_metadata.get(&request_id).and_then(|m| m.as_ref());
                                                                    response.add_tool_response_with_metadata(request_id, output, metadata);
                                                                }
                                                            }
                                                            ToolStreamItem::Notification(msg) => {
                                                                yield AgentEvent::McpNotification((request_id, msg));
                                                            }
                                                            ToolStreamItem::AgentMessage(msg) => {
                                                                yield AgentEvent::Message(msg);
                                                            }
                                                        }
                                                    }
                                                    None => break,
                                                }
                                            }

                                            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                                                // Continue loop to drain elicitation messages
                                            }
                                        }
                                    }

                                    for msg in driver.drain_elicitation_messages(&session_config.id).await {
                                        yield AgentEvent::Message(msg);
                                    }

                                    if all_install_successful && !enable_extension_request_ids.is_empty() {
                                        if let Err(e) = driver.save_extension_state(&session_config).await {
                                            warn!("Failed to save extension state after runtime changes: {}", e);
                                        }
                                        tools_updated = true;
                                    }
                                }

                                // Preserve thinking/reasoning content
                                let thinking_content: Vec<MessageContent> = response.content.iter()
                                    .filter(|c| matches!(c, MessageContent::Thinking(_)))
                                    .cloned()
                                    .collect();
                                if !thinking_content.is_empty() {
                                    let thinking_msg = Message::new(
                                        response.role.clone(),
                                        response.created,
                                        thinking_content,
                                    ).with_id(format!("msg_{}", Uuid::new_v4()));
                                    messages_to_add.push(thinking_msg);
                                }

                                let reasoning_content: Vec<MessageContent> = response.content.iter()
                                    .filter(|c| matches!(c, MessageContent::Thinking(_)))
                                    .cloned()
                                    .collect();

                                for request in all_requests.iter() {
                                    if request.tool_call.is_ok() {
                                        let mut request_msg = Message::assistant()
                                            .with_id(format!("msg_{}", Uuid::new_v4()));

                                        for rc in &reasoning_content {
                                            request_msg = request_msg.with_content(rc.clone());
                                        }

                                        request_msg = request_msg
                                            .with_tool_request_with_metadata(
                                                request.id.clone(),
                                                request.tool_call.clone(),
                                                request.metadata.as_ref(),
                                                request.tool_meta.clone(),
                                            );
                                        messages_to_add.push(request_msg);
                                        let final_response = request_to_response_map
                                            .remove(&request.id)
                                            .unwrap_or_else(|| Message::user().with_generated_id());
                                        yield AgentEvent::Message(final_response.clone());
                                        messages_to_add.push(final_response);
                                    } else {
                                        error!(
                                            "Tool call could not be parsed: {}",
                                            request.tool_call.as_ref().unwrap_err(),
                                        );
                                        yield AgentEvent::Message(
                                            Message::assistant().with_text(
                                                "A tool call could not be parsed — the response may have been truncated. Try breaking the task into smaller steps or resending your message."
                                            )
                                        );
                                        exit_chat = true;
                                        break;
                                    }
                                }

                                no_tools_called = false;
                            }
                        }
                        Err(ref provider_err) => {
                            #[cfg(feature = "telemetry")]
                            crate::posthog::emit_error(provider_err.telemetry_type(), &provider_err.to_string());

                            // --- on_error hooks ---
                            // Give hooks a chance to handle the error (e.g. compaction on ContextLengthExceeded).
                            let (hook_event_tx, mut hook_event_rx) = tokio::sync::mpsc::unbounded_channel();
                            let mut hook_ctx = LoopContext {
                                conversation: conversation.clone(),
                                system_prompt: system_prompt.clone(),
                                tools: tools.clone(),
                                toolshim_tools: toolshim_tools.clone(),
                                provider: driver.provider(),
                                session_id: session_config.id.clone(),
                                schedule_id: session_config.schedule_id.clone(),
                                session_manager: session_manager.clone(),
                                event_tx: hook_event_tx,
                            };

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

                if no_tools_called {
                    let final_output = driver.check_final_output().await;

                    match final_output {
                        Some(None) => {
                            warn!("Final output tool has not been called yet. Continuing agent loop.");
                            let message = Message::user().with_text(FINAL_OUTPUT_CONTINUATION_MESSAGE);
                            messages_to_add.push(message.clone());
                            yield AgentEvent::Message(message);
                        }
                        Some(Some(output)) => {
                            let message = Message::assistant().with_text(output);
                            messages_to_add.push(message.clone());
                            yield AgentEvent::Message(message);
                            exit_chat = true;
                        }
                        None if did_recovery_compact_this_iteration => {
                            // continue from last user message after recovery compact
                        }
                        None => {
                            match driver.handle_retry_logic(&mut conversation, &session_config, &initial_messages).await {
                                Ok(should_retry) => {
                                    if should_retry {
                                        info!("Retry logic triggered, restarting agent loop");
                                        messages_to_add = Conversation::default();
                                        session_manager.replace_conversation(&session_config.id, &conversation).await?;
                                        yield AgentEvent::HistoryReplaced(conversation.clone());
                                    } else {
                                        exit_chat = true;
                                    }
                                }
                                Err(e) => {
                                    error!("Retry logic failed: {}", e);
                                    yield AgentEvent::Message(
                                        Message::assistant().with_text(
                                            format!("Retry logic encountered an error: {}", e)
                                        )
                                    );
                                    exit_chat = true;
                                }
                            }
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
