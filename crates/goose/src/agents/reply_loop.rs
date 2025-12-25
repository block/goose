//! Helper functions and types for the agent reply loop.
//!
//! This module extracts tool execution and message processing logic from
//! the main reply_internal function to improve readability and maintainability.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures::{stream, StreamExt, TryStreamExt};
use rmcp::model::{CallToolResult, Content, Tool};
use tokio::sync::Mutex;
use tracing::warn;
use uuid::Uuid;

use super::agent::{Agent, AgentEvent, ToolCategorizeResult, ToolStream, ToolStreamItem};
use super::extension_manager_extension::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;
use super::tool_execution::CHAT_MODE_TOOL_SKIPPED_RESPONSE;
use crate::agents::types::SessionConfig;
use crate::config::GooseMode;
use crate::conversation::message::{Message, MessageContent, ProviderMetadata, ToolRequest};
use crate::conversation::Conversation;
use crate::permission::permission_judge::PermissionCheckResult;
use crate::session::Session;
use crate::utils::is_token_cancelled;
use tokio_util::sync::CancellationToken;

/// Result of context length exceeded handling
pub(crate) enum ContextLengthResult {
    Compacted {
        events: Vec<AgentEvent>,
        new_conversation: Conversation,
    },
    MaxAttemptsReached(AgentEvent),
    Failed,
}

/// Result of loop exit handling
pub(crate) struct LoopExitResult {
    pub should_exit: bool,
    pub events: Vec<AgentEvent>,
}

/// Result of processing a response with tools
pub(crate) struct ToolProcessingResult {
    pub filtered_response: Message,
    pub tool_requests: Vec<ToolRequest>,
    pub events: Vec<AgentEvent>,
    pub messages_to_add: Vec<Message>,
    pub response_messages: Vec<Message>,
    pub tools_updated: bool,
}

/// Context for tool processing operations
pub(crate) struct ToolProcessingContext<'a> {
    pub tools: &'a [Tool],
    pub goose_mode: GooseMode,
    pub conversation: &'a Conversation,
    pub session_config: &'a SessionConfig,
    pub session: &'a Session,
    pub cancel_token: &'a Option<CancellationToken>,
}

/// State for tracking tool execution within a single provider response iteration
struct ToolExecutionState {
    tool_response_messages: Vec<Arc<Mutex<Message>>>,
    request_to_response_map: HashMap<String, Arc<Mutex<Message>>>,
    request_metadata: HashMap<String, Option<ProviderMetadata>>,
    enable_extension_request_ids: Vec<String>,
}

impl ToolExecutionState {
    fn new(frontend_requests: &[ToolRequest], remaining_requests: &[ToolRequest]) -> Self {
        let num_tool_requests = frontend_requests.len() + remaining_requests.len();
        let tool_response_messages: Vec<Arc<Mutex<Message>>> = (0..num_tool_requests)
            .map(|_| {
                Arc::new(Mutex::new(
                    Message::user().with_id(format!("msg_{}", Uuid::new_v4())),
                ))
            })
            .collect();

        let mut request_to_response_map = HashMap::new();
        let mut request_metadata: HashMap<String, Option<ProviderMetadata>> = HashMap::new();
        for (idx, request) in frontend_requests
            .iter()
            .chain(remaining_requests.iter())
            .enumerate()
        {
            request_to_response_map.insert(request.id.clone(), tool_response_messages[idx].clone());
            request_metadata.insert(request.id.clone(), request.metadata.clone());
        }

        let mut enable_extension_request_ids = vec![];
        for request in remaining_requests {
            if let Ok(tool_call) = &request.tool_call {
                if tool_call.name == MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE {
                    enable_extension_request_ids.push(request.id.clone());
                }
            }
        }

        Self {
            tool_response_messages,
            request_to_response_map,
            request_metadata,
            enable_extension_request_ids,
        }
    }
}

