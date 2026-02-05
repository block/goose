//! Agent Loop - The core completion cycle
//!
//! This module contains the pure agent loop logic, separated from
//! session management and persistence concerns. It takes mutable
//! references to state and runs until completion.

use agent_client_protocol_schema::SessionId;
use rig::completion::CompletionRequest;
use rig::message::{AssistantContent, Message, ToolCall, UserContent};
use rig::OneOrMany;
use serde_json::Value;
use tracing::info;

use crate::extension::{
    builtin_tools, generate_preamble, is_builtin_tool, EnabledExtensions, ToolIndex,
    DISABLE_EXTENSION, ENABLE_EXTENSION,
};
use crate::notifier::Notifier;
use crate::provider::{Model, StreamChunk};
use crate::{Error, Result};

/// Result of a single iteration of the agent loop
pub enum StepResult {
    /// Completed a tool round - caller should checkpoint/persist
    ToolsExecuted,
    /// Turn ended with no more work to do
    Done,
}

/// Run a single step of the agent loop (streaming version)
///
/// This executes one completion request and processes the response.
/// Text is streamed to the notifier as it arrives.
/// If tools are called, it executes them and returns `ToolsExecuted`.
/// If no tools are called, returns `Done`.
///
/// The caller is responsible for:
/// - Persisting messages after `ToolsExecuted` (crash recovery)
/// - Persisting messages after `Done` (end of turn)
/// - Calling this in a loop until `Done`
pub async fn run_step<N: Notifier>(
    session_id: &SessionId,
    messages: &mut Vec<Message>,
    base_preamble: Option<&str>,
    extensions: &mut EnabledExtensions,
    model: &Model,
    notifier: &N,
) -> Result<StepResult> {
    // 1. Gather tools from extensions + builtin tools
    let (mut tools, tool_index) = extensions.gather_tools().await?;
    tools.extend(builtin_tools());

    // 2. Generate dynamic preamble with extension info
    let preamble = generate_preamble(extensions, base_preamble);

    // 3. Build completion request
    let history = prepare_history(messages);
    let completion_request = CompletionRequest {
        preamble: Some(preamble),
        chat_history: history,
        documents: vec![],
        tools,
        temperature: None,
        max_tokens: None,
        tool_choice: None,
        additional_params: None,
    };

    // 4. Stream completion request
    info!("Starting streaming completion request");

    let (mut rx, handle) = model.stream_with_channel(completion_request);

    // Process chunks as they arrive - true streaming!
    while let Some(chunk) = rx.recv().await {
        match chunk {
            StreamChunk::Text(text) => {
                // Stream text to client immediately
                notifier.send_text_chunk(session_id, &text).await?;
            }
            StreamChunk::ToolCall(tc) => {
                // Notify about tool call
                notifier.send_tool_use(session_id, &tc).await?;
            }
            StreamChunk::Reasoning(_) => {
                // TODO: Handle reasoning chunks if needed
            }
            StreamChunk::Done { .. } => {
                break;
            }
        }
    }

    // Wait for stream to complete and get final result
    let result = handle.await_result().await?;

    // 5. Add assistant message to history
    if !result.text.is_empty() || !result.tool_calls.is_empty() {
        let mut content = Vec::new();
        if !result.text.is_empty() {
            content.push(AssistantContent::text(&result.text));
        }
        for tc in &result.tool_calls {
            content.push(AssistantContent::ToolCall(tc.clone()));
        }
        if let Ok(msg) = OneOrMany::many(content) {
            messages.push(Message::Assistant {
                content: msg,
                id: None,
            });
        }
    }

    // 6. If tool calls, execute them
    if !result.tool_calls.is_empty() {
        for tc in &result.tool_calls {
            info!(
                tool_name = %tc.function.name,
                tool_id = %tc.id,
                tool_call_id = ?tc.call_id,
                "Executing tool call"
            );
            
            // Execute tool - errors become error results, not fatal failures
            let tool_result = match execute_tool(extensions, &tool_index, tc).await {
                Ok(result) => result,
                Err(e) => {
                    // Return error as tool result so the model can see what went wrong
                    format!("Error: {}", e)
                }
            };

            notifier
                .send_tool_result(session_id, tc, &tool_result)
                .await?;

            // Add tool result to history
            // Note: We need to use tool_result_with_call_id for OpenAI compatibility
            // OpenAI uses call_id to match tool results to tool calls
            let call_id = tc.call_id.clone().unwrap_or_else(|| tc.id.clone());
            messages.push(Message::User {
                content: OneOrMany::one(UserContent::tool_result_with_call_id(
                    tc.id.clone(),
                    call_id,
                    OneOrMany::one(rig::message::ToolResultContent::text(&tool_result)),
                )),
            });
        }

        return Ok(StepResult::ToolsExecuted);
    }

    // 7. No tool calls = done
    info!("Agent loop step completed (no tools)");
    Ok(StepResult::Done)
}

/// Execute a tool call, routing to the appropriate handler
async fn execute_tool(
    extensions: &mut EnabledExtensions,
    tool_index: &ToolIndex,
    tool_call: &ToolCall,
) -> Result<String> {
    let tool_name = &tool_call.function.name;
    let arguments = tool_call.function.arguments.as_object().cloned();

    // Check if it's a builtin tool
    if is_builtin_tool(tool_name) {
        return execute_builtin_tool(extensions, tool_name, arguments).await;
    }

    // Route to extension
    extensions.call_tool(tool_index, tool_name, arguments).await
}

/// Execute a built-in platform tool
async fn execute_builtin_tool(
    extensions: &mut EnabledExtensions,
    tool_name: &str,
    arguments: Option<serde_json::Map<String, Value>>,
) -> Result<String> {
    let name = arguments
        .as_ref()
        .and_then(|a| a.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Extension("Missing 'name' argument".to_string()))?;

    match tool_name {
        ENABLE_EXTENSION => {
            extensions.enable(name).await?;
            Ok(format!("Extension '{}' enabled successfully.", name))
        }
        DISABLE_EXTENSION => {
            extensions.disable(name)?;
            Ok(format!("Extension '{}' disabled.", name))
        }
        _ => Err(Error::Extension(format!(
            "Unknown builtin tool: {}",
            tool_name
        ))),
    }
}

/// Prepare message history for completion request
fn prepare_history(messages: &[Message]) -> OneOrMany<Message> {
    if messages.is_empty() {
        return OneOrMany::one(Message::user(""));
    }

    OneOrMany::many(messages.to_vec()).unwrap_or_else(|_| OneOrMany::one(Message::user("")))
}
