use anyhow::Result;
use async_trait::async_trait;

use crate::conversation::message::{Message, ToolRequest};
use crate::conversation::Conversation;
use crate::mcp_utils::ToolResult;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use rmcp::model::{CallToolResult, Tool};
use std::sync::Arc;

/// Mutable context threaded through hook calls during each iteration of the agent loop.
///
/// Hooks can read and modify the conversation, system prompt, and tool list.
/// Changes made by one hook are visible to subsequent hooks (hooks run in order).
pub struct LoopContext {
    /// Current conversation history.
    pub conversation: Conversation,
    /// System prompt sent to the provider.
    pub system_prompt: String,
    /// Tools available to the model.
    pub tools: Vec<Tool>,
    /// Toolshim tools (provider-specific; tools embedded in system prompt).
    pub toolshim_tools: Vec<Tool>,
    /// The LLM provider for this session.
    pub provider: Arc<dyn Provider>,
    /// Session identifier.
    pub session_id: String,
}

/// What to do after a hook's `on_error` returns.
#[derive(Debug)]
pub enum ErrorRecovery {
    /// The hook handled the error (e.g. compacted). Retry the current iteration.
    Retry,
    /// The hook did not handle it. Let the next hook try, or propagate.
    Propagate,
}

/// What to do when the loop is about to exit.
#[derive(Debug)]
pub enum ExitAction {
    /// Exit the loop normally.
    Exit,
    /// Continue the loop (e.g. retry logic restarted the conversation).
    Continue,
}

/// Decision from `pre_tool_call`.
#[derive(Debug)]
pub enum ToolCallDecision {
    /// Proceed with the tool call as-is.
    Proceed,
    /// Skip this tool call and inject this result instead.
    Skip(ToolResult<CallToolResult>),
    /// Ask the user for confirmation before proceeding.
    /// The `String` is the reason / context to show the user.
    NeedsApproval(String),
}

/// Why the loop is exiting.
#[derive(Debug)]
pub enum ExitReason {
    /// The model responded with text only (no tool calls).
    NoToolCalls,
    /// Maximum turns reached.
    MaxTurns,
    /// Cancellation token fired.
    Cancelled,
}

/// Extension point for the agent loop.
///
/// Each method has a default no-op implementation so hooks only need to override
/// the lifecycle points they care about. Hooks are called in registration order.
///
/// # Lifecycle
///
/// ```text
/// loop {
///   for hook in hooks: pre_inference(&mut ctx)
///   response = provider.complete(ctx.system_prompt, ctx.conversation, ctx.tools)
///   for hook in hooks: post_inference(response, &mut ctx)
///   for each tool_request in response:
///     for hook in hooks: pre_tool_call(request, &ctx) → decision
///     if proceed: result = execute(request)
///     for hook in hooks: post_tool_call(request, result, &mut ctx)
///   if error from provider:
///     for hook in hooks: on_error(error, &mut ctx) → recovery
///   if exiting:
///     for hook in hooks: on_loop_exit(reason, &mut ctx) → action
/// }
/// ```
#[async_trait]
pub trait AgentHook: Send + Sync {
    /// Called before each LLM inference call.
    ///
    /// Use this to modify the conversation (e.g. inject context), update the
    /// system prompt (e.g. reload hints), or filter/reorder tools.
    async fn pre_inference(&self, _ctx: &mut LoopContext) -> Result<()> {
        Ok(())
    }

    /// Called after the LLM responds, before tool execution begins.
    ///
    /// `response` is the assistant message just received. Hooks can inspect it
    /// but should not modify it — use the return value to signal actions.
    async fn post_inference(&self, _response: &Message, _ctx: &mut LoopContext) -> Result<()> {
        Ok(())
    }

    /// Called before dispatching a single tool call.
    ///
    /// Return `ToolCallDecision::Skip(result)` to prevent execution and inject
    /// a synthetic result, or `NeedsApproval` to gate on user confirmation.
    async fn pre_tool_call(
        &self,
        _request: &ToolRequest,
        _ctx: &LoopContext,
    ) -> Result<ToolCallDecision> {
        Ok(ToolCallDecision::Proceed)
    }

    /// Called after a tool call completes (or is skipped).
    async fn post_tool_call(
        &self,
        _request: &ToolRequest,
        _result: &ToolResult<CallToolResult>,
        _ctx: &mut LoopContext,
    ) -> Result<()> {
        Ok(())
    }

    /// Called when the provider returns an error during streaming.
    ///
    /// Return `ErrorRecovery::Retry` if the hook handled the error (e.g. by
    /// compacting the conversation in `ctx`). The loop will retry the current
    /// iteration. Return `Propagate` to let subsequent hooks try or to surface
    /// the error.
    async fn on_error(
        &self,
        _error: &ProviderError,
        _ctx: &mut LoopContext,
    ) -> Result<ErrorRecovery> {
        Ok(ErrorRecovery::Propagate)
    }

    /// Called when the loop is about to exit.
    ///
    /// Return `ExitAction::Continue` to keep the loop running (e.g. retry
    /// logic that resets the conversation).
    async fn on_loop_exit(
        &self,
        _reason: &ExitReason,
        _ctx: &mut LoopContext,
    ) -> Result<ExitAction> {
        Ok(ExitAction::Exit)
    }
}