impl Agent {
    /// Process a response message that may contain tool requests
    pub(crate) async fn process_response_with_tools(
        &self,
        response: &Message,
        ctx: &ToolProcessingContext<'_>,
    ) -> Result<ToolProcessingResult> {
        let ToolCategorizeResult {
            frontend_requests,
            remaining_requests,
            filtered_response,
        } = self.categorize_tools(response, ctx.tools).await;

        let all_requests: Vec<_> = frontend_requests
            .iter()
            .chain(remaining_requests.iter())
            .cloned()
            .collect();

        if all_requests.is_empty() {
            return Ok(ToolProcessingResult {
                filtered_response,
                tool_requests: vec![],
                events: vec![],
                messages_to_add: vec![],
                response_messages: vec![],
                tools_updated: false,
            });
        }

        let state = ToolExecutionState::new(&frontend_requests, &remaining_requests);
        let mut events = Vec::new();

        // Handle frontend tool requests
        for (idx, request) in frontend_requests.iter().enumerate() {
            let mut frontend_tool_stream = self
                .handle_frontend_tool_request(request, state.tool_response_messages[idx].clone());

            while let Some(msg) = frontend_tool_stream.try_next().await? {
                events.push(AgentEvent::Message(msg));
            }
        }

        let tools_updated = if ctx.goose_mode == GooseMode::Chat {
            Self::skip_tools_for_chat_mode(&remaining_requests, &state.request_to_response_map)
                .await;
            false
        } else {
            let (updated, tool_events) = self
                .process_tool_requests(&remaining_requests, &state, ctx)
                .await?;
            events.extend(tool_events);
            updated
        };

        // Build messages to add and response messages to yield
        let (messages_to_add, response_messages) =
            Self::build_tool_messages(&frontend_requests, &remaining_requests, &state).await;

        Ok(ToolProcessingResult {
            filtered_response,
            tool_requests: all_requests,
            events,
            messages_to_add,
            response_messages,
            tools_updated,
        })
    }

    /// Skip all tool calls in chat mode by setting appropriate responses
    async fn skip_tools_for_chat_mode(
        remaining_requests: &[ToolRequest],
        request_to_response_map: &HashMap<String, Arc<Mutex<Message>>>,
    ) {
        for request in remaining_requests.iter() {
            if let Some(response_msg) = request_to_response_map.get(&request.id) {
                let mut response = response_msg.lock().await;
                *response = response.clone().with_tool_response_with_metadata(
                    request.id.clone(),
                    Ok(CallToolResult {
                        content: vec![Content::text(CHAT_MODE_TOOL_SKIPPED_RESPONSE)],
                        structured_content: None,
                        is_error: Some(false),
                        meta: None,
                    }),
                    request.metadata.as_ref(),
                );
            }
        }
    }

    /// Process tool requests in non-chat mode: inspect, approve, and execute tools
    async fn process_tool_requests(
        &self,
        remaining_requests: &[ToolRequest],
        state: &ToolExecutionState,
        ctx: &ToolProcessingContext<'_>,
    ) -> Result<(bool, Vec<AgentEvent>)> {
        let mut events = Vec::new();

        // Run all tool inspectors
        let inspection_results = self
            .tool_inspection_manager
            .inspect_tools(remaining_requests, ctx.conversation.messages())
            .await?;

        let permission_check_result = self
            .tool_inspection_manager
            .process_inspection_results_with_permission_inspector(
                remaining_requests,
                &inspection_results,
            )
            .unwrap_or_else(|| {
                let mut result = PermissionCheckResult {
                    approved: vec![],
                    needs_approval: vec![],
                    denied: vec![],
                };
                result
                    .needs_approval
                    .extend(remaining_requests.iter().cloned());
                result
            });

        let mut tool_futures = self
            .handle_approved_and_denied_tools(
                &permission_check_result,
                &state.request_to_response_map,
                ctx.cancel_token.clone(),
                ctx.session,
            )
            .await?;

        let tool_futures_arc = Arc::new(Mutex::new(tool_futures));

        let mut tool_approval_stream = self.handle_approval_tool_requests(
            &permission_check_result.needs_approval,
            tool_futures_arc.clone(),
            &state.request_to_response_map,
            ctx.cancel_token.clone(),
            ctx.session,
            &inspection_results,
        );

        while let Some(msg) = tool_approval_stream.try_next().await? {
            events.push(AgentEvent::Message(msg));
        }

        tool_futures = {
            let mut futures_lock = tool_futures_arc.lock().await;
            futures_lock.drain(..).collect::<Vec<_>>()
        };

        let (all_install_successful, tool_events) = self
            .execute_tools_and_collect_results(
                tool_futures,
                state,
                ctx.session_config,
                ctx.cancel_token,
            )
            .await?;

        events.extend(tool_events);

        if all_install_successful && !state.enable_extension_request_ids.is_empty() {
            if let Err(e) = self.save_extension_state(ctx.session_config).await {
                warn!(
                    "Failed to save extension state after runtime changes: {}",
                    e
                );
            }
        }

        let tools_updated =
            all_install_successful && !state.enable_extension_request_ids.is_empty();
        Ok((tools_updated, events))
    }

    /// Execute tools and collect results, returning events
    async fn execute_tools_and_collect_results(
        &self,
        tool_futures: Vec<(String, ToolStream)>,
        state: &ToolExecutionState,
        session_config: &SessionConfig,
        cancel_token: &Option<CancellationToken>,
    ) -> Result<(bool, Vec<AgentEvent>)> {
        let mut events = Vec::new();
        let mut all_install_successful = true;

        let with_id = tool_futures
            .into_iter()
            .map(|(request_id, stream)| stream.map(move |item| (request_id.clone(), item)))
            .collect::<Vec<_>>();

        let mut combined = stream::select_all(with_id);

        while let Some((request_id, item)) = combined.next().await {
            if is_token_cancelled(cancel_token) {
                break;
            }

            for msg in Self::drain_elicitation_messages(&session_config.id).await {
                events.push(AgentEvent::Message(msg));
            }

            match item {
                ToolStreamItem::Result(output) => {
                    if state.enable_extension_request_ids.contains(&request_id) && output.is_err() {
                        all_install_successful = false;
                    }
                    if let Some(response_msg) = state.request_to_response_map.get(&request_id) {
                        let metadata = state
                            .request_metadata
                            .get(&request_id)
                            .and_then(|m| m.as_ref());
                        let mut response = response_msg.lock().await;
                        *response = response
                            .clone()
                            .with_tool_response_with_metadata(request_id, output, metadata);
                    }
                }
                ToolStreamItem::Message(msg) => {
                    events.push(AgentEvent::McpNotification((request_id, msg)));
                }
            }
        }

        // Check for remaining elicitation messages after all tools complete
        for msg in Self::drain_elicitation_messages(&session_config.id).await {
            events.push(AgentEvent::Message(msg));
        }

        Ok((all_install_successful, events))
    }

    /// Extract and preserve thinking content from a response message
    pub(crate) fn extract_thinking_content(response: &Message) -> Option<Message> {
        let thinking_content: Vec<MessageContent> = response
            .content
            .iter()
            .filter(|c| matches!(c, MessageContent::Thinking(_)))
            .cloned()
            .collect();

        if thinking_content.is_empty() {
            None
        } else {
            Some(
                Message::new(response.role.clone(), response.created, thinking_content)
                    .with_id(format!("msg_{}", Uuid::new_v4())),
            )
        }
    }

    /// Build messages to add to conversation after tool execution
    async fn build_tool_messages(
        frontend_requests: &[ToolRequest],
        remaining_requests: &[ToolRequest],
        state: &ToolExecutionState,
    ) -> (Vec<Message>, Vec<Message>) {
        let mut messages_to_add = Vec::new();
        let mut response_messages = Vec::new();

        for (idx, request) in frontend_requests
            .iter()
            .chain(remaining_requests.iter())
            .enumerate()
        {
            if request.tool_call.is_ok() {
                let request_msg = Message::assistant()
                    .with_id(format!("msg_{}", Uuid::new_v4()))
                    .with_tool_request_with_metadata(
                        request.id.clone(),
                        request.tool_call.clone(),
                        request.metadata.as_ref(),
                    );
                messages_to_add.push(request_msg);

                let final_response = state.tool_response_messages[idx].lock().await.clone();
                response_messages.push(final_response.clone());
                messages_to_add.push(final_response);
            }
        }

        (messages_to_add, response_messages)
    }
}
